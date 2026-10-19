//! Key schedule and the raw 32-byte block operations.

use super::tables::{INVERSE_SBOX, SBOX, T1, T2, T3, T4, T5, T6, T7, T8, xtime};
use super::{BLOCK_SIZE, PR_KEY};

/// Words per block and per round key: 256 bits.
const COLUMNS: usize = BLOCK_SIZE / 4;
/// Round keys: 14 rounds plus the initial key addition.
const ROUND_KEYS: usize = 15;

type Columns = [u32; COLUMNS];
type Lookup = [&'static [u32; 256]; 4];

/// Row shifts for a 256-bit block, `shifts[2]` of the Go reference.
const ENCRYPT_SHIFTS: [usize; 3] = [1, 3, 4];
const DECRYPT_SHIFTS: [usize; 3] = [7, 5, 4];

/// Rijndael with a 256-bit block and a 256-bit key (14 rounds).
///
/// Holds the encryption and decryption key schedules. It is not AES: AES
/// fixes the block at 128 bits. `encrypt_block` and `decrypt_block` work on
/// one block with no chaining; `cbc_encrypt` and `cbc_decrypt` chain blocks.
#[derive(Debug, Clone)]
pub struct Rijndael256 {
    pub(super) ke: [Columns; ROUND_KEYS],
    pub(super) kd: [Columns; ROUND_KEYS],
}

impl Rijndael256 {
    /// Expands `key` into the encryption and decryption schedules.
    #[must_use]
    pub fn new(key: &[u8; 32]) -> Self {
        let ke = expand_key(key);
        let kd = core::array::from_fn(|round| match round {
            0 => ke[ROUND_KEYS - 1],
            last if last == ROUND_KEYS - 1 => ke[0],
            middle => inverse_mix_key(&ke[ROUND_KEYS - 1 - middle]),
        });
        Self { ke, kd }
    }

    /// The cipher keyed with [`PR_KEY`](super::PR_KEY).
    #[must_use]
    pub fn pr() -> Self {
        Self::new(&PR_KEY)
    }

    /// Encrypts one block: no IV, no chaining, no padding.
    #[must_use]
    pub fn encrypt_block(&self, block: &[u8; 32]) -> [u8; 32] {
        crypt(&self.ke, block, ENCRYPT_SHIFTS, [&T1, &T2, &T3, &T4], &SBOX)
    }

    /// Decrypts one block: no IV, no chaining, no padding.
    #[must_use]
    pub fn decrypt_block(&self, block: &[u8; 32]) -> [u8; 32] {
        crypt(&self.kd, block, DECRYPT_SHIFTS, [&T5, &T6, &T7, &T8], &INVERSE_SBOX)
    }
}

/// Byte `N` of a word, `0` being the most significant, widened for table
/// lookup. A table has 256 entries, so a `u8` index never overflows it.
fn byte<const N: usize>(word: u32) -> usize {
    usize::from(word.to_be_bytes()[N])
}

fn sub_word(word: u32) -> u32 {
    u32::from_be_bytes(word.to_be_bytes().map(|byte| SBOX[usize::from(byte)]))
}

/// Standard Rijndael expansion for Nk = 8 key words, Nb = 8 block words.
fn expand_key(key: &[u8; 32]) -> [Columns; ROUND_KEYS] {
    let mut words = [0_u32; COLUMNS * ROUND_KEYS];
    for (slot, bytes) in words.iter_mut().zip(key.as_chunks::<4>().0) {
        *slot = u32::from_be_bytes(*bytes);
    }
    let mut round_constant = 0x01_u8;
    for index in COLUMNS..words.len() {
        let mut word = words[index - 1];
        if index % COLUMNS == 0 {
            word = sub_word(word.rotate_left(8)) ^ (u32::from(round_constant) << 24);
            round_constant = xtime(round_constant);
        } else if index % COLUMNS == 4 {
            word = sub_word(word);
        }
        words[index] = words[index - COLUMNS] ^ word;
    }
    core::array::from_fn(|round| core::array::from_fn(|column| words[round * COLUMNS + column]))
}

/// `InvMixColumns` of a round key, for the equivalent inverse cipher.
/// `T5[S[b]]` is the column `(0e, 09, 0d, 0b) * b`.
fn inverse_mix_key(round_key: &Columns) -> Columns {
    round_key.map(|word| T5[byte::<0>(sub_word(word))] ^ T6[byte::<1>(sub_word(word))] ^ T7[byte::<2>(sub_word(word))] ^ T8[byte::<3>(sub_word(word))])
}

fn load(block: &[u8; 32]) -> Columns {
    let mut columns = [0; COLUMNS];
    for (column, bytes) in columns.iter_mut().zip(block.as_chunks::<4>().0) {
        *column = u32::from_be_bytes(*bytes);
    }
    columns
}

fn store(columns: &Columns) -> [u8; 32] {
    let mut block = [0; 32];
    for (bytes, column) in block.as_chunks_mut::<4>().0.iter_mut().zip(columns) {
        *bytes = column.to_be_bytes();
    }
    block
}

/// One full round: each column reads one byte from four columns,
/// offset by the row shifts. Every `% COLUMNS` index stays inside the
/// eight-word array.
fn round(state: &Columns, round_key: &Columns, row_shifts: [usize; 3], tables: &Lookup) -> Columns {
    let [second_row, third_row, fourth_row] = row_shifts;
    let [first, second, third, fourth] = tables;
    core::array::from_fn(|column| {
        first[byte::<0>(state[column])]
            ^ second[byte::<1>(state[(column + second_row) % COLUMNS])]
            ^ third[byte::<2>(state[(column + third_row) % COLUMNS])]
            ^ fourth[byte::<3>(state[(column + fourth_row) % COLUMNS])]
            ^ round_key[column]
    })
}

/// The last round skips the column mix and substitutes bytes directly.
fn last_round(state: &Columns, round_key: &Columns, row_shifts: [usize; 3], sbox: &[u8; 256]) -> Columns {
    let [second_row, third_row, fourth_row] = row_shifts;
    core::array::from_fn(|column| {
        u32::from_be_bytes([
            sbox[byte::<0>(state[column])],
            sbox[byte::<1>(state[(column + second_row) % COLUMNS])],
            sbox[byte::<2>(state[(column + third_row) % COLUMNS])],
            sbox[byte::<3>(state[(column + fourth_row) % COLUMNS])],
        ]) ^ round_key[column]
    })
}

/// Shared by both directions: 13 full rounds and the last round.
/// Not constant time: table lookups depend on the data (see the module
/// documentation).
fn crypt(keys: &[Columns; ROUND_KEYS], block: &[u8; 32], row_shifts: [usize; 3], tables: Lookup, sbox: &[u8; 256]) -> [u8; 32] {
    let mut state = load(block);
    for (word, key_word) in state.iter_mut().zip(&keys[0]) {
        *word ^= key_word;
    }
    for round_key in &keys[1..ROUND_KEYS - 1] {
        state = round(&state, round_key, row_shifts, &tables);
    }
    store(&last_round(&state, &keys[ROUND_KEYS - 1], row_shifts, sbox))
}
