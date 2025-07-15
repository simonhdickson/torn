use std::{env, path::Path};

use argh::FromArgs;
use failure::Error;
use futures::future::try_join_all;
use log::{error, info, warn};
use tokio::time::sleep;
use tokio::{fs, task::JoinHandle};

use crate::config::Settings;
use crate::disc::{Disc, DiscType};
use crate::handbrake::HandbrakeProcess;

mod config;
mod disc;
mod handbrake;
mod makemkv;
mod web;

#[tokio::main]
async fn main() -> Result<(), Error> {
    if env::var_os("RUST_LOG").is_none() {
        // nothing else should be running right now
        // so this is as safe as it can be for now
        unsafe {
            env::set_var("RUST_LOG", "torn=info");
        }
    }

    pretty_env_logger::init();

    let args: Args = argh::from_env();

    let settings = config::Settings::new()?;

    match args.command {
        Command::Rip(_) => {
            rip(settings).await?;
        }
        Command::Debug(_) => {
            println!("Settings: {:#?}", settings);

            for device in settings.options.devices {
                let disc = Disc::new(&device);

                println!("{:#?}", disc);
            }
        }
    }

    Ok(())
}

async fn rip(settings: Settings) -> Result<(), Error> {
    let (hb_process, hb_handle) = HandbrakeProcess::new(settings.handbrake.clone());

    process_existing_directories(&hb_process, &settings).await?;

    let mut handles = Vec::with_capacity(settings.options.devices.len() + 2);

    handles.push(hb_handle);

    // Start web interface in background
    let web_settings = settings.clone();
    let web_hb_process = hb_process.clone();
    let web_handle = tokio::spawn(async move {
        if let Err(e) = web::run_web_server(web_settings, web_hb_process).await {
            warn!("Web interface error: {}", e);
        }
        Ok(())
    });
    handles.push(web_handle);

    info!("Web interface started at http://localhost:8080");

    for device in settings.options.devices.clone() {
        let settings = settings.clone();
        let hb_process = hb_process.clone();

        let handle = spawn_rip_process(device, settings, hb_process);

        handles.push(handle);
    }

    let results = try_join_all(handles).await?;

    for res in results {
        if let Err(err) = res {
            error!("Error: {}", err);
        }
    }

    info!("exiting rip process");

    Ok(())
}

fn spawn_rip_process(
    device: String,
    settings: Settings,
    hb_process: HandbrakeProcess,
) -> JoinHandle<Result<(), Error>> {
    tokio::spawn(async move {
        loop {
            let device = device.to_owned();
            let raw = Path::new(&settings.directory.raw);
            let dest = Path::new(&settings.directory.output);
            let disc = Disc::new(&device);

            if fs::File::open(device).await.is_ok() {
                match &disc.r#type {
                    Some(DiscType::Dvd) | Some(DiscType::BluRay) => {
                        let rip_target_folder = raw.join(disc.path_friendly_title());
                        println!("{}", rip_target_folder.display());
                        let rip_target_folder =
                            makemkv::rip(&settings.makemkv, &disc, &rip_target_folder).await?;
                        hb_process
                            .queue(rip_target_folder, dest.to_path_buf())
                            .await?;
                        info!("Finished ripping disc!");
                        disc::eject(&disc).await;
                    }
                    Some(t) => {
                        warn!("Disc type {:?} currently unsupported", t);
                        disc::eject(&disc).await;
                    }
                    None => {
                        warn!("Unknown disc type");
                        disc::eject(&disc).await;
                    }
                }
            }

            sleep(settings.options.sleep_time).await;
        }
    })
}

async fn process_existing_directories(
    hb_process: &HandbrakeProcess,
    settings: &Settings,
) -> Result<(), Error> {
    if settings.makemkv.enqueue_existing_jobs {
        if !Path::new(&settings.directory.raw).exists() {
            return Ok(());
        }

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
    Rip(CommandRIP),
    Debug(CommandDebug),
}

#[derive(FromArgs, PartialEq, Debug)]
/// Start the DVD/Blu-ray ripper with integrated web interface on http://localhost:8080
#[argh(subcommand, name = "rip")]
struct CommandRIP {}

#[derive(FromArgs, PartialEq, Debug)]
/// prints debug information about disc.
#[argh(subcommand, name = "debug")]
struct CommandDebug {}
