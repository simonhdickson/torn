use config::{Config, ConfigError, File, FileFormat};
use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Directory {
    pub raw: String,
    pub output: String,
}

#[derive(Debug, Deserialize)]
pub struct MakeMKV {
    pub args: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Handbrake {
    pub dvd: HandbrakeArgs,
    pub bluray: HandbrakeArgs,
}

#[derive(Debug, Deserialize)]
pub struct OMDB {
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub struct HandbrakeArgs {
    pub extension: String,
    pub preset: String,
    pub args: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub directory: Directory,
    pub makemkv: MakeMKV,
    pub handbrake: Handbrake,
    pub omdb: OMDB,
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
