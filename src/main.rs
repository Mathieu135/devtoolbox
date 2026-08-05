use clap::Parser;
use devtoolbox::cli::Cli;

fn main() {
    if let Err(error) = devtoolbox::run(Cli::parse()) {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}
