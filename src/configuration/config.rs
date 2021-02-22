use std::collections::HashMap;
use java_properties::PropertiesIter;
use std::fs::File;
use std::io::BufReader;

use crate::configuration::exceptions;
use std::path::Path;

pub struct Config {
    pub filename: String,
    pub properties: HashMap<String, String>
}

impl Config {
    pub fn new(filename: String) -> Config {
        Config {
            filename,
            properties: Default::default()
        }
    }
    pub fn read(&mut self){
        let path = Path::new(self.filename.as_str());
        let file = match File::open(&path) {
            Err(_) => panic!("{}", exceptions::FileError{filename: self.filename.clone()}),
            Ok(file) => file,
        };

        self.properties = HashMap::new();
        let err = PropertiesIter::new(BufReader::new(file)).read_into(|k, v| {
            self.properties.insert(k, v);
        }).err();
        if err.is_some() {
            panic!("Could not read properties: {}", err.unwrap());
        }
    }

    pub fn get(&mut self, key: String) -> Result<String, exceptions::ConfigPropertiesError> {
        if key.is_empty() {
            return Err(exceptions::ConfigPropertiesError::InvalidConfigPropertyKeyError{
                0:exceptions::InvalidConfigPropertyKeyError{key},
            });
        }
        let value = self.properties.get(key.as_str());
        if value.is_none() {
            return Err(exceptions::ConfigPropertiesError::MissingConfigPropertyError{
                0:exceptions::MissingConfigPropertyError{property: key.clone()},
            });
        }
        return Ok(String::from((*value.unwrap()).clone()));
    }
}