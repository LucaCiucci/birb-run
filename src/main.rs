use birb_task::cli::Cli;
use clap::Parser;

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    log::info!("Starting birb task runner");
    birb_task::cli::main(&args, true)
}

