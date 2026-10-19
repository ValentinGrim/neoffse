//! Property tests of the cipher: inverses, CBC error propagation, padding.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse_format_pr::cipher::{BLOCK_SIZE, Rijndael256, UnalignedLength, pad_zeros};
use proptest::prelude::*;
use proptest::{array::uniform32, collection::vec};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn block_decrypt_inverts_encrypt(key in uniform32(any::<u8>()), block in uniform32(any::<u8>())) {
        let cipher = Rijndael256::new(&key);
        prop_assert_eq!(cipher.decrypt_block(&cipher.encrypt_block(&block)), block);
        prop_assert_eq!(cipher.encrypt_block(&cipher.decrypt_block(&block)), block);
    }

    #[test]
    fn cbc_decrypt_inverts_encrypt(
        data in vec(any::<u8>(), 0..=1100),
        iv in uniform32(any::<u8>()),
        key in uniform32(any::<u8>()),
    ) {
        let cipher = Rijndael256::new(&key);
        let mut padded = data.clone();
        pad_zeros(&mut padded);
        let expected = padded.clone();
        prop_assert_eq!(cipher.cbc_encrypt(&iv, &mut padded), Ok(()));
        prop_assert_eq!(cipher.cbc_decrypt(&iv, &mut padded), Ok(()));
        prop_assert_eq!(&padded, &expected);
        prop_assert_eq!(&padded[..data.len()], &data[..]);
        prop_assert!(padded[data.len()..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn cbc_error_stays_inside_two_blocks(
        blocks in 2_usize..=8,
        seed in vec(any::<u8>(), 8 * BLOCK_SIZE),
        key in uniform32(any::<u8>()),
        iv in uniform32(any::<u8>()),
        target in 0_usize..8,
        bit in 0_u8..8,
        offset in 0_usize..BLOCK_SIZE,
    ) {
        let target = target % blocks;
        let cipher = Rijndael256::new(&key);
        let plain = seed[..blocks * BLOCK_SIZE].to_vec();
        let mut encrypted = plain.clone();
        prop_assert_eq!(cipher.cbc_encrypt(&iv, &mut encrypted), Ok(()));
        encrypted[target * BLOCK_SIZE + offset] ^= 1 << bit;
        let mut decrypted = encrypted.clone();
        prop_assert_eq!(cipher.cbc_decrypt(&iv, &mut decrypted), Ok(()));
        for index in 0..blocks {
            let range = index * BLOCK_SIZE..(index + 1) * BLOCK_SIZE;
            if index == target {
                prop_assert_ne!(&decrypted[range.clone()], &plain[range]);
            } else if index == target + 1 {
                let mut flipped = plain[range.clone()].to_vec();
                flipped[offset] ^= 1 << bit;
                prop_assert_eq!(&decrypted[range], &flipped[..]);
            } else {
                prop_assert_eq!(&decrypted[range.clone()], &plain[range]);
            }
        }
    }

    #[test]
    fn pad_zeros_gives_a_multiple_of_32_and_adds_only_zeros(data in vec(any::<u8>(), 0..=200)) {
        let mut padded = data.clone();
        pad_zeros(&mut padded);
        prop_assert_eq!(padded.len() % 32, 0);
        prop_assert!(padded.len() >= data.len());
        prop_assert!(padded.len() < data.len() + 32);
        prop_assert_eq!(&padded[..data.len()], &data[..]);
        prop_assert!(padded[data.len()..].iter().all(|byte| *byte == 0));
        if data.len() % 32 == 0 {
            prop_assert_eq!(&padded, &data);
        }
    }

    #[test]
    fn cbc_rejects_every_unaligned_length(
        data in vec(any::<u8>(), 1..=200),
        iv in uniform32(any::<u8>()),
        key in uniform32(any::<u8>()),
    ) {
        let len = data.len();
        prop_assume!(!len.is_multiple_of(32));
        let cipher = Rijndael256::new(&key);
        let mut buffer = data.clone();
        prop_assert_eq!(cipher.cbc_encrypt(&iv, &mut buffer), Err(UnalignedLength { len }));
        prop_assert_eq!(&buffer, &data);
        prop_assert_eq!(cipher.cbc_decrypt(&iv, &mut buffer), Err(UnalignedLength { len }));
        prop_assert_eq!(&buffer, &data);
    }
}
