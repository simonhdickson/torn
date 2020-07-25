# Torn

A minimalist DVD/Bru-ray ripper. Still very much a WIP.

Heavily inspired by [ARM](https://github.com/automatic-ripping-machine/automatic-ripping-machine), but with the goal to run entirely
in user space (and also within docker but this is currently untested).

Requirements:

* makemkvcon
* HandBrakeCLI

If you get errors related to not being able to find disc drive, you may need run `sudo modprobe sg` (this works for me).

Usage:

```
cargo install --git https://github.com/simonhdickson/torn.git

torn rip
```

Loop is quite simple:

1 .Insert a disc.
2. Wait for eject.
3. Go to step 1.

By default will output into current working directory but can be configured by placing a `config.toml` file.
See [config/default.toml](config/default.toml) for usage.
