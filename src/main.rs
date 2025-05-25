use argh::FromArgs;
use log::{error, info};
use std::{fs, io, path::PathBuf, process};

mod engine;
mod plan;

use engine::TestEngine;
use plan::TestPlan;

#[derive(FromArgs, PartialEq, Debug)]
/// tstit - Test It. REST It.
struct Args {
    #[argh(positional)]
    /// path(s) to testplan TOML files or directories containing testplans
    paths: Vec<PathBuf>,

    #[argh(switch, short = 'v')]
    /// enable verbose output
    verbose: bool,
}

fn main() -> Result<(), io::Error> {
    let args: Args = argh::from_env();

    pretty_env_logger::formatted_builder()
        .filter_level(if args.verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .init();

    if args.paths.is_empty() {
        error!("no testplan paths provided");
        println!("try:  tstit --help");
        process::exit(1);
    }

    println!(
        "{} v{} - {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_DESCRIPTION")
    );

    let mut testplans = Vec::new();
    for path in args.paths {
        collect_testplans(path, &mut testplans)?;
    }

    info!("found {} testplans", testplans.len());
    let mut success_count = 0;
    let mut fail_count = 0;

    for file_path in testplans {
        info!("processing {}...", file_path.display());
        match TestPlan::load(&file_path.to_string_lossy())
            .and_then(|plan| TestEngine::new(plan).execute())
        {
            Ok(_) => {
                info!("testplan succeeded");
                success_count += 1;
            }
            Err(e) => {
                error!("testplan failed: {}", e);
                fail_count += 1;
            }
        }
    }

    info!(
        "test execution completed, success: {}, failed: {}",
        success_count, fail_count
    );
    Ok(())
}

fn collect_testplans(path: PathBuf, testplans: &mut Vec<PathBuf>) -> Result<(), io::Error> {
    if path.is_file() && path.extension().map_or(false, |ext| ext == "toml") {
        testplans.push(path);
    } else if path.is_dir() {
        for entry in fs::read_dir(path)? {
            collect_testplans(entry?.path(), testplans)?;
        }
    }
    Ok(())
}
