use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut session = None;
    let mut out = None;
    let mut arm = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--session" => session = args.next(),
            "--out" => out = args.next(),
            "--arm-urdf" => arm = args.next(),
            "--help" | "-h" => {
                println!("replay --session DIR --out DIR [--arm-urdf FILE]");
                println!(
                    "Writes a report, CSV, and SVG from a capture directory. Sends no commands."
                );
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("unknown argument {other}");
                return ExitCode::from(2);
            }
        }
    }
    let (Some(session), Some(out)) = (session, out) else {
        eprintln!("replay --session DIR --out DIR [--arm-urdf FILE]");
        return ExitCode::from(2);
    };
    let arm = arm.map(PathBuf::from);
    match physical::write_replay(
        std::path::Path::new(&session),
        std::path::Path::new(&out),
        arm.as_deref(),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}
