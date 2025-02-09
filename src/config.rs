use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub executable_path: String,
    pub current_directory: Option<String>,
    pub dependencies: Option<Vec<String>>,
    pub args: Option<Vec<String>>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to read config file")]
    ReadFailed(#[from] std::io::Error),
    #[error("Failed to parse config file")]
    ParseFailed(#[from] toml::de::Error),
}

impl Config {
    pub fn from_file(path: &String) -> Result<Self, Error> {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        Ok(toml::from_str(&contents)?)
    }
}
