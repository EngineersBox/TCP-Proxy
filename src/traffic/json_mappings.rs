use std::vec::Vec;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct JSONHeaderMapping {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct JSONRule {
    pub kind: String, // Converted to RuleType,
    #[serde(default)]
    pub header_mappings: Vec<JSONHeaderMapping>,
    #[serde(default)]
    pub url_wildcard: String, // Converted to a regex expression
    #[serde(default)]
    pub method_enum: String, // Converted to HTTP method enum
    #[serde(default)]
    pub version_float: f32,
}

#[derive(Serialize, Deserialize, Default)]
pub struct JSONRuleSet {
    #[serde(default)]
    pub egress: Vec<JSONRule>,
    #[serde(default)]
    pub ingress: Vec<JSONRule>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct JSONBinding {
    pub name: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub rules: JSONRuleSet
}

#[derive(Serialize, Deserialize)]
pub struct JSONBindingSet {
    pub bindings: Vec<JSONBinding>
}