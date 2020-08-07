use std::path::{Path, PathBuf};

use failure::{format_err, Error};
use tokio::{
    fs,
    process::Command,
    sync::mpsc::{unbounded_channel, UnboundedSender},
    task::JoinHandle,
};

use crate::config::Handbrake;
use crate::disc::{DiscMetadata, DiscType};

pub struct HandbrakeProcess {
    pub handle: JoinHandle<Result<(), Error>>,
    tx: UnboundedSender<Job>,
}

#[derive(Debug)]
struct Job {
    src: PathBuf,
    dest: PathBuf,
}

impl HandbrakeProcess {
    pub fn new(config: Handbrake) -> HandbrakeProcess {
        let (tx, mut rx) = unbounded_channel();

        let handle = tokio::spawn(async move {
            for job in rx.recv().await {
                let job: Job = job;

                mkv(&config, &job.src, &job.dest).await?;
            }

            Ok(())
        });

        HandbrakeProcess { handle, tx }
    }

    pub async fn queue(&self, src: PathBuf, dest: PathBuf) -> Result<(), Error> {
        fs::create_dir_all(&dest).await?;

        self.tx.send(Job { src, dest })?;

        Ok(())
    }
}

async fn mkv(config: &Handbrake, src: &Path, dest: &Path) -> Result<(), Error> {
    let disc_meta: DiscMetadata = toml::from_slice(&fs::read(src.join("meta.toml")).await?)?;

    let args = match disc_meta.disc_type {
        DiscType::DVD => &config.dvd,
        DiscType::BluRay => &config.bluray,
        _ => unimplemented!(),
    };

    let dest = dest.join(src.file_name().unwrap());

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

            let child = Command::new("HandBrakeCLI")
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

            let status = child.await?;

            if !status.success() {
                return Err(format_err!(
                    "error code {:?} from handbrake, stopping process",
                    status.code()
                ));
            }
        }
    }

    let mut files = fs::read_dir(src).await?;

    while let Ok(Some(entry)) = files.next_entry().await {
        fs::remove_file(entry.path()).await?;
    }

    Ok(())
}
