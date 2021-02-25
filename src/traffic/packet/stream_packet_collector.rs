use std::io::{Write, Read, BufReader, BufRead};
use std::net::TcpStream;

use crate::try_except_return_default;
use crate::try_except_return;

type Byte = u8;

pub struct StreamPacketCollector {
    packet_content_buffer: Vec<Byte>,
    receiver: TcpStream,
    sender: BufReader<TcpStream>,
    pub packet_count: i32,
}

impl StreamPacketCollector {
    pub fn new(mut receiver: TcpStream, sender: TcpStream) -> StreamPacketCollector {
        StreamPacketCollector {
            packet_content_buffer: vec![],
            receiver,
            sender: BufReader::new(sender),
            packet_count: 0,
        }
    }
    pub fn read_all_packets_from_stream(&mut self) -> usize {
        loop {
            let buffer: &[Byte] = self.sender.fill_buf().unwrap();
            let mut length: usize = buffer.len();
            if buffer.is_empty() {
                if self.receiver.write_all(self.packet_content_buffer.as_slice()).is_err() {
                    debug!(crate::LOGGER, "Connection closed");
                    return self.packet_content_buffer.len();
                }
                self.flush_stream_to_remote();
                return self.packet_content_buffer.len();
            }
            self.packet_content_buffer.extend_from_slice(buffer);
            self.sender.consume(length);
            self.packet_count += 1;
        }
    }
    pub fn flush_stream_to_remote(&mut self) {
        try_except_return!{
            self.receiver.flush(),
            "Failed to flush to remote"
        }
    }
    pub fn get_buffer(&mut self) -> &mut Vec<Byte> {
        self.packet_content_buffer.by_ref()
    }
    pub fn buffer_to_slice(&mut self) -> &[Byte] {
        self.packet_content_buffer.as_slice()
    }
    pub fn buffer_to_string(&mut self) -> String {
        String::from_utf8_lossy(self.packet_content_buffer.as_slice()).into_owned()
    }
    pub fn empty_buffer(&mut self) {
        self.packet_content_buffer.clear();
    }
}