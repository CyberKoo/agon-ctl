use agon_osd::Feature;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "agon-ctl",
    version,
    about = "Native OSD control for AGON displays",
    arg_required_else_help = true,
    after_help = "Values accept decimal, 0x-prefixed hexadecimal, or names listed by 'features'."
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    #[command(about = "List DDC-capable external displays")]
    List,
    #[command(about = "List known OSD features and accepted named values")]
    Features,
    #[command(about = "Read current/max values and device-advertised capability lists")]
    Capabilities {
        #[arg(short, long, value_name = "ID|NAME")]
        display: Option<String>,
    },
    #[command(about = "Read an OSD feature")]
    Get {
        #[arg(value_parser = parse_feature)]
        feature: Feature,
        #[arg(short, long, value_name = "ID|NAME")]
        display: Option<String>,
    },
    #[command(about = "Set an OSD feature and read back the result")]
    Set {
        #[arg(value_parser = parse_feature)]
        feature: Feature,
        value: String,
        #[arg(short, long, value_name = "ID|NAME")]
        display: Option<String>,
    },
}

fn parse_feature(input: &str) -> Result<Feature, String> {
    Feature::from_name(input).ok_or_else(|| {
        format!("unknown feature '{input}'; run 'agon-ctl features' for the supported names")
    })
}
