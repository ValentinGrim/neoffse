//! The random draws of a level-up step.
//!
//! The game draws 1 to 8 for the HP roll and for each stat roll, and 20 to 24
//! for the strong HP bonus (`docs/formats/pixel-remaster.md#level-up-in-ff-i`,
//! "Random parts"). [`Rolls`] names those two draws so a caller scripts them,
//! and no `rand` type reaches the public API.
//!
//! A step draws in this order, and a scripted source depends on it:
//!
//! 1. [`Rolls::one_in_eight`], the HP roll, always.
//! 2. [`Rolls::hp_bonus`], only when the growth row forces the strong bonus
//!    (`hp_value1` is 1) or the HP roll returned `true`.
//! 3. Strength, vitality, agility, intelligence, spirit, magic, luck, in that
//!    order: one [`Rolls::one_in_eight`] per stat whose growth column is 0, none
//!    for a non-zero column.

/// The random draws of one level-up step
/// (`docs/formats/pixel-remaster.md#level-up-in-ff-i`, "Random parts").
pub trait Rolls {
    /// One draw of 1 to 8 that succeeds on 1: the HP roll and the stat rolls.
    fn one_in_eight(&mut self) -> bool;

    /// One draw of the strong HP bonus, 20 to 24 with both ends included.
    fn hp_bonus(&mut self) -> u32;
}

/// The real source: the thread-local generator of `rand`, seeded from the
/// system. The game seeds from the clock; any uniform source gives the same
/// distribution, so the seed carries no meaning here.
#[derive(Debug, Clone, Copy, Default)]
pub struct ThreadRolls;

impl Rolls for ThreadRolls {
    fn one_in_eight(&mut self) -> bool {
        rand::random_range(1..=8_u32) == 1
    }

    fn hp_bonus(&mut self) -> u32 {
        rand::random_range(20..=24_u32)
    }
}
