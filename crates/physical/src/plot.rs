//! SVG charts. Every title names the dataset, the unit, and the kind of number.

pub struct XySeries {
    pub name: String,
    pub kind: &'static str,
    pub color: &'static str,
    pub points: Vec<(f64, f64)>,
}

pub struct Bar {
    pub label: String,
    pub value: f64,
    pub whisker: f64,
    pub color: &'static str,
    pub kind: &'static str,
}

const CANVAS_W: f64 = 1040.0;
const WRAP: usize = 90;

pub fn xy_plot(
    title: &str,
    footer: &str,
    x_label: &str,
    y_label: &str,
    series: &[XySeries],
) -> String {
    let points = finite_points(series);
    let (min_x, max_x, min_y, max_y) = padded_bounds(&points, false);
    let left = 78.0;
    let width = CANVAS_W - left - 36.0;
    let height = 340.0;
    let (head, plot_top) = heading(title, &format!("x: {x_label} | y: {y_label}"));
    let top = plot_top;
    let bottom = top + height;
    let map = BoxMap {
        left,
        top,
        width,
        height,
        min_x,
        min_y,
        span_x: max_x - min_x,
        span_y: max_y - min_y,
    };
    let mut body = head;
    body.push_str(&axis_line("y", left, top, left, bottom));
    body.push_str(&axis_line("x", left, bottom, left + width, bottom));
    body.push_str(&edge_ticks(&map));
    for item in series {
        body.push_str(&polyline(&item.points, &map, item.color));
    }
    let legend_top = bottom + 28.0;
    let (legend, after_legend) = legend_rows(series_swatches(series), legend_top);
    body.push_str(&legend);
    let (caption, after_caption) =
        wrapped(footer, 24.0, after_legend + 8.0, 12, "data-caption=\"1\"");
    body.push_str(&caption);
    document(CANVAS_W as u32, after_caption + 16.0, &body)
}

/// Plan view in metres. The axis cross is the robot origin, not the center of the points.
pub fn robot_plan(title: &str, footer: &str, series: &[XySeries]) -> String {
    let points = finite_points(series);
    let (min_x, max_x, min_y, max_y) = padded_bounds(&points, true);
    let span = (max_x - min_x).max(max_y - min_y).max(1e-6);
    let cx = (min_x + max_x) / 2.0;
    let cy = (min_y + max_y) / 2.0;
    let min_x = cx - span / 2.0;
    let min_y = cy - span / 2.0;
    let left = 78.0;
    let size = 620.0;
    let (head, plot_top) = heading(
        title,
        "equal scale, metres. The cross is the robot origin (0 m, 0 m), not the center of the points.",
    );
    let map = BoxMap {
        left,
        top: plot_top,
        width: size,
        height: size,
        min_x,
        min_y,
        span_x: span,
        span_y: span,
    };
    let mut body = head;
    let origin_x = map.x(0.0);
    let origin_y = map.y(0.0);
    body.push_str(&axis_line("x", left, origin_y, left + size, origin_y));
    body.push_str(&axis_line(
        "y",
        origin_x,
        plot_top,
        origin_x,
        plot_top + size,
    ));
    for item in series {
        body.push_str(&polyline(&item.points, &map, item.color));
    }
    body.push_str(&format!(
        "<circle data-origin=\"robot\" cx=\"{origin_x:.1}\" cy=\"{origin_y:.1}\" r=\"3\" fill=\"none\" stroke=\"#222222\"/>"
    ));
    body.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.0}\" font-size=\"11\">robot origin is the axis cross, 0 m</text>",
        left + 8.0,
        plot_top + 16.0
    ));
    body.push_str(&origin_ticks(&map));
    let legend_top = plot_top + size + 36.0;
    let (legend, after_legend) = legend_rows(series_swatches(series), legend_top);
    body.push_str(&legend);
    let (caption, after_caption) =
        wrapped(footer, 24.0, after_legend + 8.0, 12, "data-caption=\"1\"");
    body.push_str(&caption);
    document(CANVAS_W as u32, after_caption + 16.0, &body)
}

pub fn bar_groups(title: &str, footer: &str, y_label: &str, groups: &[Vec<Bar>]) -> String {
    let mut peak = 0.1_f64;
    let mut labels = Vec::new();
    for group in groups {
        for bar in group {
            if bar.value.is_finite() {
                peak = peak.max(bar.value.abs() + bar.whisker.abs());
            }
            if labels.last() != Some(&bar.label.as_str()) {
                labels.push(bar.label.as_str());
            }
        }
    }
    let (head, plot_top) = heading(title, &format!("y: {y_label}"));
    let left = 78.0;
    let width = CANVAS_W - left - 160.0;
    let height = 300.0;
    let baseline = plot_top + height;
    let mut body = head;
    body.push_str(&axis_line("y", left, plot_top, left, baseline));
    body.push_str(&axis_line("x", left, baseline, left + width, baseline));
    for tick in ticks(0.0, peak) {
        let y = baseline - tick / peak * height;
        body.push_str(&format!(
            "<text data-axis-tick=\"y\" x=\"70\" y=\"{y:.0}\" font-size=\"11\" text-anchor=\"end\">{}</text>",
            esc(&tick_label(tick))
        ));
    }
    let slot = if groups.is_empty() {
        width
    } else {
        width / groups.len() as f64
    };
    for (group_index, group) in groups.iter().enumerate() {
        let count = group.len().max(1) as f64;
        let bar_w = (slot * 0.7 / count).clamp(4.0, 28.0);
        for (bar_index, bar) in group.iter().enumerate() {
            if !bar.value.is_finite() {
                continue;
            }
            let x =
                left + slot * group_index as f64 + slot * 0.15 + bar_index as f64 * (bar_w + 3.0);
            let h = bar.value.abs() / peak * height;
            let y = if bar.value >= 0.0 {
                baseline - h
            } else {
                baseline
            };
            body.push_str(&format!(
                "<rect x=\"{x:.1}\" y=\"{y:.1}\" width=\"{bar_w:.1}\" height=\"{h:.1}\" fill=\"{}\"/>",
                bar.color
            ));
            if bar.whisker > 0.0 {
                let mid = x + bar_w / 2.0;
                let up = baseline - (bar.value + bar.whisker).max(0.0) / peak * height;
                let down = baseline - (bar.value - bar.whisker).max(0.0) / peak * height;
                body.push_str(&plain_line(mid, up, mid, down, "#222222"));
            }
            if bar_index == 0 {
                let label_y = baseline + 14.0;
                body.push_str(&format!(
                    "<text x=\"{x:.1}\" y=\"{label_y:.0}\" font-size=\"11\" transform=\"rotate(40 {x:.1} {label_y:.0})\">{}</text>",
                    esc(&bar.label)
                ));
            }
        }
    }
    let drop = label_drop(&labels);
    let (legend, after_legend) = legend_rows(bar_swatches(groups), baseline + drop);
    body.push_str(&legend);
    let (caption, after_caption) =
        wrapped(footer, 24.0, after_legend + 8.0, 12, "data-caption=\"1\"");
    body.push_str(&caption);
    document(CANVAS_W as u32, after_caption + 16.0, &body)
}

pub fn frame_schematic(title: &str, footer: &str, notes: &[String]) -> String {
    let (mut body, mut y) = heading(title, "text schematic, not a photograph and not a mesh");
    for note in notes {
        let (block, next) = wrapped(note, 24.0, y, 14, "");
        body.push_str(&block);
        y = next;
    }
    let (caption, after) = wrapped(footer, 24.0, y + 6.0, 12, "data-caption=\"1\"");
    body.push_str(&caption);
    document(CANVAS_W as u32, after + 16.0, &body)
}

fn finite_points(series: &[XySeries]) -> Vec<(f64, f64)> {
    let mut points = Vec::new();
    for item in series {
        for point in &item.points {
            if point.0.is_finite() && point.1.is_finite() {
                points.push(*point);
            }
        }
    }
    points
}

fn padded_bounds(points: &[(f64, f64)], include_origin: bool) -> (f64, f64, f64, f64) {
    let mut min_x = if include_origin { 0.0 } else { f64::MAX };
    let mut max_x = if include_origin { 0.0 } else { f64::MIN };
    let mut min_y = if include_origin { 0.0 } else { f64::MAX };
    let mut max_y = if include_origin { 0.0 } else { f64::MIN };
    for (x, y) in points {
        min_x = min_x.min(*x);
        max_x = max_x.max(*x);
        min_y = min_y.min(*y);
        max_y = max_y.max(*y);
    }
    if !min_x.is_finite() || !max_x.is_finite() {
        return (-1.0, 1.0, -1.0, 1.0);
    }
    if max_x - min_x < 1e-9 {
        min_x -= 0.5;
        max_x += 0.5;
    }
    if max_y - min_y < 1e-9 {
        min_y -= 0.5;
        max_y += 0.5;
    }
    let pad_x = (max_x - min_x) * 0.08;
    let pad_y = (max_y - min_y) * 0.08;
    (min_x - pad_x, max_x + pad_x, min_y - pad_y, max_y + pad_y)
}

fn heading(title: &str, subtitle: &str) -> (String, f64) {
    let (title_svg, after_title) = wrapped(title, 24.0, 22.0, 15, "");
    let (sub_svg, after_sub) = wrapped(subtitle, 24.0, after_title, 12, "");
    (format!("{title_svg}{sub_svg}"), after_sub + 10.0)
}

fn wrapped(text: &str, x: f64, mut y: f64, font: u32, first_attrs: &str) -> (String, f64) {
    let mut svg = String::new();
    for (index, line) in wrap(text, WRAP).iter().enumerate() {
        let attrs = if index == 0 { first_attrs } else { "" };
        svg.push_str(&format!(
            "<text {attrs} x=\"{x:.0}\" y=\"{y:.0}\" font-size=\"{font}\">{}</text>",
            esc(line)
        ));
        y += f64::from(font) + 6.0;
    }
    (svg, y)
}

fn wrap(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if word.chars().count() > max_chars {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            let mut rest = word;
            while rest.chars().count() > max_chars {
                let (head, tail) = split_chars(rest, max_chars);
                lines.push(head);
                rest = tail;
            }
            current = rest.to_string();
            continue;
        }
        if current.is_empty() {
            current = word.to_string();
        } else if current.chars().count() + 1 + word.chars().count() <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn split_chars(text: &str, count: usize) -> (String, &str) {
    let mut end = 0;
    for (index, (byte, _)) in text.char_indices().enumerate() {
        if index == count {
            end = byte;
            break;
        }
    }
    if end == 0 {
        (text.to_string(), "")
    } else {
        (text[..end].to_string(), &text[end..])
    }
}

struct BoxMap {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    min_x: f64,
    min_y: f64,
    span_x: f64,
    span_y: f64,
}

impl BoxMap {
    fn x(&self, value: f64) -> f64 {
        self.left + (value - self.min_x) / self.span_x * self.width
    }

    fn y(&self, value: f64) -> f64 {
        self.top + self.height - (value - self.min_y) / self.span_y * self.height
    }
}

fn edge_ticks(map: &BoxMap) -> String {
    let mut body = String::new();
    let bottom = map.top + map.height;
    for tick in ticks(map.min_x, map.min_x + map.span_x) {
        let x = map.x(tick);
        body.push_str(&format!(
            "<text data-axis-tick=\"x\" x=\"{x:.1}\" y=\"{:.0}\" font-size=\"11\" text-anchor=\"middle\">{}</text>",
            bottom + 14.0,
            esc(&tick_label(tick))
        ));
    }
    for tick in ticks(map.min_y, map.min_y + map.span_y) {
        let y = map.y(tick);
        body.push_str(&format!(
            "<text data-axis-tick=\"y\" x=\"70\" y=\"{y:.0}\" font-size=\"11\" text-anchor=\"end\">{}</text>",
            esc(&tick_label(tick))
        ));
    }
    body
}

fn origin_ticks(map: &BoxMap) -> String {
    let mut body = String::new();
    for tick in ticks(map.min_x, map.min_x + map.span_x) {
        let x = map.x(tick);
        let y = map.top + map.height + 16.0;
        body.push_str(&format!(
            "<text data-axis-tick=\"x\" x=\"{x:.1}\" y=\"{y:.0}\" font-size=\"11\" text-anchor=\"middle\">{}</text>",
            esc(&tick_label(tick))
        ));
    }
    for tick in ticks(map.min_y, map.min_y + map.span_y) {
        if tick.abs() < 1e-9 {
            continue;
        }
        let x = map.left - 8.0;
        let y = map.y(tick);
        body.push_str(&format!(
            "<text data-axis-tick=\"y\" x=\"{x:.1}\" y=\"{y:.0}\" font-size=\"11\" text-anchor=\"end\">{}</text>",
            esc(&tick_label(tick))
        ));
    }
    body
}

fn polyline(points: &[(f64, f64)], map: &BoxMap, color: &str) -> String {
    let mut path = String::new();
    for (x, y) in points {
        if !x.is_finite() || !y.is_finite() {
            continue;
        }
        let px = map.x(*x);
        let py = map.y(*y);
        if path.is_empty() {
            path.push_str(&format!("M {px:.1} {py:.1}"));
        } else {
            path.push_str(&format!(" L {px:.1} {py:.1}"));
        }
    }
    if path.is_empty() {
        String::new()
    } else {
        format!("<path d=\"{path}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"1.2\"/>")
    }
}

fn ticks(min: f64, max: f64) -> Vec<f64> {
    let span = (max - min).abs().max(1e-9);
    let step = nice_step(span / 4.0);
    let mut value = (min / step).ceil() * step;
    if value < min {
        value += step;
    }
    let mut out = Vec::new();
    let mut guard = 0;
    while value <= max + step * 1e-6 && guard < 8 {
        if value >= min - step * 1e-6 {
            out.push(if value.abs() < step * 1e-6 {
                0.0
            } else {
                value
            });
        }
        value += step;
        guard += 1;
    }
    if out.is_empty() {
        out.push(0.0);
    }
    out
}

fn nice_step(raw: f64) -> f64 {
    if !(raw.is_finite() && raw > 0.0) {
        return 1.0;
    }
    let pow = 10.0_f64.powf(raw.log10().floor());
    let err = raw / pow;
    let nice = if err <= 1.0 {
        1.0
    } else if err <= 2.0 {
        2.0
    } else if err <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * pow
}

fn tick_label(value: f64) -> String {
    let value = if value.abs() < 1e-9 { 0.0 } else { value };
    if value.abs() >= 10.0 {
        format!("{value:.0}")
    } else if value.abs() >= 1.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}

fn series_swatches(series: &[XySeries]) -> Vec<(String, &str)> {
    series
        .iter()
        .map(|item| (format!("{} ({})", item.name, item.kind), item.color))
        .collect()
}

fn bar_swatches(groups: &[Vec<Bar>]) -> Vec<(String, &str)> {
    let mut rows = Vec::new();
    for group in groups {
        for bar in group {
            if !rows.iter().any(|(kind, _)| kind == bar.kind) {
                rows.push((bar.kind.to_string(), bar.color));
            }
        }
    }
    rows
}

fn legend_rows(rows: Vec<(String, &str)>, mut y: f64) -> (String, f64) {
    let mut svg = String::new();
    for (kind, color) in rows {
        svg.push_str(&format!(
            "<rect data-legend=\"swatch\" x=\"24\" y=\"{:.0}\" width=\"14\" height=\"10\" fill=\"{color}\"/>",
            y - 9.0
        ));
        svg.push_str(&format!(
            "<text data-legend=\"label\" x=\"44\" y=\"{y:.0}\" font-size=\"12\">{}</text>",
            esc(&kind)
        ));
        y += 16.0;
    }
    (svg, y)
}

fn label_drop(labels: &[&str]) -> f64 {
    let longest = labels
        .iter()
        .map(|label| label.chars().count())
        .max()
        .unwrap_or(0);
    let sweep = longest as f64 * 6.4 * 0.65;
    sweep.clamp(48.0, 150.0) + 20.0
}

fn axis_line(name: &str, x1: f64, y1: f64, x2: f64, y2: f64) -> String {
    format!(
        "<line data-axis=\"{name}\" x1=\"{x1:.1}\" y1=\"{y1:.1}\" x2=\"{x2:.1}\" y2=\"{y2:.1}\" stroke=\"#444444\"/>"
    )
}

fn plain_line(x1: f64, y1: f64, x2: f64, y2: f64, color: &str) -> String {
    format!(
        "<line x1=\"{x1:.1}\" y1=\"{y1:.1}\" x2=\"{x2:.1}\" y2=\"{y2:.1}\" stroke=\"{color}\"/>"
    )
}

fn document(width: u32, height: f64, body: &str) -> String {
    let height = height.ceil().max(1.0) as u32;
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\
         <rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>{body}</svg>\n"
    )
}

fn esc(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::{Bar, XySeries, bar_groups, robot_plan, xy_plot};

    fn series(points: Vec<(f64, f64)>) -> XySeries {
        XySeries {
            name: "sample".to_string(),
            kind: "measured-front",
            color: "#c0392b",
            points,
        }
    }

    #[test]
    fn the_robot_origin_is_not_the_data_bounds_center() {
        let svg = robot_plan(
            "plan",
            "short caption",
            &[series(vec![(0.0, 0.0), (4.0, 0.0)])],
        );
        assert!(svg.contains("data-origin=\"robot\""), "{svg}");
        assert!(svg.contains("data-axis-tick"), "{svg}");
        let origin = attr_pair(&svg, "data-origin=\"robot\"", "cx", "cy");
        let start = path_point(&svg, 0);
        assert!((origin.0 - start.0).abs() < 1.5, "{origin:?} {start:?}");
        assert!((origin.1 - start.1).abs() < 1.5, "{origin:?} {start:?}");
        let far = path_point(&svg, 1);
        assert!(far.0 > start.0 + 20.0, "{start:?} {far:?}");
        let data_mid = (start.0 + far.0) / 2.0;
        assert!(
            (origin.0 - data_mid).abs() > 20.0,
            "{origin:?} mid {data_mid}"
        );
    }

    #[test]
    fn negative_asymmetric_and_zero_extent_points_stay_finite() {
        let negative = robot_plan("plan", "caption", &[series(vec![(-3.0, 1.0)])]);
        assert!(
            !negative.contains("NaN") && !negative.contains("inf"),
            "{negative}"
        );
        let origin = attr_pair(&negative, "data-origin=\"robot\"", "cx", "cy");
        let point = path_point(&negative, 0);
        assert!(
            point.0 < origin.0 - 5.0,
            "negative x is left of the origin {point:?} {origin:?}"
        );
        assert!(
            point.1 < origin.1 - 5.0,
            "positive y is above the origin {point:?} {origin:?}"
        );

        let one = robot_plan("plan", "caption", &[series(vec![(3.0, -4.0)])]);
        assert!(!one.contains("NaN"));
        let origin = attr_pair(&one, "data-origin=\"robot\"", "cx", "cy");
        let point = path_point(&one, 0);
        let distance = (point.0 - origin.0).hypot(point.1 - origin.1);
        assert!(distance > 10.0, "{distance}");

        let zero = robot_plan("plan", "caption", &[series(vec![(0.0, 0.0)])]);
        assert!(!zero.contains("NaN"));
        let origin = attr_pair(&zero, "data-origin=\"robot\"", "cx", "cy");
        let point = path_point(&zero, 0);
        assert!((point.0 - origin.0).abs() < 1.5);
        assert!((point.1 - origin.1).abs() < 1.5);
    }

    #[test]
    fn a_long_caption_is_wrapped_and_axes_stop_above_it() {
        let footer = "word ".repeat(40);
        let svg = xy_plot(
            "title",
            &footer,
            "range metres",
            "lateral metres",
            &[series(vec![(1.0, 0.1), (4.0, 0.4)])],
        );
        assert!(svg.contains("data-axis-tick"), "{svg}");
        assert!(svg.contains("data-legend=\"swatch\""), "{svg}");
        let pieces = text_payloads(&svg);
        assert!(
            pieces.iter().all(|piece| piece.chars().count() <= 100),
            "{pieces:?}"
        );
        let axis_bottom = attr_f64(&svg, "data-axis=\"y\"", "y2");
        let caption_y = attr_f64(&svg, "data-caption=\"1\"", "y");
        assert!(
            caption_y > axis_bottom + 8.0,
            "caption {caption_y} axis {axis_bottom}"
        );
    }

    #[test]
    fn bar_legends_use_swatches_and_numeric_ticks() {
        let svg = bar_groups(
            "Depth validity by file. Dataset: a long title that used to run off the canvas before the gate was named in full.",
            "Every pixel is retained. No-return and below-gate are different classes and this caption used to be clipped.",
            "fraction of pixels",
            &[vec![
                Bar {
                    label: "course_depth_mm.npy".into(),
                    value: 0.2,
                    whisker: 0.0,
                    color: "#7f8c8d",
                    kind: "measured no-return",
                },
                Bar {
                    label: "course_depth_mm.npy".into(),
                    value: 0.05,
                    whisker: 0.01,
                    color: "#b9770e",
                    kind: "measured below configured gate",
                },
            ]],
        );
        assert!(svg.contains("data-legend=\"swatch\""), "{svg}");
        assert!(svg.contains("data-axis-tick"), "{svg}");
        assert!(svg.contains("#7f8c8d"), "{svg}");
        assert!(svg.contains("#b9770e"), "{svg}");
        let pieces = text_payloads(&svg);
        assert!(
            pieces.iter().all(|piece| piece.chars().count() <= 100),
            "{pieces:?}"
        );
        assert!(pieces.iter().any(|piece| piece.contains('0')), "{svg}");
    }

    fn attr_pair(svg: &str, marker: &str, x_attr: &str, y_attr: &str) -> (f64, f64) {
        let at = svg
            .find(marker)
            .unwrap_or_else(|| panic!("missing {marker}"));
        let window = &svg[at.saturating_sub(80)..(at + 160).min(svg.len())];
        (attr_in(window, x_attr), attr_in(window, y_attr))
    }

    fn attr_f64(svg: &str, marker: &str, name: &str) -> f64 {
        let at = svg
            .find(marker)
            .unwrap_or_else(|| panic!("missing {marker} in {svg}"));
        let window = &svg[at.saturating_sub(40)..(at + 180).min(svg.len())];
        attr_in(window, name)
    }

    fn attr_in(window: &str, name: &str) -> f64 {
        let key = format!("{name}=\"");
        let start = window
            .find(&key)
            .unwrap_or_else(|| panic!("missing {name} in {window}"));
        let rest = &window[start + key.len()..];
        let end = rest.find('"').unwrap();
        rest[..end].parse().unwrap()
    }

    fn path_point(svg: &str, index: usize) -> (f64, f64) {
        let start = svg.find("d=\"").unwrap();
        let rest = &svg[start + 3..];
        let end = rest.find('"').unwrap();
        let body = &rest[..end];
        let mut numbers = Vec::new();
        let mut token = String::new();
        for ch in body.chars() {
            if ch.is_ascii_digit() || ch == '.' || ch == '-' {
                token.push(ch);
            } else if !token.is_empty() {
                numbers.push(token.parse::<f64>().unwrap());
                token.clear();
            }
        }
        if !token.is_empty() {
            numbers.push(token.parse().unwrap());
        }
        (numbers[index * 2], numbers[index * 2 + 1])
    }

    fn text_payloads(svg: &str) -> Vec<String> {
        let mut out = Vec::new();
        let bytes = svg.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'>' {
                let start = index + 1;
                if let Some(rel) = svg[start..].find('<') {
                    let piece = svg[start..start + rel].trim();
                    if !piece.is_empty() {
                        out.push(piece.to_string());
                    }
                    index = start + rel;
                    continue;
                }
            }
            index += 1;
        }
        out
    }
}
