use crate::traffic::bindingset::BindingRule;
use std::collections::HashSet;

pub struct Enforcer {
    active: bool,
    bindings: HashSet<BindingRule>,
}