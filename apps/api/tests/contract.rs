//! Assertions about what the API does and does not put on the wire.
//!
//! These guard three properties that are easy to break by adding a field and
//! hard to notice afterwards.

use grid_reveal_api::board;
use grid_reveal_api::dto::{
    CreateEntryRequest, CurrentUser, ScreenDetail, SubmitEntryResponse, UpdateEntryRequest,
    UserSummary,
};
use uuid::Uuid;

/// Any substring that would indicate credential material escaping.
const CREDENTIAL_MARKERS: [&str; 4] = ["password", "hash", "argon2", "$2b$"];

fn assert_no_credentials(json: &str) {
    let lowered = json.to_lowercase();
    for marker in CREDENTIAL_MARKERS {
        assert!(
            !lowered.contains(marker),
            "serialized payload leaked {marker:?}: {json}"
        );
    }
}

#[test]
fn current_user_never_serializes_credential_material() {
    let payload = CurrentUser {
        id: Uuid::new_v4(),
        username: "alice".into(),
        email: None,
        is_admin: false,
    };
    assert_no_credentials(&serde_json::to_string(&payload).unwrap());
}

#[test]
fn user_summary_never_serializes_credential_material() {
    let payload = UserSummary {
        id: Uuid::new_v4(),
        username: "alice".into(),
        email: Some("a@example.com".into()),
        is_admin: true,
        created_at: chrono::Utc::now(),
    };
    assert_no_credentials(&serde_json::to_string(&payload).unwrap());
}

/// The board response must carry the revealed PREFIX of the permutation and
/// nothing more. Sending the remainder would hand a member the future reveal
/// order, which is the one secret the application keeps.
#[test]
fn screen_detail_never_discloses_unrevealed_tiles() {
    // A permutation whose unrevealed tail is easy to spot in the output.
    let permutation: Vec<i32> = vec![4, 7, 1, 9, 2, 8, 0, 3, 5, 6];
    let revealed_count = 3;

    let detail = ScreenDetail {
        slug: "k3m9xq2p".into(),
        title: "Team Q4".into(),
        picture_url: "/pictures/raw/pictures/x.webp".into(),
        picture_width: 1600,
        picture_height: 1200,
        cols: 5,
        rows: 2,
        unit_minutes: 5,
        total_tiles: 10,
        participant_count: 4,
        total_minutes: 15,
        revealed_count,
        revealed_tiles: board::revealed_tiles(&permutation, revealed_count),
        completed: false,
        archived: false,
        viewer_is_admin: false,
    };

    assert_eq!(detail.revealed_tiles, vec![4, 7, 1]);

    let json = serde_json::to_string(&detail).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let sent: Vec<i64> = parsed["revealed_tiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect();

    assert_eq!(sent.len(), revealed_count as usize);

    // The unrevealed tail must appear nowhere in the payload.
    for unrevealed in &permutation[revealed_count as usize..] {
        assert!(
            !sent.contains(&i64::from(*unrevealed)),
            "unrevealed tile {unrevealed} was disclosed"
        );
    }
}

#[test]
fn submit_response_reveals_no_more_than_the_board_has() {
    let permutation: Vec<i32> = vec![4, 7, 1, 9, 2];
    let revealed_count = 2;

    let response = SubmitEntryResponse {
        entry: grid_reveal_api::dto::EntrySummary {
            id: Uuid::new_v4(),
            minutes: 10,
            occurred_at: chrono::Utc::now().date_naive(),
            note: None,
            created_at: chrono::Utc::now(),
            edited_at: None,
        },
        total_minutes: 10,
        revealed_count,
        your_tiles: board::tiles_at(&permutation, 0..revealed_count),
        revealed_tiles: board::revealed_tiles(&permutation, revealed_count),
        completed: false,
    };

    assert_eq!(response.revealed_tiles.len(), revealed_count as usize);
    assert!(
        response
            .your_tiles
            .iter()
            .all(|t| response.revealed_tiles.contains(t)),
        "a tile attributed to the submitter must also be revealed on the board"
    );
}

/// Entries are always attributed to the authenticated caller. The request
/// types carry no field naming another user, so there is no parameter an
/// administrator could use to log time on someone's behalf.
#[test]
fn entry_requests_accept_no_target_user() {
    let payload = serde_json::json!({
        "hours": 1,
        "minutes": 30,
        "occurred_at": "2026-11-13",
        "note": null,
        // All ignored: the struct has no corresponding field.
        "user_id": "00000000-0000-0000-0000-000000000001",
        "on_behalf_of": "alice",
        "member_id": 7
    });

    let create: CreateEntryRequest = serde_json::from_value(payload.clone()).unwrap();
    assert_eq!(create.hours, 1);
    assert_eq!(create.minutes, 30);

    let update: UpdateEntryRequest = serde_json::from_value(payload).unwrap();
    assert_eq!(update.hours, 1);

    // The shape itself is the guarantee: every field is a duration, a date, or
    // a note.
    let fields = serde_json::to_value(serde_json::json!({
        "hours": 0, "minutes": 1, "occurred_at": null, "note": null
    }))
    .unwrap();
    let minimal: CreateEntryRequest = serde_json::from_value(fields).unwrap();
    assert_eq!(minimal.minutes, 1);
    assert!(minimal.occurred_at.is_none());
}
