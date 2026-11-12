//! Runs the slice-1 Help Me Carry script and writes the replay.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use physical::{run, score_text, success_setup, timeline_csv, trajectory_png};

fn main() -> ExitCode {
    let dir = env::args().nth(1).map_or_else(
        || PathBuf::from("docs/evidence/5.1-help-me-carry"),
        PathBuf::from,
    );
    if let Err(error) = fs::create_dir_all(&dir) {
        eprintln!("help-me-carry: {error}");
        return ExitCode::from(1);
    }
    let run = run(&success_setup());
    let score = score_text(&run);
    if let Err(error) = fs::write(dir.join("timeline.csv"), timeline_csv(&run))
        .and_then(|()| fs::write(dir.join("trajectory.png"), trajectory_png(&run)))
        .and_then(|()| fs::write(dir.join("score.txt"), &score))
    {
        eprintln!("help-me-carry: {error}");
        return ExitCode::from(1);
    }
    print!("{score}");
    ExitCode::from(0)
}
