//! Write one ideal camera depth trace and its pictures.
//!
//! The trace is generated here. Nothing is read from a capture and nothing is sent.

use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::process::ExitCode;

use physical::demo_camera;

fn main() -> ExitCode {
    let mut out = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--out" => out = args.next(),
            _ => return usage(),
        }
    }
    let Some(out) = out else {
        return usage();
    };
    let views = match demo_camera() {
        Ok(views) => views,
        Err(err) => {
            eprintln!("camera: {err:?}");
            return ExitCode::from(1);
        }
    };
    if let Err(err) = write(Path::new(&out), &views) {
        eprintln!("{err}");
        return ExitCode::from(1);
    }
    println!("camera depth trace written sent false");
    ExitCode::from(0)
}

fn write(dir: &Path, views: &physical::CameraViews) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    fs::write(dir.join("model.csv"), &views.model_csv)?;
    fs::write(dir.join("trace.csv"), &views.trace_csv)?;
    fs::write(dir.join("fault.csv"), &views.fault_csv)?;
    fs::write(dir.join("side.svg"), &views.side_svg)?;
    fs::write(dir.join("depth-row.svg"), &views.row_svg)?;
    fs::write(dir.join("depth-grid.svg"), &views.grid_svg)?;
    fs::write(dir.join("fault.svg"), &views.fault_svg)?;
    Ok(())
}

fn usage() -> ExitCode {
    eprintln!("usage: camera-depth --out OUT");
    ExitCode::from(2)
}
