//! Draws planned scans as SVG, for review (P34): a snake grid, and the smart
//! spiral over a sample of two overlapping disks, within its range. Images
//! are purple, background grey and the spiral's between sites orange, joined
//! in the order they're visited.
//!
//! ```text
//! cargo run -p microscope-core --example plan -- <folder>
//! ```

use std::fmt::Write as _;
use std::path::PathBuf;

use microscope_core::scan::{Order, Planner, RegularGrid, Site, SmartSpiral, Step, Visit};

fn main() -> std::io::Result<()> {
    let folder = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".to_owned()));
    let step = Step { dx: 100, dy: 80 };
    let sample = |site: Site| {
        let within =
            |c: Site, r: f64| ((site[0] - c[0]) as f64).hypot((site[1] - c[1]) as f64) <= r;
        within([0, 0], 600.0) || within([700, 300], 400.0)
    };
    let mut grid = RegularGrid::new([-500, -400], step, [8, 6], Order::Snake).expect("a grid");
    let mut spiral = SmartSpiral::new([0, 0], step, 1000).expect("a spiral");
    for (name, planner) in [
        ("snake", &mut grid as &mut dyn Planner),
        ("spiral", &mut spiral),
    ] {
        while let Some((site, _)) = planner.next() {
            let imaged = sample(site);
            planner.visited(0, imaged, imaged);
        }
        let file = folder.join(format!("{name}.svg"));
        std::fs::write(&file, svg(planner.visits(), step))?;
        let (sites, finished) = (planner.visits().len(), planner.finished());
        println!("{name}: {sites} sites, {finished:?}, in {}", file.display());
    }
    Ok(())
}

/// Each visit's field of view, coloured by what was found there, and the path.
fn svg(visits: &[Visit], step: Step) -> String {
    let (w, h) = (step.dx.abs(), step.dy.abs());
    let xs = visits.iter().map(|v| v.site[0]);
    let ys = visits.iter().map(|v| v.site[1]);
    let (left, top) = (
        xs.clone().min().unwrap_or(0) - w,
        ys.clone().min().unwrap_or(0) - h,
    );
    let (right, bottom) = (xs.max().unwrap_or(0) + w, ys.max().unwrap_or(0) + h);
    let mut out = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{left} {top} {} {}">"#,
        right - left,
        bottom - top
    );
    for visit in visits {
        let fill = match (visit.between, visit.imaged) {
            (true, _) => "#e08a1e",
            (false, true) => "#8e44ad",
            (false, false) => "#bdbdbd",
        };
        let [x, y] = visit.site;
        let _ = write!(
            out,
            r#"<rect x="{}" y="{}" width="{w}" height="{h}" fill="{fill}" fill-opacity="0.45" stroke="white"/>"#,
            x - w / 2,
            y - h / 2
        );
    }
    let points: Vec<String> = visits
        .iter()
        .map(|v| format!("{},{}", v.site[0], v.site[1]))
        .collect();
    let _ = write!(
        out,
        r#"<polyline points="{}" fill="none" stroke="black" stroke-width="6"/></svg>"#,
        points.join(" ")
    );
    out
}
