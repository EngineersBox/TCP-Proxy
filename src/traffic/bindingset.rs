use std::net::SocketAddr;
use std::collections::HashSet;
use std::vec::Vec;
use std::fs;
use std::str::FromStr;
use crate::traffic::json_mappings::*;

// ---- Enums ----

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum RuleType {
    HEADER, // Expects "header_mappings": [ { "key": "header name", "value": "header value" } ]
    URL, // Expects "url_wildcard": "regex\sstring"
    METHOD, // Expects "method_enum": "<GET | POST | DELETE | PATCH | PUT | OPTIONS>"
    VERSION, // Expects "version_float": <0.9 | 1.0 | 1.1 | 2.0 | 3.0>
}

impl FromStr for RuleType {
    type Err = ();
    fn from_str(input: &str) -> Result<RuleType, Self::Err> {
        match input {
            "HEADER"  => Ok(RuleType::HEADER),
            "URL"  => Ok(RuleType::URL),
            "METHOD"  => Ok(RuleType::METHOD),
            "VERSION"  => Ok(RuleType::VERSION),
            _ => Ok(RuleType::URL),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum HttpMethod {
    GET,
    POST,
    DELETE,
    PATCH,
    PUT,
    OPTIONS,
}

impl FromStr for HttpMethod {
    type Err = ();
    fn from_str(input: &str) -> Result<HttpMethod, Self::Err> {
        match input {
            "GET"  => Ok(HttpMethod::GET),
            "POST"  => Ok(HttpMethod::POST),
            "DELETE"  => Ok(HttpMethod::DELETE),
            "PATCH"  => Ok(HttpMethod::PATCH),
            "PUT"  => Ok(HttpMethod::PUT),
            "OPTIONS"  => Ok(HttpMethod::OPTIONS),
            _ => Ok(HttpMethod::GET),
        }
    }
}


// ---- Programmatic Structs ----

// ---- HeaderMapping ----

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct HeaderMapping {
    key: String,
    value: String,
}

impl HeaderMapping {
    pub fn new(key: String, value: String) -> HeaderMapping {
        HeaderMapping {
            key,
            value,
        }
    }
}

// ---- Rule ----

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct Rule {
    pub kind: RuleType,
    pub header_mappings: Vec<HeaderMapping>,
    pub url_wildcard: String,
    pub method_enum: HttpMethod,
    pub version_float: String,
}

impl Rule {
    pub fn new(kind: RuleType) -> Rule {
        Rule {
            kind,
            header_mappings: vec![],
            url_wildcard: String::from("."),
            method_enum: HttpMethod::GET,
            version_float: String::from("1.1"),
        }
    }
    pub fn add_header_mapping(&mut self, mapping: HeaderMapping) {
        self.header_mappings.push(mapping);
    }
    pub fn add_all_header_mappings(&mut self, mappings: Vec<HeaderMapping>) {
        self.header_mappings = mappings;
    }
    pub fn set_url_wildcard(&mut self, wildcard: String) {
        self.url_wildcard = wildcard;
    }
    pub fn set_method_enum(&mut self, method: HttpMethod) {
        self.method_enum = method;
    }
    pub fn st_version_float(&mut self, version: String) {
        self.version_float = version;
    }
}

// ---- RuleSet ----

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct RuleSet {
    pub egress: Vec<Rule>,
    pub ingress: Vec<Rule>,
}

impl RuleSet {
    pub fn new() -> RuleSet {
        RuleSet {
            egress: vec![],
            ingress: vec![],
        }
    }
    pub fn add_egress_rule(&mut self, egress_rule: Rule) {
        self.egress.push(egress_rule)
    }
    pub fn add_ingress_rule(&mut self, ingress_rule: Rule) {
        self.ingress.push(ingress_rule)
    }
}

// ---- BindingRule ----

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct BindingRule {
    pub name: String,
    pub from: SocketAddr,
    pub to: SocketAddr,
    pub rules: RuleSet
}

impl BindingRule {
    pub fn new(name: String, from: SocketAddr, to: SocketAddr, rules: RuleSet) -> BindingRule {
        BindingRule {
            name,
            from,
            to,
            rules,
        }
    }

    pub fn from_to_string(from_str: String, to_str: String) -> BindingRule {
        BindingRule::new(
            uuid::Uuid::new_v4().to_string(),
            from_str.parse::<SocketAddr>().unwrap(),
            to_str.parse::<SocketAddr>().unwrap(),
            RuleSet::new(),
        )
    }
}

// ---- BindingSet ----

#[derive(Debug)]
pub struct BindingSet {
    pub id: String,
    pub applied: bool,
    pub bindings: HashSet<BindingRule>,
}

impl BindingSet {
    pub fn new(id: String) -> BindingSet {
        BindingSet {
            id,
            applied: false,
            bindings: HashSet::new(),
        }
    }
    pub fn add_rule(&mut self, rule: BindingRule) -> bool {
        self.bindings.insert(rule)
    }
    pub fn add_rules(&mut self, rules: Vec<BindingRule>) -> Vec<bool> {
        let mut rules_present: Vec<bool> = Vec::new();
        for rule in rules {
            rules_present.push(self.add_rule(rule));
        }
        return rules_present;
    }
    pub fn from_file(id: String, filename: String) -> BindingSet {
        let data: String = fs::read_to_string(filename).expect("Unable to read file");
        let parsed: JSONBindingSet = serde_json::from_str(data.as_str()).unwrap();
        BindingSet {
            id,
            applied: false,
            bindings: assemble_bindings_from_json(parsed),
        }
    }
    pub fn set_applied(&mut self, new_applied_setting: bool) {
        self.applied = new_applied_setting;
    }
}

fn assemble_rules_from_json(json_val: JSONRule) -> Rule {
    let mut rule: Rule = Rule::new(RuleType::from_str(json_val.kind.as_str()).unwrap());
    for mapping in json_val.header_mappings {
        rule.add_header_mapping(HeaderMapping::new(mapping.key, mapping.value));
    }
    rule.url_wildcard = json_val.url_wildcard;
    rule.method_enum = HttpMethod::from_str(json_val.method_enum.as_str()).unwrap();
    rule.version_float = json_val.version_float.to_string();
    rule
}

fn assemble_bindings_from_json(json_val: JSONBindingSet) -> HashSet<BindingRule> {
    let mut binding_rule_set: HashSet<BindingRule> = HashSet::new();
    for binding in json_val.bindings {
        let mut ruleset: RuleSet = RuleSet::new();
        for json_rule in binding.rules.egress {
            ruleset.add_egress_rule(assemble_rules_from_json(json_rule));
        }
        for json_rule in binding.rules.ingress {
            ruleset.add_ingress_rule(assemble_rules_from_json(json_rule));
        }
        binding_rule_set.insert(BindingRule::new(
            binding.name,
            binding.from.parse::<SocketAddr>().unwrap(),
            binding.to.parse::<SocketAddr>().unwrap(),
            ruleset,
        ));
    }
    return binding_rule_set;
}