use std::sync::Arc;
use std::{error::Error, fs, path::Path, thread};

use argh::FromArgs;
use log::warn;

use crate::config::Settings;
use crate::disc::{Disc, DiscType};

mod config;
mod disc;
mod handbrake;
mod makemkv;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = argh::from_env();

    let settings = config::Settings::new()?;

    match args.command {
        Command::RIP(_) => {
            rip(settings)?;
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

fn rip(settings: Settings) -> Result<(), Box<dyn Error>> {
    let mkv_process = Arc::new(handbrake::MkvProcess::new(settings.handbrake.clone()));

    let mut handles = Vec::with_capacity(settings.options.devices.len());

    for device in settings.options.devices.clone() {
        let settings = settings.clone();
        let mkv_process = mkv_process.clone();

        let handle = thread::spawn(move || loop {
            let device = device.to_owned();
            let raw = Path::new(&settings.directory.raw);
            let dest = Path::new(&settings.directory.output);

            let disc = Disc::new(&device);

            if !fs::File::open(device).is_err() {
                match &disc.r#type {
                    Some(DiscType::DVD) | Some(DiscType::BluRay) => {
                        let rip_target_folder = raw.join(disc.path_friendly_title());
                        let mkv_target_folder = dest.join(disc.path_friendly_title());
                        makemkv::rip(&settings.makemkv, &disc, &rip_target_folder).unwrap();
                        mkv_process.queue(rip_target_folder, mkv_target_folder, disc.clone());
                        disc::eject(&disc);
                    }
                    Some(t) => {
                        warn!("Disc type {:?} currently unsupported", t);
                    }
                    None => (),
                }
            }

            thread::sleep(settings.options.sleep_time);
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
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
