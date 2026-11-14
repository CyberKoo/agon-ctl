mod app;
mod cli;
mod output;
mod platform;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();
    if let Err(error) = app::run(cli.command) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
