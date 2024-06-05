use crate::proxy::ProxyId;
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

const CONFIG_DIR: &'static str = ".yatangaki";
const CONFIG_FILE: &'static str = "config.toml";
const CA_PATH: &'static str = "certificate_authority.der";

#[derive(Deserialize, Serialize)]
pub struct ProxyConfig {
    port: u16,
    id: ProxyId,
    //auto_start: bool
}

#[derive(Deserialize, Serialize, Default)]
pub struct ProjectConfig {
    proxies: Vec<ProxyConfig>,
    //  projects
}

impl ProjectConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let mut full_config_dir_buf: PathBuf =
            [&env::var("HOME").unwrap(), CONFIG_DIR].iter().collect();

        let full_config_dir = full_config_dir_buf.as_path();
        if !full_config_dir.exists() {
            fs::create_dir_all(full_config_dir)?;
        }

        full_config_dir_buf.push(CONFIG_FILE);

        let full_config_file = full_config_dir_buf.as_path();
        if full_config_file.exists() {
            let raw_config = fs::read_to_string(full_config_file)?;
            let config: Self = toml::from_str(&raw_config)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        let raw_config = toml::to_string(self).unwrap();

        let config_path = [&env::var("HOME").unwrap(), CONFIG_DIR, CONFIG_FILE]
            .iter()
            .collect::<PathBuf>();

        fs::write(config_path, &raw_config)?;
        Ok(())
    }
}
