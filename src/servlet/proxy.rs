use std::io::{BufRead, BufReader, Write};
use std::net::SocketAddr;
use std::net::{TcpListener, TcpStream};
use crate::configuration::config::Config;
use rayon::ThreadPool;
use crate::bindings::bindingset;
use std::thread;

pub struct ListenerBinding {
    pub listener: *const TcpListener,
    pub rule: bindingset::BindingRule,
}

pub struct Proxy {
    pub thread_pool: ThreadPool,
    pub listeners: Vec<ListenerBinding>
}

static THREADPOOL_SIZE_KEY: &'static str = "threadpool_size";

macro_rules! try_except_return {
    ($connection_statement:expr, $msg:literal) => {
        match $connection_statement {
            Ok(value) => value,
            Err(e) => {
                error!(crate::LOGGER, "{}: {}", $msg, e);
                return;
            },
        }
    }
}

impl Proxy {
    pub fn new(configuration: Config) -> Proxy {
        let threadpool_size_str:Option<&String> = configuration.properties.get(THREADPOOL_SIZE_KEY);
        let threadpool_size: usize = if threadpool_size_str.is_none() { 50 } else { threadpool_size_str.unwrap().parse::<usize>().unwrap() };
        Proxy {
            thread_pool: rayon::ThreadPoolBuilder::new().num_threads(threadpool_size)
                .spawn_handler(|thread| {
                    let mut b = std::thread::Builder::new();
                    if let Some(name) = thread.name() {
                        b = b.name(name.to_owned());
                    } else {
                        b = b.name(uuid::Uuid::new_v4().to_string());
                    }
                    b.spawn(|| thread.run())?;
                    Ok(())
                })
                .build().unwrap(),
            listeners: Vec::new(),
        }
    }
    pub fn initialize_bindings(&mut self, rule_set: bindingset::BindingSet) {
        for rule in rule_set.bindings {
            let (proxy_addr, to_addr) = (rule.from, rule.to);
            let mut listener: TcpListener = try_except_return!{TcpListener::bind(proxy_addr.to_string()), "Unable to bind proxy address"};
            self.listeners.push(ListenerBinding{
                listener: &listener,
                rule,
            });
            self.thread_pool.spawn(move || Proxy::invoke_acceptor_handler(&mut listener, to_addr));
        }
    }
    fn invoke_acceptor_handler(listener_forward: &mut TcpListener, proxy_to: SocketAddr) {
        info!(crate::LOGGER, "Invoking acceptor handler for thread: {}", thread::current().name().unwrap());
        loop {
            let (stream_forward, _addr) = listener_forward.accept().expect("Failed to accept connection");
            debug!(crate::LOGGER, "New connection");

            let sender_forward: TcpStream = try_except_return!{TcpStream::connect(proxy_to), "Failed to bind"};
            let sender_backward: TcpStream = try_except_return!{sender_forward.try_clone(), "Failed to clone stream"};
            let stream_backward: TcpStream = try_except_return!{stream_forward.try_clone(), "Failed to clone stream"};

            thread::spawn(move || Proxy::forward_thread_handler(stream_forward, sender_forward));
            thread::spawn(move || Proxy::backward_thread_handler(stream_backward, sender_backward));
        }
    }
    pub fn forward_thread_handler(stream_forward: TcpStream, mut sender_forward: TcpStream) {
        let mut stream_forward: BufReader<TcpStream> = BufReader::new(stream_forward);
        loop {
            let length: usize = {
                let buffer: &[u8] = stream_forward.fill_buf().unwrap();
                let length: usize = buffer.len();
                if buffer.is_empty() {
                    // Connection closed
                    debug!(crate::LOGGER, "Client closed connection");
                    return;
                }
                sender_forward.write_all(&buffer).expect("Failed to write to remote");
                info!(crate::LOGGER, "TRAFFIC LOG [INGRESS]: {}", String::from_utf8_lossy(&buffer).chars().as_str());
                sender_forward.flush().expect("Failed to flush remote");
                length
            };
            stream_forward.consume(length);
        }
    }
    pub fn backward_thread_handler(mut stream_backward: TcpStream, sender_backward: TcpStream) {
        let mut sender_backward: BufReader<TcpStream> = BufReader::new(sender_backward);
        loop {
            let length: usize = {
                let buffer: &[u8] = sender_backward.fill_buf().unwrap();
                let length: usize = buffer.len();
                if buffer.is_empty() {
                    // Connection closed
                    debug!(crate::LOGGER, "Remote closed connection");
                    return;
                }
                if stream_backward.write_all(&buffer).is_err() {
                    // Connection closed
                    debug!(crate::LOGGER, "Client closed connection");
                    return;
                }

                info!(crate::LOGGER, "TRAFFIC LOG [EGRESS]: {}", String::from_utf8_lossy(&buffer).chars().as_str());
                stream_backward.flush().expect("Failed to flush locally");
                length
            };
            sender_backward.consume(length);
        }
    }
    pub fn start(&mut self, binding_set: bindingset::BindingSet) {
        info!(crate::LOGGER, "Initializing bindings");
        self.initialize_bindings(binding_set);
        loop {}
    }
}