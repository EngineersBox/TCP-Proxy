mod configuration;
mod bindings;
mod servlet;
mod traffic;

use configuration::config;
use bindings::bindingset;
use servlet::proxy;
use simplelog::*;
use std::fs::File;
use std::fs;
use log::info;

extern crate simplelog;

fn initialize_logging() {
    let log_path = String::from("logs/");
    let directory_creation_message: &str;
    match fs::create_dir(log_path.as_str()) {
        Ok(_) => { directory_creation_message = "Created logging directory"; },
        Err(_) => { directory_creation_message = "Logging directory already exists, skipping";}
    }
    CombinedLogger::init(
        vec![
            TermLogger::new(
                LevelFilter::Warn,
                Config::default(),
                TerminalMode::Mixed,
            ),
            WriteLogger::new(
                LevelFilter::Info,
                Config::default(),
                fs::File::create(
                    log_path
                        + chrono::Utc::now().to_string().as_str()
                        + String::from(".log").as_str()
                ).unwrap(),
            ),
        ]
    ).unwrap();
    info!("{}", directory_creation_message);
}

fn main() {
    initialize_logging();
    let properties: config::Config = config::Config::new(String::from("config/config.properties"));
    let mut binding_set: bindingset::BindingSet = bindingset::BindingSet::from_file(String::from("1"), String::from("config/bindings.json"));
    binding_set.set_applied(true);
    let mut tcp_proxy: proxy::Proxy = proxy::Proxy::new(properties);
    tcp_proxy.start(binding_set);
}