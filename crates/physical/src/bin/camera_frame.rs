//! Write one synthetic 320 by 240 frame for each camera mount.
//!
//! The frame is generated here. Nothing is read from a capture and nothing is sent.

use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::process::ExitCode;

use physical::{class_ppm, decode_frame, demo_frames, encode_frame, fault_csv, rgb_ppm};

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
    let views = match demo_frames() {
        Ok(views) => views,
        Err(err) => {
            eprintln!("camera-frame: {err:?}");
            return ExitCode::from(1);
        }
    };
    if let Err(err) = write(Path::new(&out), &views) {
        eprintln!("{err}");
        return ExitCode::from(1);
    }
    println!(
        "camera frame written sent false measured_ms={} configured_ms={}",
        views.measured_ms, views.configured_ms
    );
    ExitCode::from(0)
}

fn write(dir: &Path, views: &physical::FrameViews) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    fs::write(dir.join("frame-model.csv"), &views.model_csv)?;
    let measured = encode_frame(&views.measured).expect("measured");
    decode_frame(&measured).expect("measured round trip");
    fs::write(dir.join("measured.rcf"), measured)?;
    fs::write(
        dir.join("configured.rcf"),
        encode_frame(&views.configured).expect("configured"),
    )?;
    fs::write(
        dir.join("faulted.rcf"),
        encode_frame(&views.faulted).expect("faulted"),
    )?;
    fs::write(dir.join("measured-rgb.ppm"), rgb_ppm(&views.measured))?;
    fs::write(dir.join("measured-class.ppm"), class_ppm(&views.measured))?;
    fs::write(dir.join("configured-rgb.ppm"), rgb_ppm(&views.configured))?;
    fs::write(
        dir.join("configured-class.ppm"),
        class_ppm(&views.configured),
    )?;
    fs::write(
        dir.join("fault.csv"),
        fault_csv(&views.faulted, &views.fault),
    )?;
    fs::write(dir.join("class-legend.svg"), &views.legend_svg)?;
    fs::write(dir.join("fault.svg"), &views.fault_svg)?;
    fs::write(
        dir.join("timing.txt"),
        format!(
            "measured_ms {}\nconfigured_ms {}\n",
            views.measured_ms, views.configured_ms
        ),
    )?;
    Ok(())
}

fn usage() -> ExitCode {
    eprintln!("usage: camera-frame --out OUT");
    ExitCode::from(2)
}
