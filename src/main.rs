use std::{path::Path, thread, time, fs};

use argh::FromArgs;

use disc::{Disc, DiscType};

mod config;
mod disc;
mod handbrake;
mod makemkv;
mod util;

fn main() {
    let args: Args = argh::from_env();

    let dev = "/dev/sr0";

    match args.command {
        Command::RIP(_) => {
            rip(dev);
        }
        Command::Debug(_) => {
            let disc = Disc::new(dev);

            println!("name : {}", disc.name);
            println!("type : {:?}", disc.r#type);

            for (name, value) in disc.properties {
                println!("property {} = {}", name, value);
            }
        }
    }
}

fn rip(dev: &str) {
    let settings = config::Settings::new().unwrap();

    let raw = Path::new(&settings.directory.raw);
    let dest = Path::new(&settings.directory.output);

    loop {
        let disc = Disc::new(dev);

        if !fs::File::open(dev).is_err() {
            match &disc.r#type {
                Some(DiscType::DVD) => {
                    let ripped_path = makemkv::rip(&settings.makemkv, &disc, raw);
                    handbrake::mkv(&settings.handbrake, &ripped_path, dest, &disc);
                    disc.eject();
                }
                Some(_) => unimplemented!(),
                None => (),
            }
        }

        thread::sleep(time::Duration::from_secs(60));
    }
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
/// Second subcommand.
#[argh(subcommand, name = "debug")]
struct CommandDebug {}
