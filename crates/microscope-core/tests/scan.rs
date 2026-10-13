//! Scan planning (P34): the grid's step from the mapping; raster, snake and
//! C-Chip grids; the smart spiral over scripted samples; and the z to start
//! focusing from. Properties are checked over many random plans: sites are
//! visited once, within range, each next to the sample already found.

use std::collections::HashSet;

use microscope_core::scan::{
    C_CHIP_COUNTS, Finished, Order, PlanError, Planner, RegularGrid, Site, SmartSpiral, Step, Visit,
};

/// Runs `planner` to its end: an image where `sample` says, in focus there,
/// at the z `focus` gives.
fn run(
    planner: &mut impl Planner,
    sample: impl Fn(Site) -> bool,
    focus: impl Fn(Site) -> i64,
) -> Vec<Visit> {
    for _ in 0..5_000 {
        let Some((site, _)) = planner.next() else {
            return planner.visits().to_vec();
        };
        let imaged = sample(site);
        planner.visited(focus(site), imaged, imaged);
    }
    panic!("the plan didn't end");
}

/// A small deterministic generator for the property checks.
struct Random(u64);

impl Random {
    fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % n
    }
}

#[test]
fn the_step_is_the_field_less_the_overlap_through_the_mapping() {
    let size = [820, 616];
    let step = |matrix| Step::from_overlap(&matrix, size, 0.35);
    assert_eq!(
        step([[1.12, 0.0], [0.0, 1.12]]),
        Ok(Step { dx: 597, dy: 448 })
    );
    // Flipped in x, the grid still runs right in the image.
    assert_eq!(
        step([[-1.12, 0.0], [0.0, 1.12]]),
        Ok(Step { dx: -597, dy: 448 })
    );
    // Turned a quarter, x comes from the step down the image.
    assert_eq!(
        step([[0.0, 1.12], [1.12, 0.0]]),
        Ok(Step { dx: 448, dy: 597 })
    );
    assert_eq!(
        Step { dx: -597, dy: 448 }.equal(),
        Step { dx: -448, dy: 448 }
    );
    for overlap in [-0.1, 1.0, f64::NAN] {
        let refused = Step::from_overlap(&[[1.0, 0.0], [0.0, 1.0]], size, overlap);
        assert!(matches!(refused, Err(PlanError::Overlap(_))), "{overlap}");
    }
    assert_eq!(step([[0.0005, 0.0], [0.0, 1.0]]), Err(PlanError::ZeroStep));
}

#[test]
fn raster_and_snake_grids_visit_every_site_in_their_order() {
    let step = Step { dx: 10, dy: -5 };
    let sites = |order| {
        let mut grid = RegularGrid::new([100, 0], step, [3, 2], order).expect("a grid");
        let visits = run(&mut grid, |_| true, |_| 0);
        assert_eq!(grid.finished(), Some(Finished::AllVisited));
        visits.iter().map(|v| v.site).collect::<Vec<_>>()
    };
    assert_eq!(
        sites(Order::Raster),
        [
            [100, 0],
            [110, 0],
            [120, 0],
            [100, -5],
            [110, -5],
            [120, -5]
        ]
    );
    assert_eq!(
        sites(Order::Snake),
        [
            [100, 0],
            [110, 0],
            [120, 0],
            [120, -5],
            [110, -5],
            [100, -5]
        ]
    );
    let c_chip = RegularGrid::c_chip([0, 0], step).expect("a grid");
    assert_eq!(run(&mut c_chip.clone(), |_| true, |_| 0).len(), 35);
    assert_eq!(C_CHIP_COUNTS, [5, 7]);
    let refused = RegularGrid::new([0, 0], step, [0, 3], Order::Snake);
    assert!(matches!(refused, Err(PlanError::ZeroCount)));
    let refused = RegularGrid::new([0, 0], Step { dx: 0, dy: 1 }, [2, 2], Order::Raster);
    assert!(matches!(refused, Err(PlanError::ZeroStep)));
}

#[test]
fn grids_are_whole_and_unique_and_snakes_move_to_neighbours() {
    let mut random = Random(7);
    for _ in 0..300 {
        let counts = [1 + random.below(9) as u32, 1 + random.below(9) as u32];
        let step = Step {
            dx: random.below(400) as i64 - 200,
            dy: random.below(400) as i64 - 200,
        };
        let start = [
            random.below(2000) as i64 - 1000,
            random.below(2000) as i64 - 1000,
        ];
        for order in [Order::Raster, Order::Snake] {
            let Ok(mut grid) = RegularGrid::new(start, step, counts, order) else {
                assert!(step.dx == 0 || step.dy == 0);
                continue;
            };
            let sites: Vec<Site> = run(&mut grid, |_| true, |_| 0)
                .iter()
                .map(|v| v.site)
                .collect();
            assert_eq!(sites.len() as u32, counts[0] * counts[1]);
            assert_eq!(sites.iter().collect::<HashSet<_>>().len(), sites.len());
            for site in &sites {
                let (column, row) = (
                    (site[0] - start[0]) / step.dx,
                    (site[1] - start[1]) / step.dy,
                );
                assert!(
                    (0..counts[0] as i64).contains(&column) && (0..counts[1] as i64).contains(&row)
                );
            }
            if order == Order::Snake {
                for pair in sites.windows(2) {
                    let moves = ((pair[1][0] - pair[0][0]) / step.dx).abs()
                        + ((pair[1][1] - pair[0][1]) / step.dy).abs();
                    assert_eq!(moves, 1, "{pair:?}");
                }
            }
        }
    }
}

#[test]
fn a_grid_starts_focusing_from_the_nearest_focused_site() {
    let step = Step { dx: 10, dy: 10 };
    let mut grid = RegularGrid::new([0, 0], step, [3, 3], Order::Snake).expect("a grid");
    assert_eq!(grid.next(), Some(([0, 0], None)));
    // The first row in focus at z 1 and 2, then not at its end.
    grid.visited(1, true, true);
    grid.visited(2, true, true);
    grid.visited(3, true, false);
    // (20, 10) is 2 moves from (10, 0) and 3 from (0, 0).
    assert_eq!(grid.next(), Some(([20, 10], Some(2))));
    grid.visited(4, true, true);
    // (10, 10) is 1 move from (10, 0) and from (20, 10): the latest wins.
    assert_eq!(grid.next(), Some(([10, 10], Some(4))));
}

/// A round sample of `radius` steps about `centre`.
fn disk(centre: Site, radius: f64) -> impl Fn(Site) -> bool {
    move |site| ((site[0] - centre[0]) as f64).hypot((site[1] - centre[1]) as f64) <= radius
}

/// The spiral's properties, for any sample: main sites are visited once,
/// within range, and each after the first is next to one with an image;
/// every between site is halfway between an image and background.
fn check_spiral(start: Site, step: Step, range: i64, sample: &dyn Fn(Site) -> bool) -> Vec<Visit> {
    let mut spiral = SmartSpiral::new(start, step, range).expect("a spiral");
    let visits = run(&mut spiral, sample, |site| site[0] / 10);
    let main: Vec<&Visit> = visits.iter().filter(|v| !v.between).collect();
    let sites: HashSet<Site> = main.iter().map(|v| v.site).collect();
    assert_eq!(sites.len(), main.len(), "a main site twice");
    assert_eq!(visits[0].site, start);
    let on_grid =
        |site: Site| (site[0] - start[0]) % step.dx == 0 && (site[1] - start[1]) % step.dy == 0;
    assert!(
        main.iter().all(|v| on_grid(v.site)),
        "a main site off the grid"
    );
    let distance = |a: Site, b: Site| ((a[0] - b[0]) as f64).hypot((a[1] - b[1]) as f64);
    for (i, visit) in main.iter().enumerate().skip(1) {
        assert!(
            distance(visit.site, start) <= range as f64,
            "{visit:?} out of range"
        );
        let beside = |v: &&&Visit| {
            v.imaged && distance(v.site, visit.site) <= step.dx.abs().max(step.dy.abs()) as f64
        };
        assert!(
            main[..i].iter().any(|v| beside(&v)),
            "{visit:?} isn't next to an image"
        );
    }
    for visit in visits.iter().filter(|v| v.between) {
        let ends = main.iter().filter(|v| {
            distance(v.site, visit.site) <= step.dx.abs().max(step.dy.abs()) as f64 / 2.0 + 1.0
        });
        let (imaged, background): (Vec<&&Visit>, Vec<&&Visit>) = ends.partition(|v| v.imaged);
        assert!(
            !imaged.is_empty() && !background.is_empty(),
            "{visit:?} isn't between image and background"
        );
    }
    // Every gap between an image and background beside it is filled.
    let between: HashSet<Site> = visits
        .iter()
        .filter(|v| v.between)
        .map(|v| v.site)
        .collect();
    for image in main.iter().filter(|v| v.imaged) {
        let [x, y] = image.site;
        for next in [
            [x - step.dx, y],
            [x + step.dx, y],
            [x, y - step.dy],
            [x, y + step.dy],
        ] {
            if main.iter().any(|v| v.site == next && !v.imaged) {
                let halfway = [(x + next[0]).div_euclid(2), (y + next[1]).div_euclid(2)];
                assert!(between.contains(&halfway), "no gap filled at {halfway:?}");
            }
        }
    }
    assert!(spiral.finished().is_some());
    visits
}

#[test]
fn the_spiral_covers_a_round_sample_and_stops_at_its_edge() {
    let step = Step { dx: 100, dy: 80 };
    let sample = disk([0, 0], 500.0);
    let visits = check_spiral([0, 0], step, 10_000, &sample);
    let mut spiral = SmartSpiral::new([0, 0], step, 10_000).expect("a spiral");
    run(&mut spiral, &sample, |_| 0);
    assert_eq!(spiral.finished(), Some(Finished::SurroundedByBackground));
    // Every grid site on the sample is imaged.
    let imaged: HashSet<Site> = visits
        .iter()
        .filter(|v| v.imaged && !v.between)
        .map(|v| v.site)
        .collect();
    for i in -6..=6 {
        for j in -7..=7 {
            let site = [i * 100, j * 80];
            assert_eq!(imaged.contains(&site), sample(site), "{site:?}");
        }
    }
    // The first nine go round the start, before going further.
    let first: HashSet<Site> = visits[..9].iter().map(|v| v.site).collect();
    let ring: HashSet<Site> = (-1..=1)
        .flat_map(|i| (-1..=1).map(move |j| [i * 100, j * 80]))
        .collect();
    assert_eq!(first, ring);
    // The edge's gaps are filled: halfway from the last image in x, at
    // 500, on the radius, to the background at 600.
    assert!(visits.iter().any(|v| v.between && v.site == [550, 0]));
}

#[test]
fn the_spiral_stops_at_its_range_and_says_so() {
    let step = Step { dx: 100, dy: 100 };
    let mut spiral = SmartSpiral::new([1000, 0], step, 250).expect("a spiral");
    let visits = run(&mut spiral, |_| true, |_| 0);
    assert_eq!(spiral.finished(), Some(Finished::ReachedRange));
    // Every grid site within 250 steps, and only those.
    let sites: HashSet<Site> = visits.iter().map(|v| v.site).collect();
    let within: HashSet<Site> = (-2..=2)
        .flat_map(|i| (-2..=2).map(move |j| [1000 + i * 100, j * 100]))
        .filter(|s| ((s[0] - 1000) as f64).hypot(s[1] as f64) <= 250.0)
        .collect();
    assert_eq!(sites, within);
}

#[test]
fn the_spiral_keeps_its_properties_on_random_samples() {
    let mut random = Random(11);
    for _ in 0..60 {
        let blobs: Vec<(Site, f64)> = (0..1 + random.below(4))
            .map(|_| {
                let centre = [
                    random.below(1200) as i64 - 600,
                    random.below(1200) as i64 - 600,
                ];
                (centre, 100.0 + random.below(400) as f64)
            })
            .collect();
        let sample = move |site: Site| {
            blobs
                .iter()
                .any(|&(centre, radius)| disk(centre, radius)(site))
        };
        let step = Step {
            dx: 60 + random.below(80) as i64,
            dy: -(60 + random.below(80) as i64),
        };
        let range = 200 + random.below(900) as i64;
        check_spiral([0, 0], step, range, &sample);
    }
}

#[test]
fn the_spiral_starts_focusing_from_the_lowest_nearby_focus() {
    let step = Step { dx: 100, dy: 100 };
    let mut spiral = SmartSpiral::new([0, 0], step, 10_000).expect("a spiral");
    assert_eq!(spiral.next(), Some(([0, 0], None)));
    spiral.visited(50, true, true);
    // Its neighbours are planned left, right, up and down, and all tie.
    assert_eq!(spiral.next(), Some(([-100, 0], Some(50))));
    spiral.visited(20, true, true);
    // Then the nearest to here first: a corner, near both focused sites.
    assert_eq!(spiral.next(), Some(([-100, -100], Some(20))));
    spiral.visited(30, false, false);
    // Background: the gap to the image beside it is filled first, and the
    // unfocused background doesn't count.
    assert_eq!(spiral.next(), Some(([-100, -50], Some(20))));
}

/// The z to start from at `site`, by the spec: the lowest of the focused
/// sites within a step in x and y, else of the nearest focused sites.
fn spiral_z(step: Step, visits: &[Visit], site: Site) -> (Option<i64>, bool) {
    let focused: Vec<&Visit> = visits.iter().filter(|v| v.focused).collect();
    let near: Vec<&Visit> = (focused.iter().copied())
        .filter(|v| {
            (v.site[0] - site[0]).abs() <= step.dx.abs()
                && (v.site[1] - site[1]).abs() <= step.dy.abs()
        })
        .collect();
    if !near.is_empty() {
        return (near.iter().map(|v| v.z).min(), true);
    }
    let distance = |v: &&Visit| ((v.site[0] - site[0]) as f64).hypot((v.site[1] - site[1]) as f64);
    let nearest = focused.iter().map(distance).min_by(f64::total_cmp);
    let z = (focused.iter())
        .filter(|v| Some(distance(v)) == nearest)
        .map(|v| v.z)
        .min();
    (z, false)
}

#[test]
fn the_spiral_follows_its_focus_rule_on_random_scans() {
    let mut random = Random(23);
    let mut far = 0;
    for _ in 0..40 {
        let centre = [
            random.below(600) as i64 - 300,
            random.below(600) as i64 - 300,
        ];
        let sample = disk(centre, 200.0 + random.below(500) as f64);
        let step = Step {
            dx: 80 + random.below(60) as i64,
            dy: 80 + random.below(60) as i64,
        };
        let mut spiral = SmartSpiral::new([0, 0], step, 2000).expect("a spiral");
        while let Some((site, z)) = spiral.next() {
            assert!(spiral.visits().len() < 5_000, "the plan didn't end");
            let (expected, near) = spiral_z(step, spiral.visits(), site);
            assert_eq!(z, expected, "at {site:?}");
            far += usize::from(!near && expected.is_some());
            // Autofocus fails often, so some sites have no focus nearby.
            let imaged = sample(site);
            let focused = imaged && random.below(3) == 0;
            spiral.visited(random.below(1000) as i64, imaged, focused);
        }
    }
    assert!(far > 0, "no site was far from every focus");
}
