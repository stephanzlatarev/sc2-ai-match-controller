use config::{Config, Environment, File, FileFormat};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ControllerConfig {
    pub version: String,
    pub api_url: String,
    pub gamesets_directory: String,
    pub bots_directory: String,
    pub logs_directory: String,
    pub matches_file: String,
}

pub fn initialize_config() -> ControllerConfig {
    Config::builder()
        .add_source(File::from_str(include_str!("config.toml"), FileFormat::Toml).required(true))
        .add_source(File::new("config.toml", FileFormat::Toml).required(false))
        .add_source(File::new("config.json", FileFormat::Json).required(false))
        .add_source(Environment::default())
        .build()
        .expect("Could not load the client controller configuration")
        .try_deserialize::<ControllerConfig>()
        .expect("Could not deserialize the client controller configuration")
}

#[derive(Debug, Clone, Default)]
pub struct MatchRequest {
    pub bot1_id: String,
    pub bot1_name: String,
    pub bot2_id: String,
    pub bot2_name: String,
}

impl MatchRequest {

    pub fn from_csv_line(line: &str) -> Self {
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        
        Self {
            bot1_id: parts[0].to_string(),
            bot1_name: parts[1].to_string(),
            bot2_id: parts[4].to_string(),
            bot2_name: parts[5].to_string(),
        }
    }

}

