use std::path::Path;
use std::sync::Arc;

use argh::FromArgs;
use failure::Error;
use futures::future::try_join_all;
use log::warn;
use tokio::{fs, task::JoinHandle, time::delay_for};

use crate::config::Settings;
use crate::disc::{Disc, DiscType};

mod config;
mod disc;
mod handbrake;
mod makemkv;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args: Args = argh::from_env();

    let settings = config::Settings::new()?;

    match args.command {
        Command::RIP(_) => {
            rip(settings).await?;
        }
        Command::Debug(_) => {
            for device in settings.options.devices {
                let disc = Disc::new(&device);

                println!("{:#?}", disc);
            }
        }
    }

    Ok(())
}

async fn rip(settings: Settings) -> Result<(), Error> {
    let (hb_process, hb_handle) = handbrake::HandbrakeProcess::new(settings.handbrake.clone());
    let hb_process = Arc::new(hb_process);

    let mut folders = fs::read_dir(&settings.directory.raw).await?;

    while let Ok(Some(entry)) = folders.next_entry().await {
        if entry.path().is_dir() && entry.path().join("meta.toml").is_file() {
            hb_process
                .queue(
                    entry.path(),
                    Path::new(&settings.directory.output).to_path_buf(),
                )
                .await?;
        }
    }

    let mut handles = Vec::with_capacity(settings.options.devices.len() + 1);

    handles.push(hb_handle);

    for device in settings.options.devices.clone() {
        let settings = settings.clone();
        let hb_process = hb_process.clone();

        let handle: JoinHandle<Result<(), Error>> = tokio::spawn(async move {
            loop {
                let device = device.to_owned();
                let raw = Path::new(&settings.directory.raw);
                let dest = Path::new(&settings.directory.output);

                let disc = Disc::new(&device);

                if !fs::File::open(device).await.is_err() {
                    match &disc.r#type {
                        Some(DiscType::DVD) | Some(DiscType::BluRay) => {
                            let rip_target_folder = raw.join(disc.path_friendly_title());
                            let rip_target_folder =
                                makemkv::rip(&settings.makemkv, &disc, &rip_target_folder).await?;
                            hb_process
                                .queue(rip_target_folder, dest.to_path_buf())
                                .await?;
                            disc::eject(&disc);
                        }
                        Some(t) => {
                            warn!("Disc type {:?} currently unsupported", t);
                            disc::eject(&disc);
                        }
                        None => {
                            warn!("Unkown disc type");
                            disc::eject(&disc);
                        }
                    }
                }

                delay_for(settings.options.sleep_time).await;
            }
        });

        handles.push(handle);
    }

    let results = try_join_all(handles).await?;

    for res in results {
        if let Err(err) = res {
            warn!("Error: {}", err);
        }
    }

    Ok(())
}

#[derive(FromArgs)]
/// start.
struct Args {
    #[argh(subcommand)]
    command: Command,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum Command {
    RIP(CommandRIP),
    Debug(CommandDebug),
}

#[derive(FromArgs, PartialEq, Debug)]
/// running ripping loop.
#[argh(subcommand, name = "rip")]
struct CommandRIP {}

#[derive(FromArgs, PartialEq, Debug)]
/// prints debug information about disc.
#[argh(subcommand, name = "debug")]
struct CommandDebug {}
