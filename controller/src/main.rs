mod config;
mod firefox;

use std::process::ExitCode;

fn main() -> ExitCode {
    let config = match config::Config::from_args(std::env::args().skip(1)) {
        Ok(config) => config,
        Err(message) => {
            eprintln!("{message}");
            eprintln!();
            config::Config::print_usage();
            return ExitCode::from(2);
        }
    };

    if let Err(error) = firefox::launch(&config) {
        eprintln!("moth-controller: {error}");
        return ExitCode::from(1);
    }

    println!(
        "Started PokéIdle {} using profile {}.",
        config.profile_name,
        config.profile_dir.display()
    );

    ExitCode::SUCCESS
}
