use std::net::SocketAddr;
use std::net::{TcpListener, TcpStream};
use rayon::ThreadPool;
use std::{thread, fmt};
use std::sync::{Arc, Mutex};
use std::str::FromStr;
use std::borrow::Borrow;

use crate::configuration::config::Config;
use crate::traffic::bindingset;
use crate::servlet::request_metadata::RequestMetadata;
use crate::{try_except_return, inc, ternary};
use crate::servlet::threading::thread_handler::ThreadHandler;

pub struct ListenerBinding {
    pub id: u64,
    pub listener: *const TcpListener,
    pub rule: bindingset::BindingRule,
}


pub struct Proxy {
    pub thread_handler_type: ThreadHandlerType,
    pub thread_pool: ThreadPool,
    pub listeners: Vec<ListenerBinding>
}

type Byte = u8;

static THREAD_POOL_SIZE_KEY: &'static str = "thread_pool_size";
static HANDLER_TYPE_KEY: &'static str = "thread_handler_type";

#[derive(Clone, Copy)]
pub enum ThreadHandlerType {
    CAPTURE,
    PROGRESSIVE,
}

impl fmt::Display for ThreadHandlerType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "could not read file: {}", match self {
            ThreadHandlerType::CAPTURE => "CAPTURE",
            ThreadHandlerType::PROGRESSIVE => "PROGRESSIVE"
        })
    }
}

impl FromStr for ThreadHandlerType {
    type Err = ();
    fn from_str(input: &str) -> Result<ThreadHandlerType, Self::Err> {
        match input {
            "CAPTURE"  => Ok(ThreadHandlerType::CAPTURE),
            "PROGRESSIVE"  => Ok(ThreadHandlerType::PROGRESSIVE),
            _ => Ok(ThreadHandlerType::CAPTURE),
        }
    }
}

type ThreadHandlerMethod = fn(TcpStream, TcpStream, Arc<Mutex<RequestMetadata>>);

impl Proxy {
    pub fn new(configuration: Config) -> Proxy {
        let thread_pool_size_str:Option<&String> = configuration.properties.get(THREAD_POOL_SIZE_KEY);
        let thread_pool_size: usize = ternary!{
            thread_pool_size_str.is_none(),
            50,
            thread_pool_size_str.unwrap().parse::<usize>().unwrap()
        };
        let thread_handler_type_str: Option<&String> = configuration.properties.get(HANDLER_TYPE_KEY);
        let thread_handler_type: ThreadHandlerType = ternary!{
            thread_handler_type_str.is_none(),
            ThreadHandlerType::PROGRESSIVE,
            thread_handler_type_str.unwrap().parse::<ThreadHandlerType>().unwrap()
        };
        debug!{crate::LOGGER, "Creating proxy thread pool of size: {}", thread_pool_size};
        Proxy {
            thread_handler_type,
            thread_pool: rayon::ThreadPoolBuilder::new().num_threads(thread_pool_size)
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
        let mut incremental_listener_id: u64 = 0;
        for rule in rule_set.bindings {
            let (proxy_addr, to_addr) = (rule.from, rule.to);
            let mut listener: TcpListener = try_except_return!{TcpListener::bind(proxy_addr.to_string()), "Unable to bind proxy address"};
            debug!{crate::LOGGER, "Binding listener [{}] to connection: {} <-> {} ", incremental_listener_id, rule.from, rule.to};
            self.listeners.push(ListenerBinding{
                id: incremental_listener_id,
                listener: &listener,
                rule,
            });
            debug!{crate::LOGGER, "Invoked acceptor thread for listener [{}] using hadler type [{}]", incremental_listener_id, self.thread_handler_type};
            let handler_type: ThreadHandlerType = self.thread_handler_type;
            self.thread_pool.spawn(move || Proxy::invoke_acceptor_handler(&mut listener, to_addr, handler_type));
            inc!{incremental_listener_id};
        }
    }
    fn invoke_acceptor_handler(listener_forward: &mut TcpListener, proxy_to: SocketAddr, handler_type: ThreadHandlerType) {
        loop {
            let (stream_forward, _addr) = listener_forward.accept().expect("Failed to accept connection");
            debug!(crate::LOGGER, "New connection");

            let sender_forward: TcpStream = try_except_return!{TcpStream::connect(proxy_to), "Failed to bind"};
            let sender_backward: TcpStream = try_except_return!{sender_forward.try_clone(), "Failed to clone stream"};
            let stream_backward: TcpStream = try_except_return!{stream_forward.try_clone(), "Failed to clone stream"};

            let metadata: Arc<Mutex<RequestMetadata>> = Arc::new(Mutex::new(RequestMetadata::new()));
            let metadata_clone_forward: Arc<Mutex<RequestMetadata>> = metadata.clone();
            let metadata_clone_backward: Arc<Mutex<RequestMetadata>> = metadata.clone();

            let forward_fn: ThreadHandlerMethod = match handler_type {
                ThreadHandlerType::CAPTURE => ThreadHandler::forward_thread_capture_handler,
                ThreadHandlerType::PROGRESSIVE => ThreadHandler::forward_thread_progressive_handler
            };

            let backward_fn: ThreadHandlerMethod = match handler_type {
                ThreadHandlerType::CAPTURE => ThreadHandler::backward_thread_capture_handler,
                ThreadHandlerType::PROGRESSIVE => ThreadHandler::backward_thread_progressive_handler
            };

            thread::spawn(move || forward_fn(stream_forward, sender_forward, metadata_clone_forward));
            thread::spawn(move || backward_fn(stream_backward, sender_backward, metadata_clone_backward));
        }
    }
    pub fn start(&mut self, binding_set: bindingset::BindingSet) {
        let binding_count: usize = binding_set.bindings.len();
        info!(crate::LOGGER, "Initializing proxy {} binding(s)", binding_count);
        self.initialize_bindings(binding_set);
        info!(crate::LOGGER, "Starting main listener loop");
        loop {}
    }
}