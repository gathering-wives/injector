use clap::Parser;
use thiserror::Error;

use tracing::info;
use tracing_subscriber::FmtSubscriber;

mod config;
mod injector;
mod launcher;

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = "./config.toml")]
    config_path: String,
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
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

    if args.verbose {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(tracing::Level::INFO)
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
        let handle = info.hProcess;

        if let Some(deps) = config.dependencies.as_ref() {
            info!("Injecting {} dependencies...", deps.len());

            for dep in deps {
                info!("Injecting \"{}\"...", dep);
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
