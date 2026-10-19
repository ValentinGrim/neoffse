//! Checks the cipher against the vectors generated from `KiameV`'s Go library.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse_format_pr::cipher::{PR_IV, Rijndael256, pad_zeros};

const VECTORS: &str = include_str!("data/rijndael256-vectors.txt");

enum Record {
    Block { plain: [u8; 32], cipher: [u8; 32] },
    Encrypt { plain: Vec<u8>, cipher: Vec<u8> },
    Decrypt { returned: usize, cipher: Vec<u8>, plain: Vec<u8> },
}

fn hex(field: &str, line: usize) -> Vec<u8> {
    assert!(field.len().is_multiple_of(2), "line {line}: odd hex length");
    (0..field.len() / 2)
        .map(|index| u8::from_str_radix(&field[index * 2..index * 2 + 2], 16).unwrap_or_else(|_| panic!("line {line}: non-hex field")))
        .collect()
}

fn block(field: &str, line: usize) -> [u8; 32] {
    hex(field, line).try_into().unwrap_or_else(|_| panic!("line {line}: block is not 32 bytes"))
}

fn parse() -> Vec<Record> {
    let mut records = Vec::new();
    for (index, text) in VECTORS.lines().enumerate() {
        let line = index + 1;
        if text.is_empty() || text.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = text.split(' ').collect();
        let record = match (fields[0], fields.len()) {
            ("block", 3) => Record::Block {
                plain: block(fields[1], line),
                cipher: block(fields[2], line),
            },
            ("encrypt", 3) => Record::Encrypt {
                plain: hex(fields[1], line),
                cipher: hex(fields[2], line),
            },
            ("decrypt", 4) => {
                let cipher = hex(fields[2], line);
                let plain = hex(fields[3], line);
                assert_eq!(cipher.len(), plain.len(), "line {line}: cipher and plain differ in length");
                assert!(cipher.len().is_multiple_of(32), "line {line}: length is not a multiple of 32");
                Record::Decrypt {
                    returned: fields[1].parse().unwrap_or_else(|_| panic!("line {line}: bad returned length")),
                    cipher,
                    plain,
                }
            }
            (kind, count) => panic!("line {line}: unknown kind {kind:?} or wrong field count {count}"),
        };
        records.push(record);
    }
    records
}

/// The Go `padder.Decode` rule applied to a plaintext: the length that
/// Go's `Decrypt` returns.
fn reference_decrypt_len(plain: &[u8]) -> usize {
    if plain.is_empty() {
        return 0;
    }
    let trailing_zeros = plain.iter().rev().take(31).take_while(|byte| **byte == 0).count();
    plain.len() - trailing_zeros
}

#[test]
fn vector_file_record_counts() {
    let records = parse();
    let blocks = records.iter().filter(|record| matches!(record, Record::Block { .. })).count();
    let encrypts = records.iter().filter(|record| matches!(record, Record::Encrypt { .. })).count();
    let decrypts = records.iter().filter(|record| matches!(record, Record::Decrypt { .. })).count();
    assert_eq!((blocks, encrypts, decrypts), (14, 13, 14));
}

#[test]
fn block_vectors_encrypt() {
    let cipher = Rijndael256::pr();
    for record in parse() {
        if let Record::Block { plain, cipher: expected } = record {
            assert_eq!(cipher.encrypt_block(&plain), expected);
        }
    }
}

#[test]
fn block_vectors_decrypt() {
    let cipher = Rijndael256::pr();
    for record in parse() {
        if let Record::Block { plain, cipher: encrypted } = record {
            assert_eq!(cipher.decrypt_block(&encrypted), plain);
        }
    }
}

#[test]
fn encrypt_vectors_pad_then_cbc() {
    let cipher = Rijndael256::pr();
    for record in parse() {
        if let Record::Encrypt { plain, cipher: expected } = record {
            let mut data = plain.clone();
            pad_zeros(&mut data);
            assert_eq!(data.len(), 32 * plain.len().div_ceil(32));
            assert!(data[plain.len()..].iter().all(|byte| *byte == 0));
            cipher.cbc_encrypt(&PR_IV, &mut data).unwrap();
            assert_eq!(data, expected, "plain length {}", plain.len());
        }
    }
}

#[test]
fn decrypt_vectors_keep_every_byte() {
    let cipher = Rijndael256::pr();
    for record in parse() {
        if let Record::Decrypt { cipher: encrypted, plain, .. } = record {
            let mut data = encrypted.clone();
            cipher.cbc_decrypt(&PR_IV, &mut data).unwrap();
            assert_eq!(data.len(), encrypted.len());
            assert_eq!(data, plain);
        }
    }
}

#[test]
fn decrypt_vectors_match_the_reference_strip_rule() {
    for record in parse() {
        if let Record::Decrypt { returned, plain, .. } = record {
            assert_eq!(reference_decrypt_len(&plain), returned);
        }
    }
}

#[test]
fn decrypt_vectors_include_stripped_cases() {
    let mut stripped = 0;
    for record in parse() {
        if let Record::Decrypt { returned, cipher, plain } = record
            && returned < cipher.len()
        {
            stripped += 1;
            assert!(plain[returned..].iter().all(|byte| *byte == 0));
        }
    }
    assert_eq!(stripped, 8);
}
