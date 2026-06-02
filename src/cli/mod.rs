use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "cube", version, about = "A Rubik's Cube timer TUI application", long_about = None)]
pub(crate) struct Cli {
    #[arg(
        short,
        long,
        exclusive = true,
        help = "Print the data directory and exit"
    )]
    pub(crate) data: bool,
    #[arg(short, long, exclusive = true, help = "Print the config file and exit")]
    pub(crate) config: bool,
    #[command(subcommand)]
    pub(crate) subcommand: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    #[command(
        name = "import",
        alias = "i",
        about = "Imports a cstimer.json/cstimer.txt into the session history"
    )]
    Import {
        #[arg(value_name = "PATH", default_value = "cstimer.json")]
        path: PathBuf,
    },
    #[command(
        name = "export",
        alias = "e",
        about = "Exports the session history to a cstimer.json file"
    )]
    Export {
        #[arg(value_name = "PATH", default_value = "cstimer.json")]
        path: PathBuf,
    },
    #[cfg(feature = "dashboard")]
    #[command(
        name = "dashboard",
        alias = "d",
        about = "Starts a local dashboard for viewing data"
    )]
    Dashboard {
        #[arg(
            long,
            default_value_t = 7799,
            value_parser = clap::value_parser!(u16).range(1..=65535),
            help = "Port for the local dashboard server"
        )]
        port: u16,
    },
}
