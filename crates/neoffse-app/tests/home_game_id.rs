//! The Home rows carry the id of their game.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Site, all_rows, scanned};
use neoffse::pr::games::GAMES;

#[test]
fn game_rows_carry_their_game_id() {
    let site = Site::new();
    let rows = all_rows(&scanned(&site));
    let ids: Vec<String> = rows.iter().map(|row| row.id.to_string()).collect();
    let expected: Vec<&str> = GAMES.iter().map(|game| game.id).collect();
    assert_eq!(ids, expected);
}
