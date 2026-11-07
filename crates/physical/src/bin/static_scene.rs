//! Write one synthetic static scene, its lidar packets, and the posted view.
//!
//! The packets are generated here. Nothing is read from a capture and nothing is sent.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use navigation::SampleHistory;
use physical::{
    ClockLabel, PostClock, PostOutcome, SampleUse, demonstration, frame_sample, lidar_profiles,
    packet_revolution, parse_lidar, post_samples, render_views,
};

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
    let scene = demonstration();
    let profile = lidar_profiles()
        .into_iter()
        .find(|item| item.id == "measured-front")
        .expect("measured-front");
    let packet = match packet_revolution(&scene, profile.front_raw_deg, 2.0) {
        Ok(packet) => packet,
        Err(err) => {
            eprintln!("scene: {err:?}");
            return ExitCode::from(1);
        }
    };
    let parsed = parse_lidar(&packet.bytes);
    if !parsed.rejects.is_empty() || parsed.samples.len() != packet.bearings_deg.len() {
        eprintln!("packet parse did not recover the cast samples");
        return ExitCode::from(1);
    }
    let views = match render_views(&scene, &packet.bearings_deg, &profile, 2_500) {
        Ok(views) => views,
        Err(err) => {
            eprintln!("views: {err:?}");
            return ExitCode::from(1);
        }
    };
    let clock = PostClock {
        captured_ms: Some(2_500),
        delivered_ms: 2_500,
        decision_ms: 2_500,
        label: ClockLabel::InjectedReplay,
    };
    let (_history, posted) = post_samples(&profile, &parsed.samples, SampleHistory::new(), clock);
    if let Err(err) = write_all(
        Path::new(&out),
        &views,
        &packet.bytes,
        &parsed,
        &profile,
        &posted,
    ) {
        eprintln!("{err}");
        return ExitCode::from(1);
    }
    let hits = parsed
        .samples
        .iter()
        .filter(|sample| sample.range_m > 0.0)
        .count();
    println!(
        "profile {} clock injected-replay samples {} hits {} view_points {} fresh {} sent {}",
        profile.id,
        parsed.samples.len(),
        hits,
        posted.view_points.len(),
        outcome(&posted.outcome),
        posted.sent,
    );
    ExitCode::from(0)
}

fn write_all(
    out: &Path,
    views: &physical::SceneViews,
    packets: &[u8],
    parsed: &physical::LidarParse,
    profile: &physical::LidarProfile,
    posted: &physical::ScanPost,
) -> Result<(), String> {
    fs::create_dir_all(out).map_err(|err| err.to_string())?;
    fs::write(out.join("mount.csv"), &views.mount_csv).map_err(|err| err.to_string())?;
    fs::write(out.join("rays.csv"), &views.rays_csv).map_err(|err| err.to_string())?;
    fs::write(out.join("oblique.svg"), &views.oblique_svg).map_err(|err| err.to_string())?;
    fs::write(out.join("top.svg"), &views.top_svg).map_err(|err| err.to_string())?;
    fs::write(out.join("side.svg"), &views.side_svg).map_err(|err| err.to_string())?;
    fs::write(out.join("scan.svg"), &views.scan_svg).map_err(|err| err.to_string())?;
    fs::write(out.join("packets.bin"), packets).map_err(|err| err.to_string())?;
    let summary = format!(
        "field,value\nclock,injected-replay\ncaptured_ms,2500\nprofile,{}\npackets_ok,{}\nsamples,{}\nview_points,{}\npeople,{}\nfresh,{}\nsent,false\ncommand,\nclear_field,false\n",
        profile.id,
        parsed.packets_ok,
        parsed.samples.len(),
        posted.view_points.len(),
        posted.people,
        outcome(&posted.outcome),
    );
    fs::write(out.join("posted.csv"), summary).map_err(|err| err.to_string())?;
    let mut rows = String::from("raw_deg,packet_range_m,sample_use,x_mm,y_mm\n");
    for sample in &parsed.samples {
        match frame_sample(profile, sample.raw_deg, sample.range_m) {
            SampleUse::Body { x_mm, y_mm } => {
                let _ = writeln!(
                    rows,
                    "{:.4},{:.4},body,{x_mm},{y_mm}",
                    sample.raw_deg, sample.range_m
                );
            }
            other => {
                let _ = writeln!(
                    rows,
                    "{:.4},{:.4},{other:?},,",
                    sample.raw_deg, sample.range_m
                );
            }
        }
    }
    fs::write(out.join("packet-points.csv"), rows).map_err(|err| err.to_string())?;
    Ok(())
}

fn outcome(outcome: &PostOutcome) -> String {
    match outcome {
        PostOutcome::Acted { intent } => format!("acted:{intent:?}"),
        PostOutcome::Unusable { kind } => format!("unusable:{kind:?}"),
    }
}

fn usage() -> ExitCode {
    eprintln!("usage: static-scene --out DIR");
    ExitCode::from(2)
}
