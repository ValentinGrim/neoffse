//! The embedded growth tables. They pin build 20249397: a regenerated
//! table fails these tests on purpose.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1;

#[test]
fn base_hp_of_the_six_classes() {
    let hp: Vec<Option<u32>> = (1..=6).map(|id| ff1().base_hp(id)).collect();
    assert_eq!(hp, [35, 30, 33, 30, 33, 25].map(Some));
}

#[test]
fn base_hp_of_an_unknown_id_is_none() {
    for id in [0, 7, u32::MAX] {
        assert_eq!(ff1().base_hp(id), None, "status {id}");
    }
}

#[test]
fn exp_to_reach_pinned_levels() {
    let pinned = [
        (1, 0),
        (2, 14),
        (3, 42),
        (6, 350),
        (11, 2675),
        (32, 103_811),
        (59, 601_470),
        (60, 623_220),
        (61, 644_970),
        (68, 797_217),
        (69, 818_967),
        (70, 840_716),
        (99, 1_471_455),
    ];
    for (level, total) in pinned {
        assert_eq!(ff1().exp_to_reach(level), Some(total), "level {level}");
    }
}

#[test]
fn exp_to_reach_outside_the_table_is_none() {
    for level in [0, 100, u32::MAX] {
        assert_eq!(ff1().exp_to_reach(level), None, "level {level}");
    }
}

#[test]
fn the_last_level_is_the_game_cap() {
    let cap = u32::try_from(ff1().bound("MAX_LV").unwrap()).unwrap();
    assert!(ff1().exp_to_reach(cap).is_some());
    assert_eq!(ff1().exp_to_reach(cap + 1), None);
}

#[test]
fn exp_rises_over_every_level() {
    for level in 2..=99 {
        let before = ff1().exp_to_reach(level - 1).unwrap();
        let total = ff1().exp_to_reach(level).unwrap();
        assert!(total > before, "level {level}");
    }
}
