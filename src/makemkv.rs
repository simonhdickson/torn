use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

use failure::{format_err, Error};
use tokio::{fs, process::Command};

use crate::config::MakeMKV;
use crate::disc::{Disc, DiscMetadata};

pub async fn rip(config: &MakeMKV, disc: &Disc, target_folder: &Path) -> Result<PathBuf, Error> {
    let target_folder = if Path::new(target_folder).exists() {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();

        target_folder.with_file_name(format!(
            "{}_{}",
            target_folder.file_name().unwrap().to_str().unwrap(),
            timestamp,
        ))
    } else {
        target_folder.to_owned()
    };

    fs::create_dir_all(&target_folder).await?;

    let child = Command::new("makemkvcon")
        .args(&[
            "mkv",
            "-r",
            &format!("dev:{}", disc.name),
            "all",
            target_folder.to_str().unwrap(),
            "--minlength=600",
        ])
        .args(&config.args)
        .spawn()
        .expect("failed to execute process");

    let status = child.await?;

    if !status.success() {
        return Err(format_err!(
            "error code {:?} from handbrake, stopping process",
            status.code()
        ));
    }

    let toml = toml::to_string(&DiscMetadata {
        disc_type: disc.r#type.unwrap(),
    })?;

    fs::write(target_folder.join("meta.toml"), toml).await?;

    Ok(target_folder)
}
