//! The error mapping of the container pipeline: one case per `CodecError`
//! variant, the size caps and the messages.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::io::Write as _;

use flate2::Compression;
use flate2::write::DeflateEncoder;
use neoffse_format_pr::cipher::UnalignedLength;
use neoffse_format_pr::container::deflate::deflate;
use neoffse_format_pr::container::text::{BOM, encode_base64};
use neoffse_format_pr::container::{CodecError, Container, MAX_PAYLOAD_LEN, MAX_TEXT_LEN, decode, seal};

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
    assert!(matches!(CodecError::from(UnalignedLength { len: 33 }), CodecError::CipherLength { len: 33 }));
}

#[test]
fn decode_reports_inflate_garbage() {
    let outcome = decode(&seal(&[0x07], &Container::game()).unwrap());
    assert!(matches!(outcome, Err(CodecError::Inflate { .. })));
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
    assert!(matches!(decode(&file), Err(CodecError::PayloadTooLarge { limit: MAX_PAYLOAD_LEN })));
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
    let message = decode(&file).unwrap_err().to_string();
    assert!(message.contains('5'), "{message}");
    let message = CodecError::CipherLength { len: 33 }.to_string();
    assert!(message.contains("33") && message.contains("32"), "{message}");
}
