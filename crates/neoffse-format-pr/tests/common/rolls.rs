//! A scripted source of the draws of a level-up step. The order of the
//! draws is in the module docs of `neoffse_format_pr::ff1::Rolls`.

use std::collections::VecDeque;

use neoffse_format_pr::ff1::Rolls;

enum Answers {
    Fixed { eighth: bool, bonus: u32 },
    Queued { eighths: VecDeque<bool>, bonuses: VecDeque<u32> },
}

/// A scripted source of draws. Counts what it hands out.
pub(crate) struct Script {
    answers: Answers,
    eighths: usize,
    bonuses: usize,
}

impl Script {
    /// Answers forever: `Script::fixed(false, 20)` is "all rolls fail" and
    /// `Script::fixed(true, 24)` is "all rolls succeed".
    pub(crate) fn fixed(eighth: bool, bonus: u32) -> Script {
        Script {
            answers: Answers::Fixed { eighth, bonus },
            eighths: 0,
            bonuses: 0,
        }
    }

    /// Answers in order; a draw on an empty queue panics. `queued(&[], &[])`
    /// fails the test on any draw.
    pub(crate) fn queued(eighths: &[bool], bonuses: &[u32]) -> Script {
        Script {
            answers: Answers::Queued {
                eighths: eighths.iter().copied().collect(),
                bonuses: bonuses.iter().copied().collect(),
            },
            eighths: 0,
            bonuses: 0,
        }
    }

    /// `(one_in_eight calls, hp_bonus calls)`.
    pub(crate) fn drawn(&self) -> (usize, usize) {
        (self.eighths, self.bonuses)
    }

    /// Panics when a queue still holds an answer.
    pub(crate) fn finished(&self) {
        if let Answers::Queued { eighths, bonuses } = &self.answers {
            assert!(eighths.is_empty(), "{} one_in_eight answers unused", eighths.len());
            assert!(bonuses.is_empty(), "{} hp_bonus answers unused", bonuses.len());
        }
    }
}

impl Rolls for Script {
    fn one_in_eight(&mut self) -> bool {
        self.eighths += 1;
        match &mut self.answers {
            Answers::Fixed { eighth, .. } => *eighth,
            Answers::Queued { eighths, .. } => eighths.pop_front().expect("one_in_eight drawn on an empty queue"),
        }
    }

    fn hp_bonus(&mut self) -> u32 {
        self.bonuses += 1;
        match &mut self.answers {
            Answers::Fixed { bonus, .. } => *bonus,
            Answers::Queued { bonuses, .. } => bonuses.pop_front().expect("hp_bonus drawn on an empty queue"),
        }
    }
}
