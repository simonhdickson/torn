use config::{Config, ConfigError, File, FileFormat};
use serde_derive::Deserialize;
use std::time::Duration;

#[derive(Clone, Debug, Deserialize)]
pub struct Options {
    #[serde(with = "humantime_serde")]
    pub sleep_time: Duration,
    pub devices: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Directory {
    pub logs: String,
    pub raw: String,
    pub output: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MakeMKV {
    pub enqueue_existing_jobs: bool,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Handbrake {
    pub delete_on_complete: bool,
    pub dvd: HandbrakeArgs,
    pub bluray: HandbrakeArgs,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HandbrakeArgs {
    pub extension: String,
    pub preset: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Settings {
    pub options: Options,
    pub directory: Directory,
    pub makemkv: MakeMKV,
    pub handbrake: Handbrake,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        s.merge(File::from_str(
            include_str!("../config/default.toml"),
            FileFormat::Toml,
        ))?;

        s.merge(File::with_name("config").required(false))?;

        s.try_into()
    }
}
