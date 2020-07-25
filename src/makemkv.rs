use std::process::Command;
use std::{fs, path::Path, time::SystemTime};

use crate::config::MakeMKV;
use crate::disc::Disc;

pub fn rip(config: &MakeMKV, disc: &Disc, target_folder: &Path) {
    let target_folder = if Path::new(target_folder).exists() {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        target_folder.with_file_name(format!(
            "{}_{}",
            target_folder.file_name().unwrap().to_str().unwrap(),
            timestamp,
        ))
    } else {
        target_folder.to_owned()
    };

    fs::create_dir_all(&target_folder).unwrap();

    let mut child = Command::new("makemkvcon")
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

    child.wait().unwrap();
}
