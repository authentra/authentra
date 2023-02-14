use clap::Parser;
use cli::CliCommand;
use load_test::load_test;

pub mod cli;
pub mod load_test;

#[tokio::main]
async fn main() {
    let cli = CliCommand::parse();
    match cli.subcommand {
        cli::CliSubcommand::LoadTest(args) => load_test(args).await,
    }
}
