//! The error mapping of the container pipeline: one case per `CodecError`
//! variant, the size caps and the messages.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::io::Write as _;

use flate2::Compression;
use flate2::write::DeflateEncoder;
use neoffse_format_pr::cipher::{PR_IV, Rijndael256};
use neoffse_format_pr::container::deflate::deflate;
use neoffse_format_pr::container::text::{BOM, encode_base64};
use neoffse_format_pr::container::{CodecError, Container, MAX_PAYLOAD_LEN, MAX_TEXT_LEN, MAX_TRAILER_LEN, decode, encode, seal};

#[test]
fn decode_reports_an_empty_file() {
    for file in [&[][..], &BOM[..], b" \r\n"] {
        assert!(matches!(decode(file), Err(CodecError::Empty)));
    }
}

#[test]
fn decode_reports_bad_base64_with_a_file_offset() {
    let mut file = BOM.to_vec();
    file.extend_from_slice(b"QU-D");
    assert!(matches!(decode(&file), Err(CodecError::Base64 { offset: 5, .. })));
}

#[test]
fn decode_rejects_plain_json_text() {
    assert!(matches!(decode(b"{\"a\":1}"), Err(CodecError::Base64 { offset: 0, .. })));
}

#[test]
fn decode_rejects_unaligned_ciphertext() {
    for len in [31, 33] {
        let file = encode_base64(&vec![7_u8; len]);
        assert!(matches!(decode(file.as_bytes()), Err(CodecError::CipherLength { len: found }) if found == len));
    }
}

#[test]
fn unaligned_length_maps_to_cipher_length() {
    let error = Rijndael256::pr().cbc_encrypt(&PR_IV, &mut [0_u8; 33]).unwrap_err();
    assert!(matches!(CodecError::from(error), CodecError::CipherLength { len: 33 }));
}

#[test]
fn decode_reports_inflate_garbage() {
    let outcome = decode(&seal(&[0x07], &Container::game()).unwrap());
    assert!(matches!(outcome, Err(CodecError::Inflate { offset: 1, .. })), "{outcome:?}");
}

#[test]
fn decode_reports_invalid_utf8() {
    let stream = deflate(&[b'{', 0xFF, b'}']).unwrap();
    let outcome = decode(&seal(&stream, &Container::game()).unwrap());
    assert!(matches!(outcome, Err(CodecError::Utf8 { source }) if source.valid_up_to() == 1));
}

#[test]
fn decode_refuses_a_payload_above_the_limit() {
    let stream = deflate(&vec![0_u8; MAX_PAYLOAD_LEN + 1]).unwrap();
    let outcome = decode(&seal(&stream, &Container::game()).unwrap());
    assert!(matches!(outcome, Err(CodecError::PayloadTooLarge { limit: MAX_PAYLOAD_LEN })));
}

#[test]
fn decode_refuses_a_text_longer_than_any_capped_payload() {
    let file = vec![b'A'; MAX_TEXT_LEN + 4];
    assert!(matches!(decode(&file), Err(CodecError::PayloadTooLarge { limit: MAX_TEXT_LEN })));
}

/// A valid file with a BOM and no trailer, for tests that append their own.
fn file_without_trailer() -> Vec<u8> {
    let container = Container {
        bom: true,
        trailer: Vec::new(),
    };
    encode("{}", &container).unwrap()
}

#[test]
fn max_trailer_len_is_64_and_holds_the_game_crlf() {
    assert_eq!(MAX_TRAILER_LEN, 64);
    assert!(Container::game().trailer.len() <= MAX_TRAILER_LEN);
}

#[test]
fn decode_refuses_a_trailer_one_byte_above_the_cap() {
    let mut file = file_without_trailer();
    file.extend_from_slice(&b"\r\n".repeat(32));
    file.push(b'\n');
    let outcome = decode(&file);
    assert!(matches!(outcome, Err(CodecError::TrailerTooLong { len: 65, limit: 64 })), "{outcome:?}");
}

/// A file padded with megabytes of whitespace.
#[test]
fn decode_refuses_a_megabyte_of_trailing_spaces() {
    let mut file = file_without_trailer();
    file.resize(file.len() + 1_048_576, b' ');
    let outcome = decode(&file);
    assert!(matches!(outcome, Err(CodecError::TrailerTooLong { len: 1_048_576, .. })), "{outcome:?}");
}

/// A text over the cap, then `trailer_len` spaces.
fn oversized_text_with_trailer(trailer_len: usize) -> Vec<u8> {
    let mut file = vec![b'A'; MAX_TEXT_LEN + 4];
    file.resize(file.len() + trailer_len, b' ');
    file
}

#[test]
fn the_trailer_check_comes_before_the_text_cap() {
    let outcome = decode(&oversized_text_with_trailer(MAX_TRAILER_LEN + 1));
    assert!(matches!(outcome, Err(CodecError::TrailerTooLong { len: 65, limit: 64 })), "{outcome:?}");
}

#[test]
fn the_text_cap_still_fires_with_a_trailer_at_the_cap() {
    let outcome = decode(&oversized_text_with_trailer(MAX_TRAILER_LEN));
    assert!(matches!(outcome, Err(CodecError::PayloadTooLarge { limit: MAX_TEXT_LEN })), "{outcome:?}");
}

#[test]
fn whitespace_only_stays_empty_above_the_cap() {
    let spaces = vec![b' '; 1000];
    let mut with_bom = BOM.to_vec();
    with_bom.extend_from_slice(&spaces);
    for file in [spaces, with_bom] {
        assert!(matches!(decode(&file), Err(CodecError::Empty)));
    }
}

/// The slowest test of the crate: it encrypts and decrypts about 16 MiB.
#[test]
fn max_text_len_admits_the_worst_case_payload() {
    let payload = vec![b'a'; MAX_PAYLOAD_LEN];
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::none());
    encoder.write_all(&payload).unwrap();
    let stream = encoder.finish().unwrap();
    assert!(stream.len() > MAX_PAYLOAD_LEN);
    let file = seal(
        &stream,
        &Container {
            bom: false,
            trailer: Vec::new(),
        },
    )
    .unwrap();
    assert!(file.len() <= MAX_TEXT_LEN);
    let decoded = decode(&file).unwrap();
    assert_eq!(decoded.json.as_bytes(), payload.as_slice());
}

#[test]
fn error_messages_name_the_step_and_the_position() {
    let mut file = BOM.to_vec();
    file.extend_from_slice(b"QU-D");
    let error = decode(&file).unwrap_err();
    assert!(matches!(error, CodecError::Base64 { offset: 5, .. }));
    assert!(std::error::Error::source(&error).is_some());
    let message = error.to_string();
    assert!(message.contains("byte 5"), "{message}");
    let message = CodecError::CipherLength { len: 33 }.to_string();
    assert!(message.contains("33") && message.contains("32"), "{message}");
    let message = CodecError::TrailerTooLong { len: 65, limit: 64 }.to_string();
    assert!(message.contains("65") && message.contains("64"), "{message}");
}

/// A JSON string literal of exactly `len` bytes.
fn string_literal(len: usize) -> String {
    format!("\"{}\"", "a".repeat(len - 2))
}

#[test]
fn encode_refuses_a_payload_decode_would_refuse() {
    let json = string_literal(MAX_PAYLOAD_LEN + 1);
    assert!(matches!(
        encode(&json, &Container::game()),
        Err(CodecError::PayloadTooLarge { limit: MAX_PAYLOAD_LEN })
    ));
}

#[test]
fn encode_accepts_the_largest_payload_decode_accepts() {
    let json = string_literal(MAX_PAYLOAD_LEN);
    let file = encode(&json, &Container::game()).unwrap();
    assert_eq!(decode(&file).unwrap().json, json);
}
