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

pub fn xy_plot(
    title: &str,
    footer: &str,
    x_label: &str,
    y_label: &str,
    series: &[XySeries],
) -> String {
    let mut min_x = 0.0_f64;
    let mut max_x = 0.0_f64;
    let mut min_y = 0.0_f64;
    let mut max_y = 0.0_f64;
    for item in series {
        for (x, y) in &item.points {
            if x.is_finite() && y.is_finite() {
                min_x = min_x.min(*x);
                max_x = max_x.max(*x);
                min_y = min_y.min(*y);
                max_y = max_y.max(*y);
            }
        }
    }
    let span = (max_x - min_x).max(max_y - min_y).max(0.5) * 1.1;
    let left = 70.0;
    let top = 70.0;
    let size = 420.0;
    let mut body = String::new();
    body.push_str(&line(
        left,
        top + size / 2.0,
        left + size,
        top + size / 2.0,
        "#bbbbbb",
    ));
    body.push_str(&line(
        left + size / 2.0,
        top,
        left + size / 2.0,
        top + size,
        "#bbbbbb",
    ));
    for item in series {
        let mut path = String::new();
        for (x, y) in &item.points {
            if !x.is_finite() || !y.is_finite() {
                continue;
            }
            let px = left + (*x - (min_x + max_x) / 2.0) / span * size + size / 2.0;
            let py = top + size / 2.0 - (*y - (min_y + max_y) / 2.0) / span * size;
            if path.is_empty() {
                path.push_str(&format!("M {px:.1} {py:.1}"));
            } else {
                path.push_str(&format!(" L {px:.1} {py:.1}"));
            }
        }
        if !path.is_empty() {
            body.push_str(&format!(
                "<path d=\"{path}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.2\"/>",
                item.color
            ));
        }
    }
    let mut legend = String::new();
    for (index, item) in series.iter().enumerate() {
        let y = 510.0 + index as f64 * 18.0;
        legend.push_str(&format!(
            "<rect x=\"70\" y=\"{y:.0}\" width=\"14\" height=\"10\" fill=\"{}\"/><text x=\"90\" y=\"{:.0}\" font-size=\"12\">{} ({})</text>",
            item.color,
            y + 10.0,
            esc(&item.name),
            esc(item.kind)
        ));
    }
    document(
        980,
        560 + series.len() as u32 * 18,
        &format!(
            "<text x=\"70\" y=\"28\" font-size=\"15\">{}</text>\
             <text x=\"70\" y=\"48\" font-size=\"12\">x: {} | y: {}</text>\
             {body}{legend}<text x=\"70\" y=\"500\" font-size=\"12\">{}</text>",
            esc(title),
            esc(x_label),
            esc(y_label),
            esc(footer)
        ),
    )
}

pub fn bar_groups(title: &str, footer: &str, y_label: &str, groups: &[Vec<Bar>]) -> String {
    let mut peak = 0.1_f64;
    for group in groups {
        for bar in group {
            if bar.value.is_finite() {
                peak = peak.max(bar.value.abs() + bar.whisker.abs());
            }
        }
    }
    let left = 70.0;
    let top = 60.0;
    let height = 360.0;
    let width = 860.0;
    let slot = if groups.is_empty() {
        width
    } else {
        width / groups.len() as f64
    };
    let mut body = String::new();
    let baseline = top + height;
    body.push_str(&line(left, baseline, left + width, baseline, "#444444"));
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
                body.push_str(&line(mid, up, mid, down, "#222222"));
            }
            if bar_index == 0 {
                body.push_str(&format!(
                    "<text x=\"{x:.1}\" y=\"{:.0}\" font-size=\"11\" transform=\"rotate(40 {x:.1} {:.0})\">{}</text>",
                    baseline + 16.0,
                    baseline + 16.0,
                    esc(&bar.label)
                ));
            }
        }
    }
    let mut kinds = Vec::new();
    for group in groups {
        for bar in group {
            if !kinds.contains(&bar.kind) {
                kinds.push(bar.kind);
            }
        }
    }
    let legend = kinds
        .iter()
        .enumerate()
        .map(|(index, kind)| {
            format!(
                "<text x=\"70\" y=\"{}\" font-size=\"11\">{}</text>",
                500 + index * 14,
                esc(kind)
            )
        })
        .collect::<String>();
    document(
        980,
        560 + kinds.len() as u32 * 14,
        &format!(
            "<text x=\"70\" y=\"24\" font-size=\"15\">{}</text>\
             <text x=\"70\" y=\"44\" font-size=\"12\">y: {} | peak {:.4}</text>\
             {body}{legend}<text x=\"70\" y=\"{}\" font-size=\"12\">{}</text>",
            esc(title),
            esc(y_label),
            peak,
            540 + kinds.len() * 14,
            esc(footer)
        ),
    )
}

pub fn frame_schematic(title: &str, footer: &str, notes: &[String]) -> String {
    let mut lines = String::new();
    for (index, note) in notes.iter().enumerate() {
        let y = 80 + index * 22;
        lines.push_str(&format!(
            "<text x=\"40\" y=\"{y}\" font-size=\"14\">{}</text>",
            esc(note)
        ));
    }
    document(
        980,
        80 + notes.len() as u32 * 22 + 40,
        &format!(
            "<text x=\"40\" y=\"36\" font-size=\"16\">{}</text>{lines}<text x=\"40\" y=\"{}\" font-size=\"12\">{}</text>",
            esc(title),
            80 + notes.len() * 22 + 20,
            esc(footer)
        ),
    )
}

fn document(width: u32, height: u32, body: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\
         <rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>{body}</svg>\n"
    )
}

fn line(x1: f64, y1: f64, x2: f64, y2: f64, color: &str) -> String {
    format!(
        "<line x1=\"{x1:.1}\" y1=\"{y1:.1}\" x2=\"{x2:.1}\" y2=\"{y2:.1}\" stroke=\"{color}\"/>"
    )
}

fn esc(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
