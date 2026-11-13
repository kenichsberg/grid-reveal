//! Grid geometry.
//!
//! Square cells, a picture that fills the viewport, and an arbitrary picture
//! aspect ratio cannot all hold at once. The resolution is to letterbox the
//! picture and overlay the grid on exactly its area: cells are then exactly
//! square when `cols / rows` equals the picture's aspect ratio, and near-square
//! otherwise. So the job here is to pick the `cols x rows` closest in ratio to
//! the picture while landing near the administrator's requested tile count.
//!
//! A consequence worth stating plainly: the target total time is snapped, not
//! honoured exactly. 84 tiles does not factor into a rectangle resembling a 3:2
//! photograph, so the administrator is shown the exact achievable total before
//! they commit.

/// A solved grid and what it implies for the screen's total time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    pub cols: i32,
    pub rows: i32,
}

impl Geometry {
    pub fn total_tiles(&self) -> i32 {
        self.cols * self.rows
    }

    /// The exact total time this grid represents, which may differ from the
    /// target that produced it.
    pub fn exact_total_minutes(&self, unit_minutes: i32) -> i64 {
        i64::from(self.total_tiles()) * i64::from(unit_minutes)
    }

    pub fn aspect(&self) -> f64 {
        f64::from(self.cols) / f64::from(self.rows)
    }
}

/// Largest grid we will solve for. Beyond this the canvas renderer stops being
/// comfortable and the tiles stop being individually meaningful.
pub const MAX_TILES: i32 = 20_000;
pub const MAX_DIMENSION: i32 = 500;

/// Tiles implied by a target total and a unit, clamped to something sane.
pub fn desired_tiles(target_minutes: i64, unit_minutes: i32) -> i32 {
    if unit_minutes <= 0 || target_minutes <= 0 {
        return 1;
    }
    let raw = (target_minutes as f64 / f64::from(unit_minutes)).round() as i64;
    raw.clamp(1, i64::from(MAX_TILES)) as i32
}

/// Chooses the grid whose ratio best matches the picture while staying close to
/// the requested tile count.
///
/// `aspect` is the picture's width divided by its height.
pub fn solve(target_minutes: i64, unit_minutes: i32, aspect: f64) -> Geometry {
    let desired = desired_tiles(target_minutes, unit_minutes);
    solve_for_tiles(desired, aspect)
}

/// As [`solve`], but working directly from a tile count.
pub fn solve_for_tiles(desired: i32, aspect: f64) -> Geometry {
    let aspect = sanitize_aspect(aspect);
    let desired = desired.clamp(1, MAX_TILES);

    // Ideal real-valued solution: cols * rows == desired and cols / rows == aspect.
    let ideal_cols = (f64::from(desired) * aspect).sqrt();

    // Search a window around the ideal rather than the whole range; the score
    // is unimodal enough that a local search finds the optimum.
    let lo = ((ideal_cols * 0.4).floor() as i32).max(1);
    let hi = ((ideal_cols * 2.5).ceil() as i32)
        .max(lo + 1)
        .min(MAX_DIMENSION);

    let mut best: Option<(f64, Geometry)> = None;

    for cols in lo..=hi {
        let rows = (f64::from(desired) / f64::from(cols)).round() as i32;
        let rows = rows.clamp(1, MAX_DIMENSION);
        let candidate = Geometry { cols, rows };
        if candidate.total_tiles() > MAX_TILES {
            continue;
        }
        let score = score(candidate, desired, aspect);
        if best.as_ref().is_none_or(|(b, _)| score < *b) {
            best = Some((score, candidate));
        }
    }

    best.map(|(_, g)| g).unwrap_or(Geometry { cols: 1, rows: 1 })
}

/// Solves for the row count when the administrator has fixed the columns.
///
/// Used by the live preview as they adjust the column slider.
pub fn solve_with_cols(target_minutes: i64, unit_minutes: i32, cols: i32) -> Geometry {
    let desired = desired_tiles(target_minutes, unit_minutes);
    let cols = cols.clamp(1, MAX_DIMENSION);
    let rows = ((f64::from(desired) / f64::from(cols)).round() as i32).clamp(1, MAX_DIMENSION);
    Geometry { cols, rows }
}

/// Lower is better. Combines how far the grid's ratio is from the picture's
/// with how far its tile count is from the request.
///
/// The ratio term uses a log so that being twice as wide and half as wide are
/// penalised equally.
fn score(candidate: Geometry, desired: i32, aspect: f64) -> f64 {
    let ratio_error = (candidate.aspect() / aspect).ln().abs();
    let count_error =
        (f64::from(candidate.total_tiles()) - f64::from(desired)).abs() / f64::from(desired.max(1));

    // Weighted toward the ratio: a grid a few tiles off the target is barely
    // noticeable, whereas visibly non-square cells are noticeable immediately.
    ratio_error * 2.0 + count_error
}

/// Guards against degenerate or absent picture dimensions.
fn sanitize_aspect(aspect: f64) -> f64 {
    if aspect.is_finite() && aspect > 0.0 {
        aspect.clamp(0.05, 20.0)
    } else {
        1.0
    }
}

/// How far the solved cells deviate from square, as a ratio where 1.0 is exact.
///
/// Surfaced in the creation preview so an administrator can see the trade.
pub fn cell_squareness(geometry: Geometry, aspect: f64) -> f64 {
    let aspect = sanitize_aspect(aspect);
    let cell_aspect = geometry.aspect() / aspect;
    if cell_aspect >= 1.0 {
        cell_aspect
    } else {
        1.0 / cell_aspect
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_factorisation_is_found_when_one_exists() {
        // 48 tiles against a 4:3 picture has an exact 8x6 answer.
        let g = solve_for_tiles(48, 4.0 / 3.0);
        assert_eq!(g, Geometry { cols: 8, rows: 6 });
        assert!((cell_squareness(g, 4.0 / 3.0) - 1.0).abs() < 1e-9, "cells are exactly square");
    }

    #[test]
    fn sixteen_by_nine_picture_yields_a_wide_grid() {
        let g = solve_for_tiles(144, 16.0 / 9.0);
        assert_eq!(g.total_tiles(), 144);
        assert!(g.cols > g.rows, "a wide picture produces a wide grid");
        assert!(cell_squareness(g, 16.0 / 9.0) < 1.05);
    }

    #[test]
    fn portrait_picture_yields_a_tall_grid() {
        let g = solve_for_tiles(60, 2.0 / 3.0);
        assert!(g.rows > g.cols, "a tall picture produces a tall grid");
    }

    #[test]
    fn prime_tile_count_snaps_to_a_usable_rectangle() {
        // 83 is prime: an exact grid would be 83x1, which is useless. The
        // solver is expected to trade a few tiles for a sane shape.
        let aspect = 3.0 / 2.0;
        let g = solve_for_tiles(83, aspect);
        assert!(g.cols > 3 && g.rows > 3, "got {g:?}");
        assert!(cell_squareness(g, aspect) < 1.3, "cells stay close to square");
        assert!((g.total_tiles() - 83).abs() <= 15, "stays near the request");
    }

    #[test]
    fn target_and_unit_produce_the_exact_total() {
        // 4 hours at 5 minutes a tile is 48 tiles.
        let g = solve(240, 5, 4.0 / 3.0);
        assert_eq!(g.total_tiles(), 48);
        assert_eq!(g.exact_total_minutes(5), 240);
    }

    #[test]
    fn exact_total_can_differ_from_the_target() {
        let target = 415; // 83 tiles at 5 minutes, a prime count
        let g = solve(target, 5, 3.0 / 2.0);
        let exact = g.exact_total_minutes(5);
        assert_ne!(exact, target, "the snapped total is disclosed, not the target");
        assert!((exact - target).abs() < 100);
    }

    #[test]
    fn fixing_columns_recomputes_rows() {
        // 48 tiles requested, columns pinned to 12.
        let g = solve_with_cols(240, 5, 12);
        assert_eq!(g.cols, 12);
        assert_eq!(g.rows, 4);
        assert_eq!(g.exact_total_minutes(5), 240);
    }

    #[test]
    fn single_tile_is_valid() {
        let g = solve(5, 5, 1.0);
        assert_eq!(g.total_tiles(), 1);
    }

    #[test]
    fn degenerate_aspect_does_not_panic() {
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let g = solve_for_tiles(100, bad);
            assert!(g.cols >= 1 && g.rows >= 1);
        }
    }

    #[test]
    fn tile_count_is_clamped() {
        let g = solve(i64::MAX, 1, 1.0);
        assert!(g.total_tiles() <= MAX_TILES);
        assert!(g.cols <= MAX_DIMENSION && g.rows <= MAX_DIMENSION);
    }

    #[test]
    fn zero_target_still_produces_a_grid() {
        assert_eq!(desired_tiles(0, 5), 1);
        assert_eq!(desired_tiles(100, 0), 1);
    }
}
