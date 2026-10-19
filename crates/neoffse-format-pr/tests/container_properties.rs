//! Property tests of the container stages and the pipeline.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::io::Write as _;

use flate2::Compression;
use flate2::write::DeflateEncoder;
use neoffse_format_pr::container::deflate::{deflate, inflate};
use neoffse_format_pr::container::text::{BOM, decode_base64, encode_base64, split};
use neoffse_format_pr::container::{Container, InflatePath, decode, encode, seal};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::Index;

const TRAILERS: [&[u8]; 4] = [b"", b"\r\n", b"\n", b" \t\r\n"];

fn is_trailer_byte(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

fn payloads() -> impl Strategy<Value = String> {
    prop_oneof!["\\PC{0,300}", (0usize..400, "[a-d{}:,\"0-9]{1,6}").prop_map(|(count, unit)| unit.repeat(count)),]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn split_rejoins_and_never_panics(file in vec(any::<u8>(), 0..=400)) {
        let framing = split(&file);
        let mut joined = if framing.bom { BOM.to_vec() } else { Vec::new() };
        joined.extend_from_slice(framing.text);
        joined.extend_from_slice(framing.trailer);
        prop_assert_eq!(&joined, &file);
        prop_assert!(framing.trailer.iter().all(|&byte| is_trailer_byte(byte)));
        prop_assert!(framing.text.last().is_none_or(|&byte| !is_trailer_byte(byte)));
    }

    #[test]
    fn base64_stage_inverts(bytes in vec(any::<u8>(), 1..=300)) {
        let text = encode_base64(&bytes);
        prop_assert_eq!(text.len(), 4 * bytes.len().div_ceil(3));
        prop_assert_eq!(decode_base64(text.as_bytes(), 0).unwrap(), bytes.clone());
        let unpadded: Vec<u8> = text.bytes().filter(|&byte| byte != b'=').collect();
        prop_assert_eq!(decode_base64(&unpadded, 0).unwrap(), bytes);
    }

    #[test]
    fn base64_decode_never_panics(text in vec(any::<u8>(), 0..=300), base_offset in 0usize..4) {
        let _ = decode_base64(&text, base_offset);
    }

    #[test]
    fn deflate_stage_inverts(bytes in vec(any::<u8>(), 0..=2000)) {
        let stream = deflate(&bytes).unwrap();
        let done = inflate(&stream, 1 << 20).unwrap();
        prop_assert!(done.ended);
        prop_assert_eq!(done.output, bytes);
    }

    #[test]
    fn inflate_never_panics_on_garbage(stream in vec(any::<u8>(), 0..=300)) {
        let _ = inflate(&stream, 4096);
    }

    #[test]
    fn encode_then_decode_is_the_identity(json in payloads(), bom in any::<bool>(), trailer in 0usize..TRAILERS.len()) {
        let container = Container { bom, trailer: TRAILERS[trailer].to_vec() };
        let decoded = decode(&encode(&json, &container).unwrap()).unwrap();
        prop_assert_eq!(decoded.json, json);
        prop_assert_eq!(decoded.container, container);
        prop_assert_eq!(decoded.inflate_path, InflatePath::FinalBlock);
    }

    #[test]
    fn sync_flushed_objects_decode_through_the_fallback(text in "[a-z0-9 ]{0,200}") {
        let json = format!("{{\"k\":\"{text}\"}}");
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(6));
        encoder.write_all(json.as_bytes()).unwrap();
        let stream = encoder.flush_finish().unwrap();
        let decoded = decode(&seal(&stream, &Container::game()).unwrap()).unwrap();
        prop_assert_eq!(decoded.json, json);
        prop_assert_eq!(decoded.inflate_path, InflatePath::Fallback);
    }

    #[test]
    fn decode_never_panics_on_arbitrary_files(file in vec(any::<u8>(), 0..=400)) {
        let _ = decode(&file);
    }

    #[test]
    fn a_flipped_bit_never_panics(json in payloads(), position in any::<Index>(), bit in 0u32..8) {
        let mut file = encode(&json, &Container::game()).unwrap();
        let index = position.index(file.len());
        file[index] ^= 1 << bit;
        let _ = decode(&file);
    }
}
