use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::collections::HashSet;
use std::vec::Vec;
use std::fs;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Hash)]
pub struct BindingRule {
    pub from: SocketAddr,
    pub to: SocketAddr
}

impl BindingRule {
    pub fn new(from: SocketAddr, to: SocketAddr) -> BindingRule {
        BindingRule {
            from,
            to,
        }
    }

    pub fn from_to_string(from_str: String, to_str: String) -> BindingRule {
        BindingRule {
            from: from_str.parse::<SocketAddr>().unwrap(),
            to: to_str.parse::<SocketAddr>().unwrap(),
        }
    }
}

pub struct RuleSet {
    pub id: String,
    pub rules: HashSet<BindingRule>,
}

#[derive(Serialize, Deserialize)]
struct JSONRule {
    from: String,
    to: String,
}

#[derive(Serialize, Deserialize)]
struct JSONRuleSet {
    rules: Vec<JSONRule>
}

impl RuleSet {
    pub fn new(id: String) -> RuleSet {
        RuleSet {
            id,
            rules: HashSet::new(),
        }
    }
    pub fn add_rule(&mut self, rule: BindingRule) -> bool {
        self.rules.insert(rule)
    }
    pub fn add_rules(&mut self, rules: Vec<BindingRule>) -> Vec<bool> {
        let mut rules_present: Vec<bool> = Vec::new();
        for rule in rules {
            rules_present.push(self.add_rule(rule));
        }
        return rules_present;
    }
    pub fn from_file(id: String, filename: String) -> RuleSet {
        let data = fs::read_to_string(filename).expect("Unable to read file");
        let parsed: JSONRuleSet = serde_json::from_str(data.as_str()).unwrap();
        RuleSet {
            id,
            rules: assemble_rules_from_json(parsed),
        }
    }
}

fn assemble_rules_from_json(json_val: JSONRuleSet) -> HashSet<BindingRule> {
    let mut binding_rule_set: HashSet<BindingRule> = HashSet::new();
    for rule in json_val.rules {
        binding_rule_set.insert(BindingRule::from_to_string(rule.from, rule.to));
    }
    return binding_rule_set;
}