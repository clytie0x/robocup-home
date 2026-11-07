//! Post one captured lidar revolution into the navigation view.
//!
//! The profile flag is required. This binary does not open a port.

use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use physical::{lidar_profiles, load_chunks, write_chunk_post};

fn main() -> ExitCode {
    let mut chunks = None;
    let mut profile_name = None;
    let mut out = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--chunks" => chunks = args.next(),
            "--profile" => profile_name = args.next(),
            "--out" => out = args.next(),
            _ => return usage(),
        }
    }
    let (Some(chunks), Some(profile_name), Some(out)) = (chunks, profile_name, out) else {
        return usage();
    };
    let Some(profile) = lidar_profiles()
        .into_iter()
        .find(|item| item.id == profile_name)
    else {
        eprintln!("profile must be measured-front or configured-front");
        return ExitCode::from(2);
    };
    let bytes = match fs::read(&chunks) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("chunks: {err}");
            return ExitCode::from(1);
        }
    };
    let parsed = match load_chunks(&bytes) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("chunks: {err}");
            return ExitCode::from(1);
        }
    };
    match write_chunk_post(&parsed, &profile, Path::new(&out)) {
        Ok(report) => {
            println!(
                "profile {} clock {:?} view_points {} fresh {:?} sent {}",
                report.profile_id, report.clock, report.view_points, report.fresh, report.sent
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}

fn usage() -> ExitCode {
    eprintln!("post-scan --chunks FILE --profile measured-front|configured-front --out DIR");
    ExitCode::from(2)
}
