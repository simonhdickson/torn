use std::process::Command;
use std::{
    fs,
    path::Path,
};

use crate::config::MakeMKV;
use crate::disc::Disc;

pub fn rip(config: &MakeMKV, disc: &Disc, target_folder: &Path) {
    fs::create_dir_all(&target_folder).unwrap();

    let mut child = Command::new("makemkvcon")
        .args(&[
            "mkv",
            "-r",
            &format!("dev:{}", disc.name),
            "all",
            &target_folder.to_str().unwrap(),
            "--minlength=600",
        ])
        .args(&config.args)
        .spawn()
        .expect("failed to execute process");

    child.wait().unwrap();
}
