//! The raw DEFLATE stage of the container.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::io::Write as _;

use flate2::Compression;
use flate2::write::{DeflateEncoder, ZlibEncoder};
use neoffse_format_pr::container::CodecError;
use neoffse_format_pr::container::deflate::{LEVEL, deflate, inflate};

const LIMIT: usize = 1 << 20;

fn mixed(len: usize) -> Vec<u8> {
    (0..len).map(|index| u8::try_from((index * 7 + index / 13) % 251).unwrap()).collect()
}

fn sync_flushed(payload: &[u8]) -> Vec<u8> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(6));
    encoder.write_all(payload).unwrap();
    let stream = encoder.flush_finish().unwrap();
    assert!(!inflate(&stream, LIMIT).unwrap().ended);
    stream
}

#[test]
fn deflate_then_inflate_returns_the_payload() {
    for payload in [Vec::new(), vec![b'x'], vec![b'q'; 10_000], mixed(100_000)] {
        let stream = deflate(&payload).unwrap();
        let done = inflate(&stream, LIMIT).unwrap();
        assert!(done.ended);
        assert_eq!(done.consumed, stream.len());
        assert_eq!(done.output, payload);
    }
}

#[test]
fn inflate_stops_at_the_final_block_and_ignores_the_tail() {
    let payload = mixed(5_000);
    let stream = deflate(&payload).unwrap();
    for tail in [vec![0_u8; 31], vec![0xAA_u8; 40]] {
        let mut padded = stream.clone();
        padded.extend_from_slice(&tail);
        let done = inflate(&padded, LIMIT).unwrap();
        assert!(done.ended);
        assert_eq!(done.consumed, stream.len());
        assert_eq!(done.output, payload);
    }
}

#[test]
fn inflate_reports_a_stream_cut_before_its_end() {
    let stream = deflate(&mixed(5_000)).unwrap();
    let cut = &stream[..stream.len() - 1];
    let done = inflate(cut, LIMIT).unwrap();
    assert!(!done.ended);
    assert_eq!(done.consumed, cut.len());
}

#[test]
fn inflate_reports_a_sync_flushed_stream_as_running_dry() {
    let payload = b"{\"k\":1234567}".to_vec();
    let stream = sync_flushed(&payload);
    let done = inflate(&stream, LIMIT).unwrap();
    assert!(!done.ended);
    assert_eq!(done.consumed, stream.len());
    assert_eq!(done.output, payload);
}

#[test]
fn inflate_of_empty_input_is_not_a_final_block() {
    let done = inflate(&[], 10).unwrap();
    assert!(!done.ended);
    assert_eq!(done.consumed, 0);
    assert_eq!(done.output, b"");
}

#[test]
fn inflate_reports_a_reserved_block_type() {
    assert!(matches!(inflate(&[0x07, 0, 0, 0], LIMIT), Err(CodecError::Inflate { .. })));
}

#[test]
fn inflate_rejects_a_zlib_wrapped_stream() {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::new(6));
    encoder.write_all(b"{\"a\":1}").unwrap();
    let wrapped = encoder.finish().unwrap();
    assert!(matches!(inflate(&wrapped, LIMIT), Err(CodecError::Inflate { .. })));
}

#[test]
fn inflate_enforces_the_output_limit() {
    let stream = deflate(&[b'a'; 10_000]).unwrap();
    assert_eq!(inflate(&stream, 10_000).unwrap().output.len(), 10_000);
    assert!(matches!(inflate(&stream, 9_999), Err(CodecError::PayloadTooLarge { limit: 9_999 })));
}

#[test]
fn deflate_uses_the_documented_level() {
    let input: Vec<u8> = (0..4_000)
        .flat_map(|number: u32| format!("{{\"id\":{number},\"name\":\"item{}\"}}", number % 37).into_bytes())
        .collect();
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(LEVEL));
    encoder.write_all(&input).unwrap();
    assert_eq!(deflate(&input).unwrap(), encoder.finish().unwrap());
}
