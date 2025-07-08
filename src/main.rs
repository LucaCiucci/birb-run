use birb_run::cli::Cli;
use clap::Parser;


fn main() {
    env_logger::init();

    let args = Cli::parse();

    birb_run::cli::main(&args);
}

