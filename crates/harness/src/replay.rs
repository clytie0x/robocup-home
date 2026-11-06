//! Std-only SVG replay of one recorded episode.
//!
//! The picture is a 2D simulation. It is not hardware and it is not a score.
//! A sensor mark is drawn only when the recorded sample was accepted as ready.
//! Missing, stale, duplicate, reversed, and future samples stay labels.

/// Which recorded episode to draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayCase {
    Success,
    Loss,
    Delay200,
    Missing,
}

/// One recorded delivery. `fresh` is true only for an accepted ready sample.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mark {
    pub captured_ms: Option<u64>,
    pub delivered_ms: u64,
    pub decision_ms: u64,
    pub age_ms: Option<u64>,
    pub kind: &'static str,
    pub fresh: bool,
    pub sensed_x: Option<i32>,
    pub sensed_y: Option<i32>,
    pub truth_x: i32,
    pub truth_y: i32,
    pub action: &'static str,
    pub custody: &'static str,
}

/// Recorded picture input. Coordinates come from the episode log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    pub seed: u64,
    pub profile: &'static str,
    pub phase: &'static str,
    pub fail: &'static str,
    pub custody: &'static str,
    /// `(ms, robot_x, robot_y, operator_x, operator_y)` from the world trace.
    pub truth: Vec<(u64, i32, i32, i32, i32)>,
    pub marks: Vec<Mark>,
}

pub fn replay_svg(case: ReplayCase) -> String {
    render(&crate::episode::replay_picture(case))
}

pub fn render(picture: &Picture) -> String {
    let map = bounds(picture);
    let mut body = String::new();
    body.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"980\" height=\"760\" viewBox=\"0 0 980 760\">\n");
    body.push_str("<title>2D simulation. Not hardware. Not a score.</title>\n");
    body.push_str("<rect width=\"980\" height=\"760\" fill=\"#f7f4ef\"/>\n");
    text(
        &mut body,
        16,
        28,
        16,
        "#222",
        "2D simulation. Not hardware. Not a score.",
    );
    text(
        &mut body,
        16,
        50,
        13,
        "#222",
        &format!(
            "seed {}  {}  phase {}  fail {}  custody {}",
            picture.seed,
            xml(picture.profile),
            xml(picture.phase),
            xml(picture.fail),
            xml(picture.custody)
        ),
    );
    text(
        &mut body,
        16,
        70,
        12,
        "#333",
        "Legend: black truth operator, blue truth robot, orange posted sensor. A sample that is missing, stale, duplicate, reversed, or future is not a fresh observation.",
    );
    overview(&mut body, picture, &map);
    let frames = frame_indexes(&picture.marks);
    for (slot, index) in frames.iter().enumerate() {
        let col = slot % 3;
        let row = slot / 3;
        let x = 16 + i32::try_from(col).unwrap_or(0) * 320;
        let y = 470 + i32::try_from(row).unwrap_or(0) * 140;
        frame(&mut body, &picture.marks[*index], x, y);
    }
    body.push_str("</svg>\n");
    body
}

fn overview(body: &mut String, picture: &Picture, map: &Bounds) {
    body.push_str("<g id=\"overview\">\n");
    body.push_str(
        "<rect x=\"16\" y=\"84\" width=\"948\" height=\"370\" fill=\"#fff\" stroke=\"#ccc\"/>\n",
    );
    if picture.truth.len() >= 2 {
        let mut robot = String::from(
            "<polyline class=\"truth-robot\" fill=\"none\" stroke=\"#1d4e89\" stroke-width=\"2\" points=\"",
        );
        let mut operator = String::from(
            "<polyline class=\"truth-operator\" fill=\"none\" stroke=\"#111\" stroke-width=\"2\" points=\"",
        );
        for point in &picture.truth {
            robot.push_str(&format!("{},{} ", px(map, point.1), py(map, point.2)));
            operator.push_str(&format!("{},{} ", px(map, point.3), py(map, point.4)));
        }
        robot.push_str("\"/>\n");
        operator.push_str("\"/>\n");
        body.push_str(&robot);
        body.push_str(&operator);
    }
    for mark in &picture.marks {
        if mark.fresh
            && let (Some(x), Some(y)) = (mark.sensed_x, mark.sensed_y)
        {
            body.push_str(&format!(
                "<circle class=\"sensor\" data-fresh=\"1\" data-kind=\"ready\" data-captured=\"{}\" data-delivered=\"{}\" data-decision=\"{}\" data-sensed=\"{},{}\" data-truth=\"{},{}\" cx=\"{}\" cy=\"{}\" r=\"3\" fill=\"none\" stroke=\"#e07a3d\" stroke-width=\"1.5\"/>\n",
                opt_ms(mark.captured_ms),
                mark.delivered_ms,
                mark.decision_ms,
                x,
                y,
                mark.truth_x,
                mark.truth_y,
                px(map, x),
                py(map, y)
            ));
        }
    }
    body.push_str("</g>\n");
}

fn frame(body: &mut String, mark: &Mark, x: i32, y: i32) {
    body.push_str(&format!(
        "<g data-kind=\"{}\" data-fresh=\"{}\" data-captured=\"{}\" data-delivered=\"{}\" data-decision=\"{}\" data-age=\"{}\" data-action=\"{}\">\n",
        xml(mark.kind),
        if mark.fresh { "1" } else { "0" },
        opt_ms(mark.captured_ms),
        mark.delivered_ms,
        mark.decision_ms,
        opt_ms(mark.age_ms),
        xml(mark.action)
    ));
    body.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"300\" height=\"128\" fill=\"#fff\" stroke=\"#ddd\"/>\n"
    ));
    text(
        body,
        x + 8,
        y + 18,
        12,
        "#222",
        &format!(
            "{}  capture {}  delivery {}  decision {}",
            mark.kind,
            opt_ms(mark.captured_ms),
            mark.delivered_ms,
            mark.decision_ms
        ),
    );
    text(
        body,
        x + 8,
        y + 36,
        12,
        "#222",
        &format!("action {}  custody {}", mark.action, mark.custody),
    );
    if mark.fresh
        && let (Some(sx), Some(sy)) = (mark.sensed_x, mark.sensed_y)
    {
        text(
            body,
            x + 8,
            y + 56,
            12,
            "#e07a3d",
            &format!("posted sensor {sx},{sy}"),
        );
        text(
            body,
            x + 8,
            y + 74,
            12,
            "#111",
            &format!("truth operator {},{}", mark.truth_x, mark.truth_y),
        );
    } else {
        text(
            body,
            x + 8,
            y + 56,
            12,
            "#666",
            &format!("{}. Not a fresh observation.", mark.kind),
        );
        text(
            body,
            x + 8,
            y + 74,
            12,
            "#111",
            &format!("truth operator {},{}", mark.truth_x, mark.truth_y),
        );
    }
    body.push_str("</g>\n");
}

fn frame_indexes(marks: &[Mark]) -> Vec<usize> {
    if marks.is_empty() {
        return Vec::new();
    }
    let mut ids = vec![0, marks.len() / 2, marks.len() - 1];
    if let Some(index) = marks.iter().position(|mark| !mark.fresh) {
        ids.push(index);
    }
    if let Some(index) = marks.iter().position(|mark| {
        mark.fresh
            && mark
                .sensed_x
                .zip(mark.sensed_y)
                .is_some_and(|(x, y)| x != mark.truth_x || y != mark.truth_y)
    }) {
        ids.push(index);
    }
    ids.sort_unstable();
    ids.dedup();
    ids.truncate(6);
    ids
}

struct Bounds {
    min_x: i32,
    min_y: i32,
    span_x: i32,
    span_y: i32,
}

fn bounds(picture: &Picture) -> Bounds {
    let mut min_x = 0;
    let mut max_x = 1;
    let mut min_y = 0;
    let mut max_y = 1;
    let mut started = false;
    let mut take = |x: i32, y: i32| {
        if !started {
            min_x = x;
            max_x = x;
            min_y = y;
            max_y = y;
            started = true;
            return;
        }
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    };
    for point in &picture.truth {
        take(point.1, point.2);
        take(point.3, point.4);
    }
    for mark in &picture.marks {
        take(mark.truth_x, mark.truth_y);
        if mark.fresh
            && let (Some(x), Some(y)) = (mark.sensed_x, mark.sensed_y)
        {
            take(x, y);
        }
    }
    min_x -= 400;
    min_y -= 400;
    max_x += 400;
    max_y += 400;
    Bounds {
        min_x,
        min_y,
        span_x: (max_x - min_x).max(1),
        span_y: (max_y - min_y).max(1),
    }
}

fn px(map: &Bounds, x: i32) -> i32 {
    24 + (x - map.min_x) * 920 / map.span_x
}

fn py(map: &Bounds, y: i32) -> i32 {
    440 - (y - map.min_y) * 340 / map.span_y
}

fn text(body: &mut String, x: i32, y: i32, size: i32, fill: &str, value: &str) {
    body.push_str(&format!(
        "<text x=\"{x}\" y=\"{y}\" font-size=\"{size}\" font-family=\"sans-serif\" fill=\"{fill}\">{}</text>\n",
        xml(value)
    ));
}

fn opt_ms(value: Option<u64>) -> String {
    value.map(|ms| ms.to_string()).unwrap_or_default()
}

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha256(bytes: &[u8]) -> String {
        let mut child = std::process::Command::new("sha256sum")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("sha256sum");
        use std::io::Write;
        child.stdin.take().unwrap().write_all(bytes).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        text.split_whitespace().next().unwrap().to_string()
    }

    fn write_artifact(name: &str, svg: &str) -> String {
        let dir = std::path::Path::new("/opt/cursor/artifacts");
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, svg).unwrap();
        sha256(svg.as_bytes())
    }

    #[test]
    fn a_withheld_sample_is_not_drawn_as_a_fresh_sensor() {
        let picture = Picture {
            seed: 1,
            profile: "stale",
            phase: "following",
            fail: "-",
            custody: "gripper",
            truth: vec![(1_000, 0, 0, 10, 20)],
            marks: vec![Mark {
                captured_ms: Some(0),
                delivered_ms: 0,
                decision_ms: 1_001,
                age_ms: Some(1_001),
                kind: "stale",
                fresh: false,
                sensed_x: Some(9),
                sensed_y: Some(9),
                truth_x: 10,
                truth_y: 20,
                action: "need-fresh",
                custody: "gripper",
            }],
        };
        let svg = render(&picture);
        assert!(svg.contains("data-kind=\"stale\""));
        assert!(svg.contains("data-fresh=\"0\""));
        assert!(svg.contains("Not a fresh observation."));
        assert!(!svg.contains("data-sensed=\"9,9\""));
        assert!(!svg.contains("class=\"sensor\""));
    }

    #[test]
    fn recorded_replays_are_deterministic_and_not_the_same_picture() {
        let success = replay_svg(ReplayCase::Success);
        let again = replay_svg(ReplayCase::Success);
        assert_eq!(success, again);
        assert!(success.contains("2D simulation"));
        assert!(success.contains("Not hardware"));
        assert!(success.contains("Not a score"));
        assert!(success.starts_with("<svg"));
        assert!(success.contains("</svg>"));
        assert!(success.contains("phase complete"));
        assert!(success.contains("custody operator"));
        assert!(success.contains("data-kind=\"ready\""));
        assert!(success.contains("data-fresh=\"1\""));

        let loss = replay_svg(ReplayCase::Loss);
        assert_ne!(loss, success);
        assert!(loss.contains("phase lost"));
        assert!(loss.contains("custody gripper"));
        assert!(loss.contains("fail lost"));

        let delayed = replay_svg(ReplayCase::Delay200);
        assert_ne!(delayed, success);
        assert!(delayed.contains("data-kind=\"ready\""));
        assert!(
            delayed_layers_diverge(&delayed),
            "delay picture did not show a posted point away from truth"
        );

        let missing = replay_svg(ReplayCase::Missing);
        assert!(missing.contains("data-kind=\"missing\""));
        assert!(missing.contains("data-fresh=\"0\""));
        let missing_group = missing.split("data-kind=\"missing\"").nth(1).unwrap();
        let group = missing_group.split("</g>").next().unwrap();
        assert!(group.contains("Not a fresh observation."));
        assert!(!group.contains("class=\"sensor\""));

        let success_hash = write_artifact("r5-success-seed1.svg", &success);
        let loss_hash = write_artifact("r5-loss-seed1.svg", &loss);
        let delay_hash = write_artifact("r5-delay200-seed1.svg", &delayed);
        println!("SHA256-SHA256SUM success {success_hash}");
        println!("SHA256-SHA256SUM loss {loss_hash}");
        println!("SHA256-SHA256SUM delay200 {delay_hash}");
        assert_ne!(success_hash, loss_hash);
        assert_ne!(success_hash, delay_hash);
        assert_eq!(success_hash, sha256(again.as_bytes()));
    }

    fn delayed_layers_diverge(svg: &str) -> bool {
        svg.split("data-kind=\"ready\"").skip(1).any(|chunk| {
            let chunk = chunk.split("/>").next().unwrap_or("");
            let sensed = attr(chunk, "data-sensed");
            let truth = attr(chunk, "data-truth");
            sensed.is_some() && truth.is_some() && sensed != truth
        })
    }

    fn attr<'a>(chunk: &'a str, name: &str) -> Option<&'a str> {
        let key = format!("{name}=\"");
        let rest = chunk.split(&key).nth(1)?;
        Some(rest.split('"').next().unwrap_or(""))
    }
}
