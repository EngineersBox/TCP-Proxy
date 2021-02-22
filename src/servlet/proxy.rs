use std::env;
use std::io;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::vec::Vec;
use threadpool::ThreadPool;

use crate::rules::ruleset;
use crate::configuration::config::Config;

#[macro_export]
macro_rules! try_or_continue {
    ($r:expr) => {
        if let Ok(val) = $r {
            val
        } else {
            continue;
        }
    };
}

pub struct ListenerBinding {
    pub listener: TcpListener,
    pub rule: ruleset::BindingRule,
}

pub struct Proxy {
    pub threadpool: ThreadPool,
    pub listeners: Vec<ListenerBinding>
}

static THREADPOOL_SIZE_KEY: &'static str = "threadpool_size";

impl Proxy {
    pub fn new(configuration: Config) -> Proxy {
        let threadpool_size_str = configuration.properties.get(THREADPOOL_SIZE_KEY);
        let threadpool_size = if threadpool_size_str.is_none() { 50 } else { threadpool_size_str.unwrap().parse::<usize>().unwrap() };
        Proxy {
            threadpool: ThreadPool::new(threadpool_size),
            listeners: Vec::new(),
        }
    }
    pub fn initialize_bindings(&mut self, rule_set: ruleset::RuleSet) {
        for rule in rule_set.rules {
            let (proxy_addr, to_addr) = (rule.from, rule.to);
            println!("{}", proxy_addr.to_string());
            let listener = TcpListener::bind(proxy_addr.to_string()).expect("Unable to bind proxy address");
            self.listeners.push(ListenerBinding{
                listener,
                rule,
            });
            println!("Allocated thread for TCP connection from {} to {}", proxy_addr, to_addr);
            self.invoke_acceptor_handler(self.listeners.len() - 1);
        }
    }
    fn invoke_acceptor_handler(&mut self, listener_idx: usize) {
        let listener_binding: Option<&ListenerBinding> = self.listeners.get(listener_idx);
        if listener_binding.is_none() {
            return
        }
        let listener_binding_unwrapped = listener_binding.unwrap();
        for incoming_stream in listener_binding_unwrapped.listener.incoming() {
            let proxy_stream = try_or_continue!(incoming_stream);
            let conn_thread = TcpStream::connect(listener_binding_unwrapped.rule.to.to_string())
                .map(|to_stream| self.threadpool.execute(move || handle_conn(proxy_stream, to_stream)));

            match conn_thread {
                Ok(_) => { println!("Successfully established connection"); }
                Err(err) => { println!("Unable to establish a connection: {}", err); }
            }
        }
    }
}

fn handle_conn(lhs_stream: TcpStream, rhs_stream: TcpStream) {
    let lhs_arc = Arc::new(lhs_stream);
    let rhs_arc = Arc::new(rhs_stream);

    let (mut lhs_tx, mut lhs_rx) = (lhs_arc.try_clone().unwrap(), lhs_arc.try_clone().unwrap());
    let (mut rhs_tx, mut rhs_rx) = (rhs_arc.try_clone().unwrap(), rhs_arc.try_clone().unwrap());

    let connections = vec![
        thread::spawn(move || io::copy(&mut lhs_tx, &mut rhs_rx).unwrap()),
        thread::spawn(move || io::copy(&mut rhs_tx, &mut lhs_rx).unwrap()),
    ];

    for t in connections {
        t.join().unwrap();
    }
}