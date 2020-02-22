use std::{
    fs,
    io::{self, Write},
};
use std::{
    path::Path,
    process::{Command, Stdio},
};

use crate::config::Handbrake;
use crate::disc::{Disc, DiscType};

pub fn mkv(config: &Handbrake, src: &Path, dest: &Path, disc: &Disc) {
    let args = match disc.r#type {
        Some(DiscType::DVD) => &config.dvd,
        Some(DiscType::BluRay) => &config.bluray,
        _ => unimplemented!(),
    };

    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            let mut output_file = path.clone();
            output_file.set_extension(&args.extension);
            let source_file = path.to_str().unwrap();
            let dest_file = dest.join(output_file.file_name().unwrap().to_str().unwrap());
            let dest_file = dest_file.to_str().unwrap();

            let mut child = Command::new("HandBrakeCLI")
                .args(&[
                    "-i",
                    source_file,
                    "-o",
                    dest_file,
                    "--preset",
                    &args.preset,
                    "--subtitle",
                    "scan",
                    "-F",
                ])
                .args(&args.args)
                .spawn()
                .expect("failed to execute process");

            child.wait().unwrap();
        }
    }
}
