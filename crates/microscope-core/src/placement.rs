//! Correlation placement (P12, ADR-0046): where each image of a mosaic
//! really is, from its overlaps with its neighbours.
//!
//! 1. **Pairs:** two tiles whose stage placements overlap enough are
//!    registered on that overlap by phase correlation (ADR-0023), which says
//!    how far the second is from where the stage put it, against the first,
//!    and how sure that is. A second registration, on the overlap the first
//!    found, takes out the first's slight shortfall on large shifts.
//! 2. **Rejects:** a pair less sure than [`PlacementOptions::min_confidence`],
//!    or whose correction is larger than [`PlacementOptions::max_correction`],
//!    isn't used.
//! 3. **The solve:** every tile's position at once, by least squares, each
//!    pair weighted by its confidence, and each tile drawn weakly
//!    ([`PlacementOptions::anchor`]) to its stage placement. A tile with no
//!    pair used keeps its stage placement, and a group of tiles keeps
//!    theirs on average.
//! 4. **Pairs that disagree:** the used pair that disagrees most with where
//!    the solve put its two tiles, by more than
//!    [`PlacementOptions::max_residual`], is dropped, and the tiles are
//!    solved again, until none does. A confident but wrong match is outvoted
//!    by its tiles' other pairs.
//!
//! [`place`] does it all for tiles in memory, and [`place_with`] for tiles
//! read as they're needed. [`pair`] and [`solve_consistent`] do it a step at
//! a time, and [`solve`] is one solve.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::convert::Infallible;

use crate::registration::{Gray, Options, register};

/// How tiles are placed by correlation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacementOptions {
    /// The least overlap, each way, for a pair to be registered: a share of
    /// the tile's width and of its height.
    pub min_overlap: f64,
    /// The least confidence a pair's registration needs to be used.
    pub min_confidence: f64,
    /// The largest correction, in pixels, a pair may make: one further from
    /// its stage placement is rejected.
    pub max_correction: f64,
    /// How strongly each tile keeps to its stage placement, against a pair
    /// of confidence 1.
    pub anchor: f64,
    /// The most, in pixels, a used pair may disagree with where the solve
    /// puts its two tiles.
    pub max_residual: f64,
    /// How pairs are registered.
    pub registration: Options,
}

impl Default for PlacementOptions {
    /// Overlaps of 5% or more, used if at least 0.3 sure and within 50
    /// pixels, each tile anchored at 1/10,000 of a sure pair's weight, so a
    /// slow drift across a large scan isn't held back, and pairs dropped
    /// that disagree with the solve by more than 3 pixels.
    fn default() -> Self {
        Self {
            min_overlap: 0.05,
            min_confidence: 0.3,
            max_correction: 50.0,
            anchor: 1e-4,
            max_residual: 3.0,
            registration: Options::default(),
        }
    }
}

/// Two tiles that overlap, as registered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pair {
    /// The first tile's index.
    pub a: usize,
    /// The second's.
    pub b: usize,
    /// How far `b` is from where the stage put it, against `a`, in pixels.
    pub correction: [f64; 2],
    /// How sure the registration is.
    pub confidence: f64,
    /// Whether the solve uses it.
    pub used: bool,
}

/// Where the tiles are, and the pairs that said so.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    /// Each tile's top-left corner, in the mosaic's pixels.
    pub origins: Vec<[f64; 2]>,
    /// Every pair registered.
    pub pairs: Vec<Pair>,
}

impl Placement {
    /// How far each tile moved from `expected`, its stage placement, in
    /// pixels.
    pub fn moves(&self, expected: &[[f64; 2]]) -> Vec<f64> {
        (self.origins.iter().zip(expected))
            .map(|(o, e)| (o[0] - e[0]).hypot(o[1] - e[1]))
            .collect()
    }
}

/// Where each of `tiles` really is. `expected` is each one's top-left
/// corner, in the mosaic's pixels, where the stage put it. The tiles are all
/// one size.
pub fn place(tiles: &[Gray], expected: &[[f64; 2]], options: &PlacementOptions) -> Placement {
    assert_eq!(tiles.len(), expected.len(), "a placement for each tile");
    let size = tiles
        .first()
        .map_or([0; 2], |tile| [tile.width, tile.height]);
    let read = &mut |index: usize| Ok::<_, Infallible>(tiles[index].clone());
    match place_with(expected, size, read, options) {
        Ok(placement) => placement,
    }
}

/// [`place`] for tiles `size` pixels each (width and height), which `tile`
/// reads by index. Each is read once, when its first pair is registered, and
/// dropped after its last, so a scan's images, taken in order, are held a
/// row or two at a time. The first error reading one is returned.
pub fn place_with<E>(
    expected: &[[f64; 2]],
    size: [usize; 2],
    tile: &mut dyn FnMut(usize) -> Result<Gray, E>,
    options: &PlacementOptions,
) -> Result<Placement, E> {
    let n = expected.len();
    let candidates: Vec<(usize, usize)> = (0..n)
        .flat_map(|a| (a + 1..n).map(move |b| (a, b)))
        .filter(|&(a, b)| overlap(size, stage(expected, a, b).map(f64::round), options).is_some())
        .collect();
    let mut last = vec![0; n];
    for (k, &(a, b)) in candidates.iter().enumerate() {
        (last[a], last[b]) = (k, k);
    }
    let (mut held, mut pairs) = (HashMap::new(), Vec::new());
    for (k, &(a, b)) in candidates.iter().enumerate() {
        for index in [a, b] {
            if let Entry::Vacant(entry) = held.entry(index) {
                entry.insert(tile(index)?);
            }
        }
        pairs.extend(pair((a, &held[&a]), (b, &held[&b]), expected, options));
        held.retain(|&index, _| last[index] > k);
    }
    Ok(Placement {
        origins: solve_consistent(expected, &mut pairs, options),
        pairs,
    })
}

/// How far `b`'s stage placement is from `a`'s.
fn stage(expected: &[[f64; 2]], a: usize, b: usize) -> [f64; 2] {
    [0, 1].map(|axis| expected[b][axis] - expected[a][axis])
}

/// Tiles `a` and `b`, by index, registered on their overlap, or `None` if
/// their stage placements in `expected` don't overlap enough.
pub fn pair(
    (a, tile_a): (usize, &Gray),
    (b, tile_b): (usize, &Gray),
    expected: &[[f64; 2]],
    options: &PlacementOptions,
) -> Option<Pair> {
    let stage = stage(expected, a, b);
    let first = offset([tile_a, tile_b], stage.map(f64::round), options)?;
    let near = |offset: [f64; 2]| {
        let correction = [0, 1].map(|axis| offset[axis] - stage[axis]);
        (
            correction,
            correction[0].hypot(correction[1]) <= options.max_correction,
        )
    };
    let (mut correction, mut small) = near(first.0);
    let mut confidence = first.1;
    if small && let Some(second) = offset([tile_a, tile_b], first.0.map(f64::round), options) {
        (correction, small) = near(second.0);
        confidence = second.1;
    }
    Some(Pair {
        a,
        b,
        correction,
        confidence,
        used: small && confidence >= options.min_confidence,
    })
}

/// How far `b`'s top-left corner really is from `a`'s, and how sure, from
/// their overlap with `b` at `guess` from `a`, in whole pixels; `None` if
/// that's too small.
fn offset(
    [a, b]: [&Gray; 2],
    guess: [f64; 2],
    options: &PlacementOptions,
) -> Option<([f64; 2], f64)> {
    let (across, down) = overlap([a.width, a.height], guess, options)?;
    let [dx, dy] = guess;
    // Where the overlap starts in each tile.
    let start = |d: f64| (d.max(0.0) as usize, (-d).max(0.0) as usize);
    let ((xa, xb), (ya, yb)) = (start(dx), start(dy));
    let shift = register(
        &a.crop(xa, ya, across, down),
        &b.crop(xb, yb, across, down),
        &options.registration,
    );
    // `b`'s part shows what `a`'s does `guess − offset` further on.
    Some(([dx - shift.dx, dy - shift.dy], shift.confidence))
}

/// The overlap of two tiles `size` pixels each, the second at `guess` from
/// the first in whole pixels, across and down, if it's enough to register.
fn overlap(
    size: [usize; 2],
    guess: [f64; 2],
    options: &PlacementOptions,
) -> Option<(usize, usize)> {
    let least = (16 * options.registration.downsample.max(1)) as f64;
    let enough = |side: usize| (side as f64 * options.min_overlap).max(least);
    let (across, down) = (
        size[0] as f64 - guess[0].abs(),
        size[1] as f64 - guess[1].abs(),
    );
    let fits = across >= enough(size[0]) && down >= enough(size[1]);
    fits.then_some((across as usize, down as usize))
}

/// Every tile's top-left corner: the least-squares solution of each pair
/// used, `origin[b] − origin[a] = expected[b] − expected[a] + correction`,
/// weighted by its confidence, and of each tile's `origin = expected`,
/// weighted by `anchor`, which must be positive.
pub fn solve(expected: &[[f64; 2]], pairs: &[Pair], anchor: f64) -> Vec<[f64; 2]> {
    assert!(anchor > 0.0, "each tile is anchored");
    let [x, y] = [0, 1].map(|axis| solve_axis(expected, pairs, axis, anchor));
    x.into_iter().zip(y).map(|(x, y)| [x, y]).collect()
}

/// [`solve`], dropping the used pair that disagrees most with the solution,
/// by more than [`PlacementOptions::max_residual`], and solving again, until
/// none does. The pairs dropped are no longer `used`.
pub fn solve_consistent(
    expected: &[[f64; 2]],
    pairs: &mut [Pair],
    options: &PlacementOptions,
) -> Vec<[f64; 2]> {
    loop {
        let origins = solve(expected, pairs, options.anchor);
        let residual = |pair: &Pair| {
            let [x, y] = [0, 1].map(|axis| {
                let (a, b) = (pair.a, pair.b);
                let said = expected[b][axis] - expected[a][axis] + pair.correction[axis];
                origins[b][axis] - origins[a][axis] - said
            });
            x.hypot(y)
        };
        let worst = (pairs.iter_mut())
            .filter(|pair| pair.used)
            .map(|pair| (residual(pair), pair))
            .max_by(|a, b| a.0.total_cmp(&b.0));
        match worst {
            Some((residual, pair)) if residual > options.max_residual => pair.used = false,
            _ => return origins,
        }
    }
}

/// One axis of [`solve`], by its normal equations.
fn solve_axis(expected: &[[f64; 2]], pairs: &[Pair], axis: usize, anchor: f64) -> Vec<f64> {
    let n = expected.len();
    let mut normal = vec![0.0; n * n];
    let mut rhs: Vec<f64> = expected.iter().map(|e| anchor * e[axis]).collect();
    for i in 0..n {
        normal[i * n + i] = anchor;
    }
    for pair in pairs.iter().filter(|pair| pair.used) {
        let (a, b, w) = (pair.a, pair.b, pair.confidence);
        let difference = expected[b][axis] - expected[a][axis] + pair.correction[axis];
        normal[a * n + a] += w;
        normal[b * n + b] += w;
        normal[a * n + b] -= w;
        normal[b * n + a] -= w;
        rhs[a] -= w * difference;
        rhs[b] += w * difference;
    }
    cholesky_solve(&mut normal, rhs, n)
}

/// `x` where `matrix · x = rhs`, for a symmetric positive-definite `n` × `n`
/// matrix, which is overwritten by its Cholesky factor.
fn cholesky_solve(matrix: &mut [f64], mut rhs: Vec<f64>, n: usize) -> Vec<f64> {
    for j in 0..n {
        let diagonal = matrix[j * n + j] - (0..j).map(|k| matrix[j * n + k].powi(2)).sum::<f64>();
        let root = diagonal.max(f64::MIN_POSITIVE).sqrt();
        matrix[j * n + j] = root;
        for i in j + 1..n {
            let dot: f64 = (0..j).map(|k| matrix[i * n + k] * matrix[j * n + k]).sum();
            matrix[i * n + j] = (matrix[i * n + j] - dot) / root;
        }
    }
    for i in 0..n {
        let dot: f64 = (0..i).map(|k| matrix[i * n + k] * rhs[k]).sum();
        rhs[i] = (rhs[i] - dot) / matrix[i * n + i];
    }
    for i in (0..n).rev() {
        let dot: f64 = (i + 1..n).map(|k| matrix[k * n + i] * rhs[k]).sum();
        rhs[i] = (rhs[i] - dot) / matrix[i * n + i];
    }
    rhs
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A textured scene, as stained tissue is: smooth noise at three
    /// scales, from a fixed hash.
    fn scene(x: f64, y: f64) -> f32 {
        let lattice = |i: i64, j: i64, scale: u64| {
            let h = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                ^ (j as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
                ^ scale.wrapping_mul(0x1656_67B1_9E37_79F9);
            let h = (h ^ (h >> 31)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            (h >> 40) as f64 / (1u64 << 24) as f64
        };
        let noise = |scale: f64| {
            let (u, v) = (x / scale, y / scale);
            let (i, j) = (u.floor() as i64, v.floor() as i64);
            let (fx, fy) = (u - u.floor(), v - v.floor());
            let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
            let at = |di: i64, dj: i64| lattice(i + di, j + dj, scale as u64);
            let top = at(0, 0) + (at(1, 0) - at(0, 0)) * sx;
            let bottom = at(0, 1) + (at(1, 1) - at(0, 1)) * sx;
            top + (bottom - top) * sy
        };
        (noise(3.0) * 0.25 + noise(8.0) * 0.5 + noise(21.0)) as f32
    }

    /// The scene from `origin`, 512 × 384 pixels.
    fn tile(origin: [f64; 2]) -> Gray {
        Gray::from_fn(512, 384, |i, j| {
            scene(origin[0] + i as f64, origin[1] + j as f64)
        })
    }

    /// A 3 × 3 grid of tiles overlapping by a quarter, at the truth, and
    /// where the stage put them: `errors` further on.
    fn grid(errors: impl Fn(usize) -> [f64; 2]) -> (Vec<Gray>, Vec<[f64; 2]>, Vec<[f64; 2]>) {
        let truth: Vec<[f64; 2]> = (0..9)
            .map(|k| {
                [
                    (k % 3) as f64 * 384.0 + 1000.3,
                    (k / 3) as f64 * 288.0 + 500.6,
                ]
            })
            .collect();
        let tiles = truth.iter().map(|&origin| tile(origin)).collect();
        let expected = (truth.iter().enumerate())
            .map(|(k, t)| [t[0] + errors(k)[0], t[1] + errors(k)[1]])
            .collect();
        (tiles, expected, truth)
    }

    /// The largest distance from the truth, once the placement's common
    /// offset is taken out.
    fn residual(placed: &[[f64; 2]], truth: &[[f64; 2]]) -> f64 {
        let n = placed.len() as f64;
        let mean = |axis: usize| {
            (placed.iter().zip(truth))
                .map(|(p, t)| p[axis] - t[axis])
                .sum::<f64>()
                / n
        };
        let (mx, my) = (mean(0), mean(1));
        (placed.iter().zip(truth))
            .map(|(p, t)| (p[0] - t[0] - mx).hypot(p[1] - t[1] - my))
            .fold(0.0, f64::max)
    }

    #[test]
    fn known_shifts_are_recovered() {
        let errors = |k: usize| {
            [
                ((k * 7) % 11) as f64 * 2.5 - 12.0,
                ((k * 5) % 9) as f64 * 2.0 - 8.0,
            ]
        };
        let (tiles, expected, truth) = grid(errors);
        assert!(residual(&expected, &truth) > 10.0, "the stage is off");
        let placement = place(&tiles, &expected, &PlacementOptions::default());
        let off = residual(&placement.origins, &truth);
        assert!(off < 0.3, "{off}: {:?}", placement.origins);
        // Side by side, above and below, and at the corners.
        let used = placement.pairs.iter().filter(|pair| pair.used).count();
        assert_eq!(used, 20, "{:?}", placement.pairs);
    }

    #[test]
    fn featureless_overlaps_keep_their_stage_placement() {
        let tiles = vec![Gray::from_fn(512, 384, |_, _| 0.5); 4];
        let expected = vec![[0.0, 0.0], [384.0, 3.0], [-2.0, 288.0], [386.0, 292.0]];
        let placement = place(&tiles, &expected, &PlacementOptions::default());
        assert_eq!(placement.pairs.len(), 6);
        assert!(
            placement.pairs.iter().all(|pair| !pair.used),
            "{:?}",
            placement.pairs
        );
        assert!(placement.moves(&expected).iter().all(|&moved| moved < 1e-9));
    }

    #[test]
    fn a_correction_too_large_is_rejected() {
        // The last tile is 12 pixels from where the stage put it, beyond a
        // limit of 5.
        let errors = |k: usize| if k == 8 { [12.0, 0.0] } else { [0.0, 0.0] };
        let (tiles, expected, truth) = grid(errors);
        let options = PlacementOptions {
            max_correction: 5.0,
            ..PlacementOptions::default()
        };
        let placement = place(&tiles, &expected, &options);
        let (to_8, others): (Vec<Pair>, Vec<Pair>) =
            placement.pairs.iter().partition(|pair| pair.b == 8);
        assert!(to_8.iter().all(|pair| !pair.used), "{to_8:?}");
        assert!(others.iter().all(|pair| pair.used), "{others:?}");
        let moved = placement.moves(&expected);
        assert!(moved[8] < 1e-3, "it keeps its stage placement: {moved:?}");
        assert!(residual(&placement.origins[..8], &truth[..8]) < 0.3);
    }

    #[test]
    fn tiles_read_as_needed_are_each_read_once() {
        let (tiles, expected, truth) = grid(|k| [(k % 2) as f64 * 6.0, 0.0]);
        let mut reads = vec![0; 9];
        let mut read = |index: usize| {
            reads[index] += 1;
            Ok::<_, String>(tiles[index].clone())
        };
        let options = PlacementOptions::default();
        let placement = place_with(&expected, [512, 384], &mut read, &options).expect("placed");
        assert_eq!(reads, [1; 9]);
        assert_eq!(placement, place(&tiles, &expected, &options));
        assert!(residual(&placement.origins, &truth) < 0.3);
        // An error reading one is the placement's.
        let mut broken = |index: usize| match index {
            4 => Err("unreadable"),
            _ => Ok(tiles[index].clone()),
        };
        let placed = place_with(&expected, [512, 384], &mut broken, &options);
        assert_eq!(placed, Err("unreadable"));
    }

    #[test]
    fn tiles_too_far_apart_aren_t_paired() {
        let tiles = vec![tile([0.0, 0.0]), tile([505.0, 0.0])];
        let placement = place(
            &tiles,
            &[[0.0, 0.0], [505.0, 0.0]],
            &PlacementOptions::default(),
        );
        assert!(placement.pairs.is_empty(), "{:?}", placement.pairs);
    }

    #[test]
    fn a_sure_pair_its_tiles_other_pairs_disagree_with_is_dropped() {
        // A 3 × 3 grid, 100 pixels apart, whose pairs say the stage was right,
        // but one, as sure, that says the middle is 30 pixels right of it.
        let expected: Vec<[f64; 2]> = (0..9)
            .map(|k| [(k % 3) as f64 * 100.0, (k / 3) as f64 * 100.0])
            .collect();
        let neighbours = (0..9).flat_map(|a| [(a, a + 1), (a, a + 3)]);
        let mut pairs: Vec<Pair> = (neighbours
            .filter(|&(a, b)| b < 9 && (b != a + 1 || a % 3 != 2)))
        .map(|(a, b)| Pair {
            a,
            b,
            correction: if (a, b) == (3, 4) {
                [30.0, 0.0]
            } else {
                [0.0, 0.0]
            },
            confidence: 0.8,
            used: true,
        })
        .collect();
        assert_eq!(pairs.len(), 12);
        let placed = solve_consistent(&expected, &mut pairs, &PlacementOptions::default());
        let dropped: Vec<(usize, usize)> = (pairs.iter())
            .filter(|pair| !pair.used)
            .map(|pair| (pair.a, pair.b))
            .collect();
        assert_eq!(dropped, [(3, 4)]);
        for (placed, stage) in placed.iter().zip(&expected) {
            assert!(
                (placed[0] - stage[0]).hypot(placed[1] - stage[1]) < 1e-6,
                "{placed:?}"
            );
        }
        // Solved once, it would have moved the middle.
        let all: Vec<Pair> = pairs
            .iter()
            .map(|pair| Pair {
                used: true,
                ..*pair
            })
            .collect();
        let once = solve(&expected, &all, 1e-4);
        assert!(once[4][0] - expected[4][0] > 5.0, "{once:?}");
    }

    #[test]
    fn the_solve_is_exact_for_a_consistent_chain() {
        // Three in a row, the second 2 pixels right of and 1 above where the
        // stage put it, the third as much again; a fourth with no pair stays
        // put.
        let corrected = |a, b| Pair {
            a,
            b,
            correction: [2.0, -1.0],
            confidence: 0.8,
            used: true,
        };
        let pairs = [corrected(0, 1), corrected(1, 2)];
        let expected = [[0.0, 0.0], [100.0, 0.0], [200.0, 0.0], [500.0, 50.0]];
        let placed = solve(&expected, &pairs, 1e-6);
        let relative = |k: usize| [0, 1].map(|axis| placed[k][axis] - placed[0][axis]);
        let close = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-3;
        assert!(close(relative(1), [102.0, -1.0]), "{placed:?}");
        assert!(close(relative(2), [204.0, -2.0]), "{placed:?}");
        assert!(close(placed[3], [500.0, 50.0]), "{placed:?}");
    }
}
