use crate::traffic::bindingset::BindingRule;
use std::collections::{HashSet, VecDeque};
use pnet::packet::tcp::TcpPacket;

pub struct Enforcer {
    active: bool,
    bindings: HashSet<BindingRule>,
}

impl Enforcer {

}

pub struct TransferFilterService {
    active: bool,
    bindings: HashSet<BindingRule>,
    packet_buf: VecDeque<TcpPacket<'static>>,
}