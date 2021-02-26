use std::net::TcpStream;
use std::sync::{MutexGuard, Mutex, Arc};
use std::io::{BufReader, BufRead, Write};

use crate::traffic::packet::stream_packet_collector::StreamPacketCollector;
use crate::servlet::request_metadata::RequestMetadata;
use crate::inc;
use core::fmt;
use std::str::FromStr;

#[derive(Clone, Copy)]
pub(crate) enum ThreadHandlerType {
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

pub(crate) type ThreadHandlerMethod = fn(TcpStream, TcpStream, Arc<Mutex<RequestMetadata>>);

pub(crate) struct ThreadHandler;

type Byte = u8;

impl ThreadHandler {
    pub fn forward_thread_capture_handler(stream_forward: TcpStream, sender_forward: TcpStream, metadata: Arc<Mutex<RequestMetadata>>) {
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
    pub fn forward_thread_progressive_handler(stream_forward: TcpStream, mut sender_forward: TcpStream, metadata: Arc<Mutex<RequestMetadata>>) {
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
    pub fn backward_thread_progressive_handler(mut stream_backward: TcpStream, sender_backward: TcpStream, metadata: Arc<Mutex<RequestMetadata>>) {
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
    pub fn backward_thread_capture_handler(stream_backward: TcpStream, sender_backward: TcpStream, metadata: Arc<Mutex<RequestMetadata>>) {
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
}