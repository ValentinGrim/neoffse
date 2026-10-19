//! Edge cases of the public cipher API.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse_format_pr::cipher::{BLOCK_SIZE, PR_IV, PR_KEY, Rijndael256, UnalignedLength, pad_zeros};

const KEY_HEX: &str = "61090700b9c3b8b9421599e79ca57b87bea932d3797bad7663ed4dde0a943cc5";
const IV_HEX: &str = "0224f9771fdb6e0e3bd508d7b795bf2e0cbd176942680a637b12bc6273db2ebb";

fn decode_hex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|index| u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).unwrap())
        .collect()
}

fn counting(len: usize) -> Vec<u8> {
    (0..len).map(|index| u8::try_from(index).unwrap()).collect()
}

fn counting_block() -> [u8; 32] {
    counting(32).try_into().unwrap()
}

fn padded_len(len: usize) -> usize {
    let mut data = vec![0xaa; len];
    pad_zeros(&mut data);
    data.len()
}

#[test]
fn pr_constants_match_the_documented_hex() {
    assert_eq!(PR_KEY.to_vec(), decode_hex(KEY_HEX));
    assert_eq!(PR_IV.to_vec(), decode_hex(IV_HEX));
}

#[test]
fn pr_equals_new_with_pr_key() {
    let block = counting_block();
    assert_eq!(Rijndael256::pr().encrypt_block(&block), Rijndael256::new(&PR_KEY).encrypt_block(&block));
}

#[test]
fn pad_zeros_leaves_aligned_input_alone() {
    for len in [0, 32, 64, 96] {
        assert_eq!(padded_len(len), len);
    }
}

#[test]
fn pad_zeros_rounds_up_to_the_next_multiple() {
    let lengths: Vec<usize> = [1, 31, 33, 63, 65].into_iter().map(padded_len).collect();
    assert_eq!(lengths, [32, 32, 64, 64, 96]);
}

#[test]
fn pad_zeros_appends_zeros_only() {
    let mut data = vec![1, 2, 3];
    pad_zeros(&mut data);
    let mut expected = vec![1, 2, 3];
    expected.extend([0; 29]);
    assert_eq!(data, expected);
}

#[test]
fn cbc_accepts_empty_input() {
    let cipher = Rijndael256::pr();
    let mut data: Vec<u8> = Vec::new();
    assert_eq!(cipher.cbc_encrypt(&PR_IV, &mut data), Ok(()));
    assert_eq!(data, Vec::<u8>::new());
    assert_eq!(cipher.cbc_decrypt(&PR_IV, &mut data), Ok(()));
    assert_eq!(data, Vec::<u8>::new());
}

#[test]
fn cbc_rejects_unaligned_lengths_and_leaves_data() {
    let cipher = Rijndael256::pr();
    for len in [1, 31, 33, 63, 65] {
        let original = counting(len);
        let mut data = original.clone();
        assert_eq!(cipher.cbc_encrypt(&PR_IV, &mut data), Err(UnalignedLength { len }));
        assert_eq!(data, original);
        assert_eq!(cipher.cbc_decrypt(&PR_IV, &mut data), Err(UnalignedLength { len }));
        assert_eq!(data, original);
    }
}

#[test]
fn cbc_chains_blocks() {
    let cipher = Rijndael256::pr();
    let mut data = vec![0; 2 * BLOCK_SIZE];
    cipher.cbc_encrypt(&PR_IV, &mut data).unwrap();
    let first: [u8; 32] = data[..32].try_into().unwrap();
    let second: [u8; 32] = data[32..].try_into().unwrap();
    assert_eq!(first, cipher.encrypt_block(&PR_IV));
    assert_eq!(second, cipher.encrypt_block(&first));
    assert_ne!(first, second);
}

#[test]
fn cbc_decrypt_with_another_iv_changes_only_the_first_block() {
    let cipher = Rijndael256::pr();
    let mut other_iv = PR_IV;
    other_iv[0] ^= 1;
    let mut with_pr = counting(96);
    let mut with_other = with_pr.clone();
    cipher.cbc_decrypt(&PR_IV, &mut with_pr).unwrap();
    cipher.cbc_decrypt(&other_iv, &mut with_other).unwrap();
    assert_ne!(with_pr[..32], with_other[..32]);
    assert_eq!(with_pr[32..], with_other[32..]);
}

#[test]
fn decrypt_keeps_an_all_zero_block() {
    let cipher = Rijndael256::pr();
    let mut data = vec![0; 32];
    cipher.cbc_encrypt(&PR_IV, &mut data).unwrap();
    cipher.cbc_decrypt(&PR_IV, &mut data).unwrap();
    assert_eq!(data, vec![0; 32]);
}

#[test]
fn unaligned_length_display_names_the_length() {
    let text = UnalignedLength { len: 33 }.to_string();
    assert!(text.contains("33") && text.contains("32"), "{text}");
}
