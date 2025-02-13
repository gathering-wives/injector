use clap::Parser;
use thiserror::Error;

use tracing::{debug, info};
use tracing_subscriber::FmtSubscriber;

mod config;
mod injector;
mod launcher;

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = "./config.toml")]
    /// Relative or absolute path to config file.
    config_path: String,
    #[arg(short, long, action = clap::ArgAction::Count)]
    /// Verbosity level. -v for INFO, -vv for DEBUG, -vvv for TRACE.
    verbose: u8,
}

#[derive(Error, Debug)]
enum Error {
    #[error("Config file error")]
    ConfigFailed(#[from] config::Error),
    #[error("Failed to launch process")]
    LaunchFailed(#[from] launcher::Error),
    #[error("Failed to inject dependency")]
    InjectFailed(#[from] injector::Error),
}

fn main() -> Result<(), Error> {
    let args = Args::parse();

    if args.verbose > 0 {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(match args.verbose {
                1 => tracing::Level::INFO,
                2 => tracing::Level::DEBUG,
                _ => tracing::Level::TRACE,
            })
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set default global subscriber");
    }

    info!("Reading config...");
    let config = match config::Config::from_file(&args.config_path) {
        Ok(config) => config,
        Err(config::Error::ReadFailed(err)) if err.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("Config file is not found!");
            std::process::exit(1);
        }
        Err(err) => return Err(err.into()),
    };

    unsafe {
        info!("Launching process...");

        let info = launcher::launch(
            &config.executable_path,
            config.args.as_ref(),
            config.current_directory.as_ref(),
        )?;
        debug!("{:?}", info);

        let handle = info.hProcess;

        if let Some(deps) = config.dependencies.as_ref() {
            info!("Injecting {} dependencies...", deps.len());

            for dep in deps {
                debug!("Injecting \"{}\"...", dep);
                injector::inject(handle, dep).unwrap();
            }
        } else {
            info!("No dependencies to inject.");
        }

        info!("Resuming process...");
        launcher::resume_process(&info);
        launcher::free_info(info);
    }

    info!("Finished.");
    Ok(())
}
