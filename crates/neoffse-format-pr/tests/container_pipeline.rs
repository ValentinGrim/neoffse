//! The whole container pipeline: encode, decode, the stripping trap and the
//! Fallback. The error mapping lives in `container_errors.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::io::Write as _;

use flate2::Compression;
use flate2::write::DeflateEncoder;
use neoffse_format_pr::cipher::{BLOCK_SIZE, PR_IV, Rijndael256};
use neoffse_format_pr::container::deflate::{deflate, inflate};
use neoffse_format_pr::container::text::{BOM, decode_base64, split};
use neoffse_format_pr::container::{CodecError, Container, InflatePath, MAX_PAYLOAD_LEN, MAX_TRAILER_LEN, decode, encode, seal};

/// The decrypted bytes of a file: stream plus padding.
fn plain_of(file: &[u8]) -> Vec<u8> {
    let framing = split(file);
    let mut plain = decode_base64(framing.text, 0).unwrap();
    Rijndael256::pr().cbc_decrypt(&PR_IV, &mut plain).unwrap();
    plain
}

/// The Go `padder.Decode` rule: drop up to 31 trailing zeros.
fn reference_strip_len(plain: &[u8]) -> usize {
    let zeros = plain.iter().rev().take(31).take_while(|&&byte| byte == 0).count();
    plain.len() - zeros
}

fn sync_flushed(payload: &[u8]) -> Vec<u8> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(6));
    encoder.write_all(payload).unwrap();
    let stream = encoder.flush_finish().unwrap();
    assert!(!inflate(&stream, MAX_PAYLOAD_LEN).unwrap().ended);
    stream
}

fn padding_of(len: usize) -> usize {
    (BLOCK_SIZE - len % BLOCK_SIZE) % BLOCK_SIZE
}

fn counting_object(count: usize) -> String {
    let items: Vec<String> = (1..=count).map(|item| item.to_string()).collect();
    format!("{{\"v\":[{}]}}", items.join(","))
}

/// A counting object whose deflate stream has `len % 32 == remainder`.
fn counting_object_with_remainder(remainder: usize) -> String {
    (0..400)
        .map(counting_object)
        .find(|json| deflate(json.as_bytes()).unwrap().len() % BLOCK_SIZE == remainder)
        .unwrap()
}

/// The stripping trap: a short payload whose stream ends in a zero byte and
/// whose padding stays below 31.
fn trap() -> (String, Vec<u8>) {
    (0..4096)
        .map(|id| format!("{{\"id\":{id}}}"))
        .map(|json| {
            let stream = deflate(json.as_bytes()).unwrap();
            (json, stream)
        })
        .find(|(_, stream)| stream.last() == Some(&0) && padding_of(stream.len()) < 31)
        .unwrap()
}

fn container(bom: bool, trailer: &[u8]) -> Container {
    Container {
        bom,
        trailer: trailer.to_vec(),
    }
}

fn containers() -> [Container; 3] {
    [Container::game(), container(false, b""), container(true, b" \n")]
}

#[test]
fn encode_writes_bom_base64_and_crlf() {
    let file = encode("{}", &Container::game()).unwrap();
    assert!(file.starts_with(&BOM));
    assert!(file.ends_with(b"\r\n"));
    let middle = &file[3..file.len() - 2];
    assert!(middle.iter().all(|&byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'=')));
    assert_eq!(middle.len() % 4, 0);
}

#[test]
fn encode_without_bom_or_trailer() {
    let file = encode("{}", &container(false, b"")).unwrap();
    assert_ne!(file[0], 0xEF);
    let last = *file.last().unwrap();
    assert!(last.is_ascii_alphanumeric() || matches!(last, b'+' | b'/' | b'='));
}

#[test]
fn encode_pads_the_stream_with_zeros_to_a_block() {
    let payload = "{\"id\":12}";
    let file = encode(payload, &Container::game()).unwrap();
    let plain = plain_of(&file);
    let stream = deflate(payload.as_bytes()).unwrap();
    assert_eq!(plain.len() % BLOCK_SIZE, 0);
    assert_eq!(&plain[..stream.len()], stream.as_slice());
    assert!(plain[stream.len()..].iter().all(|&byte| byte == 0));
}

#[test]
fn encode_adds_no_block_when_the_stream_is_aligned() {
    let json = counting_object_with_remainder(0);
    let stream = deflate(json.as_bytes()).unwrap();
    let plain = plain_of(&encode(&json, &Container::game()).unwrap());
    assert_eq!(plain.len(), stream.len());
}

#[test]
fn encode_rejects_a_trailer_with_a_non_whitespace_byte() {
    let outcome = encode("{}", &container(true, b" x"));
    assert!(matches!(outcome, Err(CodecError::Trailer { index: 1 })));
}

#[test]
fn a_trailer_at_the_cap_round_trips() {
    let at_cap = container(true, &b"\r\n".repeat(MAX_TRAILER_LEN / 2));
    assert_eq!(at_cap.trailer.len(), MAX_TRAILER_LEN);
    let decoded = decode(&encode("{\"a\":1}", &at_cap).unwrap()).unwrap();
    assert_eq!(decoded.json, "{\"a\":1}");
    assert_eq!(decoded.container, at_cap);
}

#[test]
fn seal_refuses_a_trailer_above_the_cap() {
    let mut trailer = b"\r\n".repeat(32);
    trailer.push(b' ');
    let above = container(true, &trailer);
    assert_eq!(above.trailer.len(), 65);
    let stream = deflate(b"{}").unwrap();
    let sealed = seal(&stream, &above);
    assert!(matches!(sealed, Err(CodecError::TrailerTooLong { len: 65, limit: 64 })), "{sealed:?}");
    let encoded = encode("{}", &above);
    assert!(matches!(encoded, Err(CodecError::TrailerTooLong { len: 65, limit: 64 })), "{encoded:?}");
}

#[test]
fn the_length_check_comes_before_the_byte_check() {
    let mut trailer = vec![b' '; MAX_TRAILER_LEN];
    trailer.push(b'x');
    let stream = deflate(b"{}").unwrap();
    let sealed = seal(&stream, &container(false, &trailer));
    assert!(matches!(sealed, Err(CodecError::TrailerTooLong { len: 65, .. })), "{sealed:?}");
}

#[test]
fn decode_reports_the_container_it_read() {
    for container in containers() {
        let file = encode("{\"a\":1}", &container).unwrap();
        assert_eq!(decode(&file).unwrap().container, container);
    }
}

#[test]
fn round_trip_keeps_json_and_container() {
    let json = "{\"name\":\"Hero\",\"gil\":999}";
    for container in containers() {
        let decoded = decode(&encode(json, &container).unwrap()).unwrap();
        assert_eq!(decoded.json, json);
        assert_eq!(decoded.container, container);
        assert_eq!(decoded.inflate_path, InflatePath::FinalBlock);
    }
}

#[test]
fn decode_accepts_a_file_with_the_base64_padding_removed() {
    let json = counting_object(10);
    let file = encode(&json, &container(false, b"")).unwrap();
    let unpadded: Vec<u8> = file.iter().copied().filter(|&byte| byte != b'=').collect();
    assert_ne!(unpadded.len(), file.len());
    assert_eq!(decode(&unpadded).unwrap().json, json);
}

#[test]
fn decode_keeps_a_stream_byte_that_is_zero() {
    let (json, stream) = trap();
    let file = seal(&stream, &Container::game()).unwrap();
    let decoded = decode(&file).unwrap();
    assert_eq!(decoded.json, json);
    assert_eq!(decoded.inflate_path, InflatePath::FinalBlock);
}

#[test]
fn the_reference_strip_would_cut_that_byte() {
    let (_, stream) = trap();
    let plain = plain_of(&seal(&stream, &Container::game()).unwrap());
    let stripped = reference_strip_len(&plain);
    assert!(stripped < stream.len());
    let outcome = inflate(&plain[..stripped], MAX_PAYLOAD_LEN);
    assert!(!matches!(outcome, Ok(ref done) if done.ended));
}

#[test]
fn decode_handles_31_bytes_of_padding() {
    let json = counting_object_with_remainder(1);
    let decoded = decode(&encode(&json, &Container::game()).unwrap()).unwrap();
    assert_eq!(decoded.json, json);
    assert_eq!(decoded.inflate_path, InflatePath::FinalBlock);
}

#[test]
fn bytes_after_the_final_block_are_ignored() {
    let json = "{\"a\":[1,2,3]}";
    let mut stream = deflate(json.as_bytes()).unwrap();
    stream.extend_from_slice(&[0xAA; 5]);
    let decoded = decode(&seal(&stream, &Container::game()).unwrap()).unwrap();
    assert_eq!(decoded.json, json);
    assert_eq!(decoded.inflate_path, InflatePath::FinalBlock);
}

#[test]
fn decode_takes_the_fallback_for_a_sync_flushed_stream() {
    let payloads = (0..200)
        .map(|number| format!("{{\"k\":{number}}}"))
        .chain((0..120).map(|len| format!("{{\"k\":\"{}\"}}", "a".repeat(len))));
    let mut short_padding = false;
    let mut long_padding = false;
    for json in payloads {
        let stream = sync_flushed(json.as_bytes());
        let padding = padding_of(stream.len());
        short_padding |= padding < 5;
        long_padding |= padding >= 5;
        let decoded = decode(&seal(&stream, &Container::game()).unwrap()).unwrap();
        assert_eq!(decoded.json, json);
        assert_eq!(decoded.inflate_path, InflatePath::Fallback);
    }
    assert!(short_padding && long_padding);
}

#[test]
fn fallback_rejects_an_incomplete_json_object() {
    // The primary error depends on the padding: below 5 zero bytes the
    // inflater runs dry, from 5 the zeros read as a stored block with a bad
    // LEN/NLEN pair. The Fallback returns whichever it was.
    let (mut dry, mut bad_block) = (false, false);
    for extra in 0..40 {
        let json = format!("{{\"a\":[1,2{}", ",3".repeat(extra));
        let stream = sync_flushed(json.as_bytes());
        let outcome = decode(&seal(&stream, &Container::game()).unwrap());
        if padding_of(stream.len()) < 5 {
            assert!(matches!(outcome, Err(CodecError::InflateTruncated { .. })), "{json}");
            dry = true;
        } else {
            assert!(matches!(outcome, Err(CodecError::Inflate { .. })), "{json}");
            bad_block = true;
        }
    }
    assert!(dry && bad_block);

    // The depth reaches zero before the last byte: the stripped view is no
    // single object, so the primary error stands.
    let stream = sync_flushed(b"{\"a\":1}x");
    let outcome = decode(&seal(&stream, &Container::game()).unwrap());
    assert!(
        matches!(outcome, Err(CodecError::InflateTruncated { .. } | CodecError::Inflate { .. })),
        "{outcome:?}"
    );
}

#[test]
fn fallback_rejects_a_long_stream_cut_mid_document() {
    let stream = deflate(counting_object(300).as_bytes()).unwrap();
    let cut = &stream[..stream.len() - 10];
    let outcome = decode(&seal(cut, &Container::game()).unwrap());
    assert!(matches!(outcome, Err(CodecError::InflateTruncated { .. } | CodecError::Inflate { .. })));
}

#[test]
fn fallback_rejects_garbage_after_a_flushed_stream() {
    let mut stream = sync_flushed(b"{\"a\":1}");
    stream.extend_from_slice(&[0x07, 0xFF]);
    let outcome = decode(&seal(&stream, &Container::game()).unwrap());
    assert!(matches!(outcome, Err(CodecError::Inflate { .. })));
}

#[test]
fn decode_of_zero_plaintext_is_an_inflate_error() {
    let outcome = decode(&seal(&[0_u8; 32], &Container::game()).unwrap());
    assert!(matches!(outcome, Err(CodecError::Inflate { .. })));
}

#[test]
fn final_block_stream_never_uses_the_fallback() {
    for number in 0..200 {
        let json = format!("{{\"k\":{number}}}");
        let decoded = decode(&encode(&json, &Container::game()).unwrap()).unwrap();
        assert_eq!(decoded.inflate_path, InflatePath::FinalBlock);
    }
}
