use std::collections::VecDeque;
use pnet::packet::tcp::TcpPacket;

pub struct PacketHandler<'a> {
    pub packet_buf: VecDeque<TcpPacket<'a>>
}