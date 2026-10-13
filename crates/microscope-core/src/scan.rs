//! Scan planning (P34, ADR-0028): where a scan goes next. A planner hands
//! out sites, stage positions in steps, each with a z to start focusing
//! from, and is told what happened at each. The behaviour spec opens the
//! P34 notes.

use std::collections::VecDeque;
use std::fmt;

use crate::mapping::{Matrix2, apply};

/// A position of the stage in x and y, in steps.
pub type Site = [i64; 2];

/// How far apart neighbouring sites are, in stage steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    /// In x: across the image.
    pub dx: i64,
    /// In y: down the image.
    pub dy: i64,
}

/// What happened at a site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visit {
    /// Where.
    pub site: Site,
    /// The z the stage ended at.
    pub z: i64,
    /// Whether an image was taken, rather than skipped as background.
    pub imaged: bool,
    /// Whether autofocus succeeded.
    pub focused: bool,
    /// Whether it was halfway between grid sites (the smart spiral's).
    pub between: bool,
}

/// Why a planner finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finished {
    /// Every planned site was visited.
    AllVisited,
    /// It reached its maximum range.
    ReachedRange,
    /// The scanned area is surrounded by background.
    SurroundedByBackground,
}

/// Why a plan can't be made.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlanError {
    /// The overlap isn't from 0 to less than 1.
    Overlap(f64),
    /// A step would be 0 steps.
    ZeroStep,
    /// A grid would have no columns or rows.
    ZeroCount,
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanError::Overlap(overlap) => write!(f, "an overlap of {overlap} isn't from 0 to 1"),
            PlanError::ZeroStep => f.write_str("the step between sites would be 0 steps"),
            PlanError::ZeroCount => f.write_str("a grid needs at least one column and row"),
        }
    }
}

impl std::error::Error for PlanError {}

/// The counts of the C-Chip pattern's columns and rows.
pub const C_CHIP_COUNTS: [u32; 2] = [5, 7];
/// The C-Chip pattern's overlap.
pub const C_CHIP_OVERLAP: f64 = 0.1;

impl Step {
    /// The step between frames of `size` (width, height) that overlap by
    /// `overlap`, through `matrix`, the camera–stage mapping's image-to-stage
    /// matrix. Each axis takes the image's motion that moves the stage most
    /// along it, so a camera turned a quarter still works.
    pub fn from_overlap(matrix: &Matrix2, size: [u32; 2], overlap: f64) -> Result<Step, PlanError> {
        if !(0.0..1.0).contains(&overlap) {
            return Err(PlanError::Overlap(overlap));
        }
        let across = apply(matrix, [f64::from(size[0]) * (1.0 - overlap), 0.0]);
        let down = apply(matrix, [0.0, f64::from(size[1]) * (1.0 - overlap)]);
        let (dx, dy) = if across[0].abs() > across[1].abs() {
            (across[0], down[1])
        } else {
            (down[0], across[1])
        };
        Step::checked(dx.round() as i64, dy.round() as i64)
    }

    /// The same step with both sizes the smaller of the two, keeping signs.
    pub fn equal(self) -> Step {
        let size = self.dx.abs().min(self.dy.abs());
        Step {
            dx: size * self.dx.signum(),
            dy: size * self.dy.signum(),
        }
    }

    fn checked(dx: i64, dy: i64) -> Result<Step, PlanError> {
        if dx == 0 || dy == 0 {
            return Err(PlanError::ZeroStep);
        }
        Ok(Step { dx, dy })
    }

    /// How many grid moves from `a` to `b`, in x and in y.
    fn moves(self, a: Site, b: Site) -> [f64; 2] {
        [
            ((b[0] - a[0]) as f64 / self.dx as f64).abs(),
            ((b[1] - a[1]) as f64 / self.dy as f64).abs(),
        ]
    }

    fn neighbours(self, site: Site) -> [Site; 4] {
        let [x, y] = site;
        [
            [x - self.dx, y],
            [x + self.dx, y],
            [x, y - self.dy],
            [x, y + self.dy],
        ]
    }
}

/// Plans a scan: see the module's description.
pub trait Planner {
    /// The next site, and the z to start focusing from if there's one to
    /// go by; `None` once it has finished.
    fn next(&self) -> Option<(Site, Option<i64>)>;
    /// Notes what happened at the site `next` gave: the z the stage ended
    /// at, whether an image was taken, and whether autofocus succeeded.
    fn visited(&mut self, z: i64, imaged: bool, focused: bool);
    /// Every visit so far, in order.
    fn visits(&self) -> &[Visit];
    /// Why it finished, once it has.
    fn finished(&self) -> Option<Finished>;
}

/// The straight distance between two sites, in steps.
fn distance(a: Site, b: Site) -> f64 {
    ((a[0] - b[0]) as f64).hypot((a[1] - b[1]) as f64)
}

/// Raster or snake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// Every row in the same direction.
    Raster,
    /// Every other row reversed.
    Snake,
}

/// A raster or snake over a grid of columns and rows.
#[derive(Debug, Clone)]
pub struct RegularGrid {
    step: Step,
    remaining: VecDeque<Site>,
    visits: Vec<Visit>,
}

impl RegularGrid {
    /// `counts` (columns, rows) sites from `start`, in `order`.
    pub fn new(start: Site, step: Step, counts: [u32; 2], order: Order) -> Result<Self, PlanError> {
        let step = Step::checked(step.dx, step.dy)?;
        let [columns, rows] = counts.map(i64::from);
        if columns == 0 || rows == 0 {
            return Err(PlanError::ZeroCount);
        }
        let site = |column: i64, row: i64| [start[0] + column * step.dx, start[1] + row * step.dy];
        let remaining = (0..rows)
            .flat_map(|row| {
                let reversed = order == Order::Snake && row % 2 == 1;
                (0..columns).map(move |i| site(if reversed { columns - 1 - i } else { i }, row))
            })
            .collect();
        Ok(RegularGrid {
            step,
            remaining,
            visits: Vec::new(),
        })
    }

    /// The C-Chip pattern from `start`: a snake of `C_CHIP_COUNTS`.
    pub fn c_chip(start: Site, step: Step) -> Result<Self, PlanError> {
        RegularGrid::new(start, step, C_CHIP_COUNTS, Order::Snake)
    }
}

impl Planner for RegularGrid {
    fn next(&self) -> Option<(Site, Option<i64>)> {
        let &site = self.remaining.front()?;
        // The focused site fewest moves away, the latest if several are.
        let moves = |visit: &Visit| self.step.moves(site, visit.site).iter().sum::<f64>();
        let nearest = (self.visits.iter().rev())
            .filter(|visit| visit.focused)
            .min_by(|a, b| moves(a).total_cmp(&moves(b)));
        Some((site, nearest.map(|visit| visit.z)))
    }

    fn visited(&mut self, z: i64, imaged: bool, focused: bool) {
        if let Some(site) = self.remaining.pop_front() {
            let between = false;
            self.visits.push(Visit {
                site,
                z,
                imaged,
                focused,
                between,
            });
        }
    }

    fn visits(&self) -> &[Visit] {
        &self.visits
    }

    fn finished(&self) -> Option<Finished> {
        self.remaining.is_empty().then_some(Finished::AllVisited)
    }
}

/// A site the smart spiral has planned.
#[derive(Debug, Clone, Copy)]
struct Planned {
    site: Site,
    between: bool,
}

/// Spirals out from the start, around the sample, skipping background,
/// within a maximum range.
#[derive(Debug, Clone)]
pub struct SmartSpiral {
    start: Site,
    step: Step,
    max_range: i64,
    remaining: Vec<Planned>,
    visits: Vec<Visit>,
    out_of_range: bool,
}

impl SmartSpiral {
    /// A spiral from `start`, no further from it than `max_range` steps.
    pub fn new(start: Site, step: Step, max_range: i64) -> Result<Self, PlanError> {
        Ok(SmartSpiral {
            start,
            step: Step::checked(step.dx, step.dy)?,
            max_range,
            remaining: vec![Planned {
                site: start,
                between: false,
            }],
            visits: Vec::new(),
            out_of_range: false,
        })
    }

    /// The main site visited at `site`, if any.
    fn visit_at(&self, site: Site) -> Option<&Visit> {
        self.visits.iter().find(|v| !v.between && v.site == site)
    }

    fn halfway(&mut self, a: Site, b: Site) {
        let site = [(a[0] + b[0]).div_euclid(2), (a[1] + b[1]).div_euclid(2)];
        self.remaining.push(Planned {
            site,
            between: true,
        });
    }

    /// After an image at `site`: its neighbours, or what fills the gap to
    /// the background beside it.
    fn surround(&mut self, site: Site) {
        for next in self.step.neighbours(site) {
            if self.remaining.iter().any(|p| !p.between && p.site == next) {
                continue;
            }
            match self.visit_at(next).map(|v| v.imaged) {
                Some(true) => {}
                Some(false) => self.halfway(site, next),
                None if distance(next, self.start) > self.max_range as f64 => {
                    self.out_of_range = true;
                }
                None => self.remaining.push(Planned {
                    site: next,
                    between: false,
                }),
            }
        }
    }

    /// After background at `site`: what fills the gap to each image beside it.
    fn fill(&mut self, site: Site) {
        for next in self.step.neighbours(site) {
            if self.visit_at(next).is_some_and(|v| v.imaged) {
                self.halfway(site, next);
            }
        }
    }

    /// Between sites first, then the fewest grid moves from `here`, then
    /// from the start (each the larger of the moves in x and y), then the
    /// shortest distance from `here`.
    fn sort(&mut self, here: Site) {
        let (step, start) = (self.step, self.start);
        let moves = |a: Site, b: Site| {
            let [x, y] = step.moves(a, b);
            x.max(y)
        };
        self.remaining.sort_by(|a, b| {
            (b.between.cmp(&a.between))
                .then(moves(here, a.site).total_cmp(&moves(here, b.site)))
                .then(moves(start, a.site).total_cmp(&moves(start, b.site)))
                .then(distance(here, a.site).total_cmp(&distance(here, b.site)))
        });
    }
}

impl Planner for SmartSpiral {
    fn next(&self) -> Option<(Site, Option<i64>)> {
        let site = self.remaining.first()?.site;
        let focused: Vec<&Visit> = self.visits.iter().filter(|v| v.focused).collect();
        let near: Vec<&Visit> = (focused.iter().copied())
            .filter(|v| {
                (v.site[0] - site[0]).abs() <= self.step.dx.abs()
                    && (v.site[1] - site[1]).abs() <= self.step.dy.abs()
            })
            .collect();
        let pool = if near.is_empty() {
            let nearest = (focused.iter())
                .map(|v| distance(site, v.site))
                .min_by(f64::total_cmp);
            (focused.iter().copied())
                .filter(|v| Some(distance(site, v.site)) == nearest)
                .collect()
        } else {
            near
        };
        // The lowest z, the latest if several are.
        let lowest = pool.iter().rev().min_by_key(|v| v.z);
        Some((site, lowest.map(|v| v.z)))
    }

    fn visited(&mut self, z: i64, imaged: bool, focused: bool) {
        if self.remaining.is_empty() {
            return;
        }
        let Planned { site, between } = self.remaining.remove(0);
        self.visits.push(Visit {
            site,
            z,
            imaged,
            focused,
            between,
        });
        if between {
            return;
        }
        if imaged {
            self.surround(site);
        } else {
            self.fill(site);
        }
        self.sort(site);
    }

    fn visits(&self) -> &[Visit] {
        &self.visits
    }

    fn finished(&self) -> Option<Finished> {
        self.remaining.is_empty().then_some(if self.out_of_range {
            Finished::ReachedRange
        } else {
            Finished::SurroundedByBackground
        })
    }
}
