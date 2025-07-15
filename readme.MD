# Torn

A minimalist DVD/Bru-ray ripper. Still very much a WIP.

Heavily inspired by [ARM](https://github.com/automatic-ripping-machine/automatic-ripping-machine), but with the goal to run entirely
in user space (and also within docker but this is currently untested).

Now includes an integrated web interface for monitoring drive status and encoding jobs!


## Features

- Detects insertion of disc using udev
- Determines disc type
  - If (Blu-Ray or DVD)
    - Rip using MakeMKV
    - Eject disc and queue up Handbrake transcoding job when done.
    - Transcoding jobs are asynchronously batched from ripping
- Headless, designed to be run from a server
- Can rip from multiple-optical drives in parallel (just need to add additional drives to config file)
- Will enqueue transcoding jobs for previously ripped files in case of failure.
- Integrated web interface for monitoring drive status and HandBrake jobs

Requirements:

* makemkvcon
* HandBrakeCLI

If you get errors related to not being able to find disc drive, you may need run `sudo modprobe sg` (this works for me).

Usage:

```
cargo install torn

# Start the ripper daemon (includes web interface at http://localhost:8080)
torn rip
```

By default will output into current working directory but can be configured by placing a `config.toml` file.
See [config/default.toml](config/default.toml) for usage.

## Web Interface

The web interface is automatically started when you run `torn rip` and provides a real-time dashboard showing:

- **Drive Status**: View all configured drives, whether they have discs inserted, disc types, and titles
- **HandBrake Jobs**: Monitor active encoding jobs with progress bars and queue status
- **Remote Control**: Eject discs remotely from the web interface

Access the web interface at `http://localhost:8080` while the rip process is running.
