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

#[test]
fn base_vitality_of_the_six_classes() {
    let vitality: Vec<Option<u32>> = (1..=6).map(|id| ff1().base_vitality(id)).collect();
    assert_eq!(vitality, [15, 5, 10, 5, 8, 2].map(Some));
    for id in [0, 7, u32::MAX] {
        assert_eq!(ff1().base_vitality(id), None, "status {id}");
    }
}

/// One pinned row: group, level, `hp_value1`, stats, accuracy, evasion, charges.
type Pinned = (u32, u32, bool, [u32; 7], u32, u32, [u32; 8]);

#[test]
fn growth_step_pinned_rows() {
    let rows: [Pinned; 12] = [
        (1, 2, true, [1, 0, 0, 0, 0, 0, 0], 3, 2, [0; 8]),
        (1, 3, false, [0, 1, 1, 0, 0, 0, 0], 3, 2, [0; 8]),
        (7, 2, true, [1, 0, 0, 0, 0, 0, 0], 4, 3, [1, 1, 1, 0, 0, 0, 0, 0]),
        (7, 3, false, [0, 1, 1, 0, 0, 0, 0], 4, 3, [0; 8]),
        (7, 32, true, [1, 0, 0, 0, 0, 0, 0], 4, 3, [1, 1, 1, 0, 0, 0, 0, 0]),
        (7, 33, false, [0, 1, 1, 0, 0, 0, 0], 4, 3, [0; 8]),
        (7, 34, false, [1, 0, 0, 0, 0, 0, 1], 4, 3, [1, 1, 1, 0, 0, 0, 0, 0]),
        (8, 33, false, [0, 0, 1, 0, 0, 0, 0], 5, 3, [0; 8]),
        (8, 34, true, [1, 0, 0, 0, 0, 0, 1], 5, 3, [1, 1, 1, 1, 0, 0, 0, 0]),
        (10, 2, false, [1, 0, 0, 0, 0, 0, 0], 3, 3, [1, 1, 0, 0, 0, 0, 0, 0]),
        (10, 3, false, [0, 0, 1, 0, 0, 0, 0], 3, 3, [1, 2, 0, 0, 0, 0, 0, 0]),
        (12, 99, false, [0, 1, 0, 0, 0, 0, 0], 2, 5, [7, 9, 9, 9, 9, 9, 9, 9]),
    ];
    for (group, level, strong_hp, stats, accuracy_rate, magic_evasion_rate, charges) in rows {
        let step = ff1().growth_step(group, level).unwrap();
        let label = format!("group {group} level {level}");
        assert_eq!(step.strong_hp, strong_hp, "{label}");
        assert_eq!(step.stats, stats, "{label}");
        assert_eq!(step.accuracy_rate, accuracy_rate, "{label}");
        assert_eq!(step.magic_evasion_rate, magic_evasion_rate, "{label}");
        assert_eq!(step.charges, charges, "{label}");
    }
}

#[test]
fn a_job_reads_its_own_group() {
    // A promoted job does not read its base group: job 10 differs from job 4.
    let pairs = [(1, (3, 2)), (7, (4, 3)), (8, (5, 3)), (10, (3, 3))];
    for (job, expected) in pairs {
        let step = ff1().growth_step(job, 2).unwrap();
        assert_eq!((step.accuracy_rate, step.magic_evasion_rate), expected, "job {job}");
    }
}

#[test]
fn growth_step_outside_the_table_is_none() {
    for job in [0, 13, u32::MAX] {
        assert_eq!(ff1().growth_step(job, 2), None, "job {job}");
    }
    for level in [0, 1, 100, u32::MAX] {
        assert_eq!(ff1().growth_step(7, level), None, "level {level}");
    }
}

#[test]
fn every_job_has_rows_from_2_to_the_cap() {
    let cap = u32::try_from(ff1().bound("MAX_LV").unwrap()).unwrap();
    for job in 1..=12 {
        for level in 2..=cap {
            assert!(ff1().growth_step(job, level).is_some(), "job {job} level {level}");
        }
    }
}

fn every_row() -> impl Iterator<Item = (u32, u32, neoffse_data::GrowthStep)> {
    (1..=12).flat_map(|job| (2..=99).map(move |level| (job, level, ff1().growth_step(job, level).unwrap())))
}

#[test]
fn the_strong_hp_flag_sits_on_210_rows() {
    assert_eq!(every_row().filter(|(_, _, step)| step.strong_hp).count(), 210);
}

#[test]
fn spirit_and_magic_are_zero_on_every_row() {
    for (job, level, step) in every_row() {
        // The rule's order is strength, vitality, agility, intelligence, spirit, magic, luck.
        assert_eq!((step.stats[4], step.stats[5]), (0, 0), "job {job} level {level}");
    }
}

#[test]
fn promoted_groups_carry_the_flags_of_their_base_groups() {
    for base in 1..=6 {
        for level in 2..=99 {
            let low = ff1().growth_step(base, level).unwrap();
            let high = ff1().growth_step(base + 6, level).unwrap();
            assert_eq!(low.strong_hp, high.strong_hp, "jobs {base} and {} level {level}", base + 6);
            assert_eq!(low.stats, high.stats, "jobs {base} and {} level {level}", base + 6);
        }
    }
}

#[test]
fn charges_stay_within_the_game_cap() {
    let cap = u32::try_from(ff1().bound("MAX_MAGICAL_TIMES").unwrap()).unwrap();
    for (job, level, step) in every_row() {
        assert!(step.charges.iter().all(|value| *value <= cap), "job {job} level {level}");
    }
}
