mod configuration;
use configuration::config;

mod rules;
use rules::ruleset;

mod servlet;
use servlet::proxy;

fn main() {
    let mut properties: config::Config = config::Config::new(String::from("config/config.properties"));
    let mut rule_sets: ruleset::RuleSet = ruleset::RuleSet::from_file(String::from("1"), String::from("config/ruleset.json"));
    let mut tcp_proxy: proxy::Proxy = proxy::Proxy::new(properties);
    tcp_proxy.initialize_bindings(rule_sets);
}