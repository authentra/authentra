use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
pub struct CliCommand {
    #[command(subcommand)]
    pub subcommand: CliSubcommand,
}

#[derive(Subcommand)]
pub enum CliSubcommand {
    LoadTest(LoadTestArgs),
}

#[derive(Args)]
pub struct LoadTestArgs {
    #[arg(short = 'r', long)]
    pub requests: u32,
    #[arg(short = 'm', long)]
    pub max_concurrent: u32,
}
