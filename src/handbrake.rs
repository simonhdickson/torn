use std::path::{Path, PathBuf};

use failure::{format_err, Error};
use log::info;
use tokio::{
    fs,
    process::Command,
    sync::mpsc::{unbounded_channel, UnboundedSender},
    task::JoinHandle,
};

use crate::config::Handbrake;
use crate::disc::{DiscMetadata, DiscType};

#[derive(Clone)]
pub struct HandbrakeProcess {
    tx: UnboundedSender<Job>,
}

#[derive(Debug)]
struct Job {
    src: PathBuf,
    dest: PathBuf,
}

impl HandbrakeProcess {
    pub fn new(config: Handbrake) -> (HandbrakeProcess, JoinHandle<Result<(), Error>>) {
        let (tx, mut rx) = unbounded_channel();

        let handle = tokio::spawn(async move {
            while let Some(job) = rx.recv().await {
                let job: Job = job;

                handbrake(&config, &job.src, &job.dest).await?;
            }

            info!("exiting handbrake process");

            Ok(())
        });

        (HandbrakeProcess { tx }, handle)
    }

    pub async fn queue(&self, src: PathBuf, dest: PathBuf) -> Result<(), Error> {
        self.tx.send(Job { src, dest })?;

        Ok(())
    }
}

async fn handbrake(config: &Handbrake, src: &Path, dest: &Path) -> Result<(), Error> {
    let disc_meta: DiscMetadata = toml::from_slice(&fs::read(src.join("meta.toml")).await?)?;

    let args = match disc_meta.disc_type {
        DiscType::Dvd => &config.dvd,
        DiscType::BluRay => &config.bluray,
        _ => unimplemented!(),
    };

    let dest = dest.join(src.file_name().unwrap());

    fs::create_dir_all(&dest).await?;

    let mut files = fs::read_dir(src).await?;

    while let Ok(Some(entry)) = files.next_entry().await {
        if "toml" == entry.path().extension().unwrap().to_str().unwrap() {
            continue;
        }

        let path = entry.path();

        if path.is_file() {
            let mut output_file = path.clone();
            output_file.set_extension(&args.extension);

            let source_file = path
                .to_str()
                .ok_or_else(|| format_err!("path is not a valid string: {:?}", path))?;

            let dest_file = dest.join(
                output_file
                    .file_name()
                    .ok_or_else(|| format_err!("path is not a valid string: {:?}", dest))?,
            );
            let dest_file = dest_file
                .to_str()
                .ok_or_else(|| format_err!("path is not a valid string: {:?}", dest_file))?;

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

            let status = child.wait().await?;

            if !status.success() {
                return Err(format_err!(
                    "error code {:?} from handbrake, stopping process",
                    status.code()
                ));
            }
        }
    }

    info!(
        "finished handbake processing into directory {}",
        dest.to_str().unwrap()
    );

    let mut files = fs::read_dir(src).await?;

    if config.delete_on_complete {
        while let Ok(Some(entry)) = files.next_entry().await {
            fs::remove_file(entry.path()).await?;
        }
    }

    Ok(())
}
