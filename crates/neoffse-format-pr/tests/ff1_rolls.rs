//! The real random source of a level-up step
//! (`docs/formats/pixel-remaster.md#level-up-in-ff-i`, "Random parts").

use neoffse_format_pr::ff1::{Rolls, ThreadRolls};

#[test]
fn rolls_is_object_safe() {
    let rolls: &mut dyn Rolls = &mut ThreadRolls;
    let _ = rolls.one_in_eight();
    assert!((20..=24).contains(&rolls.hp_bonus()));
}

#[test]
fn the_bonus_stays_in_range_and_reaches_every_value() {
    let mut rolls = ThreadRolls;
    let mut seen = [false; 5];
    for _ in 0..10_000 {
        let bonus = rolls.hp_bonus();
        assert!((20..=24).contains(&bonus), "bonus {bonus} outside 20 to 24");
        let index = usize::try_from(bonus - 20).unwrap_or(0);
        seen[index] = true;
    }
    assert!(seen.iter().all(|hit| *hit), "a value never appeared: {seen:?}");
}

#[test]
fn one_in_eight_succeeds_about_one_time_in_eight() {
    let mut rolls = ThreadRolls;
    let successes = (0..80_000).filter(|_| rolls.one_in_eight()).count();
    // Mean 10 000, standard deviation 93.5: the bounds sit at 7.5 sigma.
    assert!((9_300..=10_700).contains(&successes), "{successes} successes in 80 000 draws");
}

#[test]
fn thread_rolls_is_copy_default_and_send() {
    fn needs_traits<T: Copy + Default + Send>() {}
    needs_traits::<ThreadRolls>();
}
