# sarctool

A tool for working with Nintendo SARCs with support for both big and little endian files, optionally supporting either yaz0 or zstd compression.

```
sarctool 1.0.0

USAGE:
    sarc <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    from-zip    
    help        Prints this message or the help of the given subcommand(s)
    into-zip    
    list        
    unzip       
    zip
```

## Install


Proper install (requires Rust to be installed, supports all Operating Systems):

```
cargo install --git https://github.com/jam1garner/sarctool
```

Rust-less install (Windows and Linux only):

1. Download from [the releases page](https://github.com/jam1garner/sarctool/releases) (no MacOS build available)
2. Copy the executable (`sarc` for Linux or `sarc.exe` for Windows) to either a folder added to path or to wherever you want to use it

## Build from source

```
   git clone https://github.com/jam1garner/sarctool
   cd sarctool
   cargo build --release
```

### Bug reporting

Get any crashes? Submit a bug report in the issues tab. Make sure to attach the problematic file(s).
