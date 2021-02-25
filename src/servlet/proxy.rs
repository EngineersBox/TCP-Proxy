use std::io::{BufRead, BufReader, Write, Read};
use std::net::SocketAddr;
use std::net::{TcpListener, TcpStream};
use crate::configuration::config::Config;
use rayon::ThreadPool;
use crate::traffic::bindingset;
use std::thread;
use crate::servlet::request_metadata::RequestMetadata;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use crate::{try_except_return, inc, ternary};
use crate::traffic::packet::stream_packet_collector::StreamPacketCollector;

pub struct ListenerBinding {
    pub id: u64,
    pub listener: *const TcpListener,
    pub rule: bindingset::BindingRule,
}


pub struct Proxy {
    pub thread_pool: ThreadPool,
    pub listeners: Vec<ListenerBinding>
}

type Byte = u8;

static THREADPOOL_SIZE_KEY: &'static str = "threadpool_size";

impl Proxy {
    pub fn new(configuration: Config) -> Proxy {
        let threadpool_size_str:Option<&String> = configuration.properties.get(THREADPOOL_SIZE_KEY);
        let threadpool_size: usize = ternary!{
            threadpool_size_str.is_none(),
            50,
            threadpool_size_str.unwrap().parse::<usize>().unwrap()
        };
        debug!{crate::LOGGER, "Creating proxy thread pool of size: {}", threadpool_size};
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
            debug!{crate::LOGGER, "Invoked acceptor thread for listener [{}]", incremental_listener_id};
            self.thread_pool.spawn(move || Proxy::invoke_acceptor_handler(&mut listener, to_addr));
            inc!{incremental_listener_id};
        }
    }
    fn invoke_acceptor_handler(listener_forward: &mut TcpListener, proxy_to: SocketAddr) {
        loop {
            let (stream_forward, _addr) = listener_forward.accept().expect("Failed to accept connection");
            debug!(crate::LOGGER, "New connection");

            let sender_forward: TcpStream = try_except_return!{TcpStream::connect(proxy_to), "Failed to bind"};
            let sender_backward: TcpStream = try_except_return!{sender_forward.try_clone(), "Failed to clone stream"};
            let stream_backward: TcpStream = try_except_return!{stream_forward.try_clone(), "Failed to clone stream"};

            let metadata: Arc<Mutex<RequestMetadata>> = Arc::new(Mutex::new(RequestMetadata::new()));
            let metadata_clone_forward: Arc<Mutex<RequestMetadata>> = metadata.clone();
            let metadata_clone_backward: Arc<Mutex<RequestMetadata>> = metadata.clone();

            thread::spawn(move || Proxy::forward_thread_handler(stream_forward, sender_forward, metadata_clone_forward));
            thread::spawn(move || Proxy::backward_thread_capture_handler(stream_backward, sender_backward, metadata_clone_backward));
        }
    }
    fn forward_thread_capture_handler(stream_forward: TcpStream, mut sender_forward: TcpStream, metadata: Arc<Mutex<RequestMetadata>>) {
        let mut packet_collector: StreamPacketCollector = StreamPacketCollector::new(sender_forward, stream_forward);
        let mut md: MutexGuard<RequestMetadata> = metadata.lock().unwrap();
        packet_collector.read_all_packets_from_stream();
        match packet_collector.write_buffer_to_remote() {
            Some(_) => {},
            None => { debug!{crate::LOGGER, "Connection closed"}; }
        };
        packet_collector.flush_stream_to_remote();
        md.tag_response_end_time();
        md.tag_request_start_time();
        info!(crate::LOGGER, "TRAFFIC LOG [EGRESS] [{}] [Packets: {}]", md.id, packet_collector.packet_count);
        debug!(crate::LOGGER, "REQUEST CONTENT [EGRESS]: {}", packet_collector.buffer_to_string().chars().as_str());
        debug!(crate::LOGGER, "Remote closed connection");
    }
    fn forward_thread_handler(stream_forward: TcpStream, mut sender_forward: TcpStream, metadata: Arc<Mutex<RequestMetadata>>) {
        let mut stream_forward: BufReader<TcpStream> = BufReader::new(stream_forward);
        let mut buffer: &[Byte];
        let mut buffer_length: usize;
        loop {
            buffer = stream_forward.fill_buf().unwrap();
            buffer_length = buffer.len();
            if buffer.is_empty() {
                debug!(crate::LOGGER, "Client closed connection");
                return;
            }
            sender_forward.write_all(&buffer).expect("Failed to write to remote");
            let mut md: MutexGuard<RequestMetadata> = metadata.lock().unwrap();
            debug!(crate::LOGGER, "REQUEST CONTENT [EGRESS]: {}", String::from_utf8_lossy(&buffer).chars().as_str());
            info!(crate::LOGGER, "TRAFFIC LOG [EGRESS] [{}]", md.id);
            md.tag_request_start_time();
            sender_forward.flush().expect("Failed to flush remote");
            stream_forward.consume(buffer_length);
        }
    }
    // "Progressive" refers to forwarding all packets as they come through
    fn backward_thread_progressive_handler(mut stream_backward: TcpStream, mut sender_backward: TcpStream, metadata: Arc<Mutex<RequestMetadata>>) {
        let mut sender_backward: BufReader<TcpStream> = BufReader::new(sender_backward);
        let mut buffer: &[Byte];
        let mut length: usize;
        loop {
            buffer = sender_backward.fill_buf().unwrap();
            length = buffer.len();
            let mut md: MutexGuard<RequestMetadata> = metadata.lock().unwrap();
            if buffer.is_empty() {
                md.tag_response_end_time();
                info!(crate::LOGGER, "TRAFFIC LOG [INGRESS] [{}] [Packets: {}] [{} ms]", md.id, md.response_packet_count, md.get_request_response_duration());
                debug!(crate::LOGGER, "Remote closed connection");
                return;
            }
            if stream_backward.write_all(&buffer).is_err() {
                debug!(crate::LOGGER, "Client closed connection");
                return;
            }

            debug!(crate::LOGGER, "RESPONSE CONTENT [EGRESS]: {}", String::from_utf8_lossy(&buffer).chars().as_str());
            inc!{md.response_packet_count};
            stream_backward.flush().expect("Failed to flush locally");
            sender_backward.consume(length);
        }
    }
    // "Capture" refers to reading all packets and sending as one packet to client
    fn backward_thread_capture_handler(mut stream_backward: TcpStream, sender_backward: TcpStream, metadata: Arc<Mutex<RequestMetadata>>) {
        let mut packet_collector: StreamPacketCollector = StreamPacketCollector::new(stream_backward, sender_backward);
        let mut md: MutexGuard<RequestMetadata> = metadata.lock().unwrap();
        packet_collector.read_all_packets_from_stream();
        match packet_collector.write_buffer_to_remote() {
            Some(_) => {},
            None => { debug!{crate::LOGGER, "Connection closed"}; }
        };
        packet_collector.flush_stream_to_remote();
        md.tag_response_end_time();
        info!(crate::LOGGER, "TRAFFIC LOG [INGRESS] [{}] [Packets: {}] [{} ms]", md.id, packet_collector.packet_count, md.get_request_response_duration());
        debug!(crate::LOGGER, "RESPONSE CONTENT [INGRESS]: {}", packet_collector.buffer_to_string().chars().as_str());
        debug!(crate::LOGGER, "Remote closed connection");
    }
    pub fn start(&mut self, binding_set: bindingset::BindingSet) {
        let binding_count: usize = binding_set.bindings.len();
        info!(crate::LOGGER, "Initializing proxy {} binding(s)", binding_count);
        self.initialize_bindings(binding_set);
        info!(crate::LOGGER, "Starting main listener loop");
        loop {}
    }
}