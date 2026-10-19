//! Rijndael tables, built at compile time from the field arithmetic of
//! GF(2^8) with the reduction polynomial 0x11b. No numeric table is
//! embedded: each entry follows from the code below.
//!
//! Index casts from `u8` to `usize` are lossless; `usize::from` is not
//! `const`, so the `const fn` bodies use `as`.

/// Multiplies by `x` in GF(2^8).
pub(super) const fn xtime(value: u8) -> u8 {
    (value << 1) ^ if value & 0x80 != 0 { 0x1b } else { 0 }
}

/// Shift-and-add product in GF(2^8).
const fn gmul(mut left: u8, mut right: u8) -> u8 {
    let mut product = 0;
    while right != 0 {
        if right & 1 != 0 {
            product ^= left;
        }
        left = xtime(left);
        right >>= 1;
    }
    product
}

/// Multiplicative inverse as `a^254`; `0` maps to `0`.
const fn ginv(value: u8) -> u8 {
    let mut result = 1;
    let mut base = value;
    let mut exponent = 254_u8;
    while exponent != 0 {
        if exponent & 1 != 0 {
            result = gmul(result, base);
        }
        base = gmul(base, base);
        exponent >>= 1;
    }
    result
}

const fn build_sbox() -> [u8; 256] {
    let mut sbox = [0; 256];
    let mut input = 0_u8;
    loop {
        let inverse = ginv(input);
        sbox[input as usize] = inverse ^ inverse.rotate_left(1) ^ inverse.rotate_left(2) ^ inverse.rotate_left(3) ^ inverse.rotate_left(4) ^ 0x63;
        if input == u8::MAX {
            return sbox;
        }
        input += 1;
    }
}

const fn build_inverse_sbox(sbox: &[u8; 256]) -> [u8; 256] {
    let mut inverse = [0; 256];
    let mut input = 0_u8;
    loop {
        inverse[sbox[input as usize] as usize] = input;
        if input == u8::MAX {
            return inverse;
        }
        input += 1;
    }
}

/// Encrypt table: column `(02, 01, 01, 03) * S[x]`, rotated right by
/// `rotation` bits.
const fn build_encrypt_table(sbox: &[u8; 256], rotation: u32) -> [u32; 256] {
    let mut table = [0; 256];
    let mut input = 0_u8;
    loop {
        let substituted = sbox[input as usize];
        let doubled = xtime(substituted);
        let column = u32::from_be_bytes([doubled, substituted, substituted, doubled ^ substituted]);
        table[input as usize] = column.rotate_right(rotation);
        if input == u8::MAX {
            return table;
        }
        input += 1;
    }
}

/// Decrypt table: column `(0e, 09, 0d, 0b) * SI[x]`, rotated right by
/// `rotation` bits.
const fn build_decrypt_table(inverse_sbox: &[u8; 256], rotation: u32) -> [u32; 256] {
    let mut table = [0; 256];
    let mut input = 0_u8;
    loop {
        let substituted = inverse_sbox[input as usize];
        let column = u32::from_be_bytes([
            gmul(substituted, 0x0e),
            gmul(substituted, 0x09),
            gmul(substituted, 0x0d),
            gmul(substituted, 0x0b),
        ]);
        table[input as usize] = column.rotate_right(rotation);
        if input == u8::MAX {
            return table;
        }
        input += 1;
    }
}

// Each static copies a private const: a static must not read another
// static in its initializer.
const S: [u8; 256] = build_sbox();
const SI: [u8; 256] = build_inverse_sbox(&S);

pub(super) static SBOX: [u8; 256] = S;
pub(super) static INVERSE_SBOX: [u8; 256] = SI;

pub(super) static T1: [u32; 256] = build_encrypt_table(&S, 0);
pub(super) static T2: [u32; 256] = build_encrypt_table(&S, 8);
pub(super) static T3: [u32; 256] = build_encrypt_table(&S, 16);
pub(super) static T4: [u32; 256] = build_encrypt_table(&S, 24);
pub(super) static T5: [u32; 256] = build_decrypt_table(&SI, 0);
pub(super) static T6: [u32; 256] = build_decrypt_table(&SI, 8);
pub(super) static T7: [u32; 256] = build_decrypt_table(&SI, 16);
pub(super) static T8: [u32; 256] = build_decrypt_table(&SI, 24);
