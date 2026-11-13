//! Reveal derivation.
//!
//! The visible state of a board is a pure function of two things: the total
//! minutes logged against the screen, and the screen's fixed tile permutation.
//! Nothing about tile removal is stored, so concurrent submissions cannot race
//! and an edited or deleted entry rewinds the board correctly for free.
//!
//! ```text
//! revealed_count = min( floor(total_minutes / unit_minutes), cols * rows )
//! revealed tiles = permutation[0 .. revealed_count)
//! ```

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// The minimum fields of an entry needed to replay attribution.
#[derive(Debug, Clone)]
pub struct ReplayEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub minutes: i32,
    /// Attribution replays in creation order, never in `occurred_at` order, so
    /// that backdating an entry cannot reshuffle who revealed earlier tiles.
    pub created_at: DateTime<Utc>,
}

/// Which entry revealed one tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileAttribution {
    /// Position in the permutation, i.e. the order the tile was revealed in.
    pub position: i32,
    /// The tile's index on the grid, `row * cols + col`.
    pub tile_index: i32,
    pub entry_id: Uuid,
    pub user_id: Uuid,
}

/// How many tiles a cumulative total reveals, capped at the size of the grid.
///
/// Time logged beyond full reveal still counts toward the displayed total but
/// reveals nothing further.
pub fn revealed_count(total_minutes: i64, unit_minutes: i32, total_tiles: i32) -> i32 {
    debug_assert!(unit_minutes > 0, "unit_minutes must be positive");
    if unit_minutes <= 0 || total_tiles <= 0 || total_minutes <= 0 {
        return 0;
    }
    let units = total_minutes / i64::from(unit_minutes);
    units.clamp(0, i64::from(total_tiles)) as i32
}

/// The permutation positions a single submission reveals, given the total
/// before it landed.
///
/// Returned as a half-open range `before..after`. Empty when the submission
/// did not complete a unit, which is the common case for small entries.
pub fn positions_revealed_by(
    total_before: i64,
    minutes: i32,
    unit_minutes: i32,
    total_tiles: i32,
) -> std::ops::Range<i32> {
    let before = revealed_count(total_before, unit_minutes, total_tiles);
    let after = revealed_count(total_before + i64::from(minutes), unit_minutes, total_tiles);
    before..after
}

/// Maps permutation positions to grid tile indices.
///
/// Positions beyond the permutation's length are ignored rather than panicking,
/// so a board whose geometry changed mid-flight degrades instead of crashing.
pub fn tiles_at(permutation: &[i32], positions: std::ops::Range<i32>) -> Vec<i32> {
    positions
        .filter_map(|p| usize::try_from(p).ok())
        .filter_map(|p| permutation.get(p).copied())
        .collect()
}

/// The revealed prefix of the permutation. This is the only part of the reveal
/// order ever sent to a client; disclosing the remainder would spoil the
/// surprise the whole application exists to produce.
pub fn revealed_tiles(permutation: &[i32], revealed: i32) -> Vec<i32> {
    let n = usize::try_from(revealed).unwrap_or(0).min(permutation.len());
    permutation[..n].to_vec()
}

/// Replays every live entry in creation order to determine which entry revealed
/// each tile.
///
/// `entries` must already be sorted by `created_at` ascending and must exclude
/// soft-deleted rows.
pub fn attribute(
    entries: &[ReplayEntry],
    permutation: &[i32],
    unit_minutes: i32,
    total_tiles: i32,
) -> Vec<TileAttribution> {
    let mut out = Vec::new();
    let mut running: i64 = 0;

    for entry in entries {
        let range = positions_revealed_by(running, entry.minutes, unit_minutes, total_tiles);
        for position in range {
            let Some(tile_index) = usize::try_from(position)
                .ok()
                .and_then(|p| permutation.get(p).copied())
            else {
                continue;
            };
            out.push(TileAttribution {
                position,
                tile_index,
                entry_id: entry.id,
                user_id: entry.user_id,
            });
        }
        running += i64::from(entry.minutes);
    }

    out
}

/// Total live minutes across a set of entries.
pub fn total_minutes(entries: &[ReplayEntry]) -> i64 {
    entries.iter().map(|e| i64::from(e.minutes)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn entry(minutes: i32, seq: u32) -> ReplayEntry {
        ReplayEntry {
            id: Uuid::from_u128(u128::from(seq)),
            user_id: Uuid::from_u128(u128::from(seq) + 1000),
            minutes,
            created_at: Utc.timestamp_opt(1_700_000_000 + i64::from(seq), 0).unwrap(),
        }
    }

    fn identity_permutation(n: i32) -> Vec<i32> {
        (0..n).collect()
    }

    // The worked example from the design document.
    #[test]
    fn worked_sequence_at_unit_five() {
        let unit = 5;
        let tiles = 100;

        // A logs 13 minutes: two tiles, three minutes carried.
        assert_eq!(positions_revealed_by(0, 13, unit, tiles), 0..2);
        // B logs 2 minutes, completing the carried unit: one tile.
        assert_eq!(positions_revealed_by(13, 2, unit, tiles), 2..3);
        // C logs 4 minutes: not enough to complete another unit.
        assert_eq!(positions_revealed_by(15, 4, unit, tiles), 3..3);
        // A logs 1 more minute, reaching 20: one tile.
        assert_eq!(positions_revealed_by(19, 1, unit, tiles), 3..4);
    }

    #[test]
    fn attribution_follows_the_worked_sequence() {
        let unit = 5;
        let tiles = 100;
        let perm = identity_permutation(tiles);
        let entries = vec![entry(13, 1), entry(2, 2), entry(4, 3), entry(1, 4)];

        let attributed = attribute(&entries, &perm, unit, tiles);
        let owners: Vec<(i32, Uuid)> = attributed
            .iter()
            .map(|a| (a.position, a.user_id))
            .collect();

        let a = entries[0].user_id;
        let b = entries[1].user_id;
        let a2 = entries[3].user_id;

        assert_eq!(
            owners,
            vec![(0, a), (1, a), (2, b), (3, a2)],
            "tiles 0 and 1 belong to the 13-minute entry, tile 2 to the entry \
             that completed the carried unit, tile 3 to the entry reaching 20"
        );
    }

    #[test]
    fn partial_unit_reveals_nothing() {
        assert_eq!(revealed_count(13, 5, 100), 2);
        assert_eq!(revealed_count(4, 5, 100), 0);
        assert_eq!(revealed_count(0, 5, 100), 0);
    }

    #[test]
    fn revealed_count_is_capped_at_the_grid_size() {
        // Twelve tiles' worth of time on a nine-tile grid.
        assert_eq!(revealed_count(60, 5, 9), 9);
        // Overflow stays capped no matter how much is logged.
        assert_eq!(revealed_count(100_000, 5, 9), 9);
    }

    #[test]
    fn overflow_submission_reveals_only_remaining_tiles() {
        // Grid of 4 tiles, 3 already revealed, a 60-minute entry arrives.
        let range = positions_revealed_by(15, 60, 5, 4);
        assert_eq!(range, 3..4, "only the one remaining tile is revealed");
    }

    #[test]
    fn deleting_an_entry_rewinds_the_board() {
        let unit = 5;
        let tiles = 100;
        let with_entry = vec![entry(13, 1), entry(10, 2)];
        let without = vec![entry(13, 1)];

        assert_eq!(revealed_count(total_minutes(&with_entry), unit, tiles), 4);
        assert_eq!(revealed_count(total_minutes(&without), unit, tiles), 2);
    }

    #[test]
    fn backdating_does_not_reshuffle_attribution() {
        let unit = 5;
        let tiles = 100;
        let perm = identity_permutation(tiles);

        let before = vec![entry(10, 1), entry(10, 2)];
        let baseline = attribute(&before, &perm, unit, tiles);

        // A third entry is created later but dated earlier. Because replay
        // orders by created_at, it appends and leaves earlier tiles alone.
        let mut after = before.clone();
        after.push(entry(10, 3));
        let replayed = attribute(&after, &perm, unit, tiles);

        assert_eq!(
            baseline,
            replayed[..baseline.len()],
            "previously revealed tiles keep their owners"
        );
        assert_eq!(replayed.len(), 6);
    }

    #[test]
    fn attribution_respects_the_permutation_order() {
        let unit = 5;
        let perm = vec![7, 3, 9, 1];
        let entries = vec![entry(10, 1)];

        let attributed = attribute(&entries, &perm, unit, 4);
        let tiles: Vec<i32> = attributed.iter().map(|a| a.tile_index).collect();
        assert_eq!(tiles, vec![7, 3], "tiles come from the permutation, not in index order");
    }

    #[test]
    fn revealed_tiles_returns_only_the_prefix() {
        let perm = vec![7, 3, 9, 1];
        assert_eq!(revealed_tiles(&perm, 2), vec![7, 3]);
        assert_eq!(revealed_tiles(&perm, 0), Vec::<i32>::new());
        // A count beyond the permutation clamps rather than panicking.
        assert_eq!(revealed_tiles(&perm, 99), vec![7, 3, 9, 1]);
    }

    #[test]
    fn zero_and_negative_inputs_are_safe() {
        assert_eq!(revealed_count(-5, 5, 10), 0);
        assert_eq!(revealed_count(10, 5, 0), 0);
    }
}
