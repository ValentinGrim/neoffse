//! The text stage of the container: BOM, line end and base64.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse_format_pr::container::CodecError;
use neoffse_format_pr::container::text::{BOM, decode_base64, encode_base64, split};

fn with_bom(rest: &[u8]) -> Vec<u8> {
    let mut file = BOM.to_vec();
    file.extend_from_slice(rest);
    file
}

#[test]
fn split_reads_bom_text_and_crlf() {
    let file = with_bom(b"QUJD\r\n");
    let framing = split(&file);
    assert!(framing.bom);
    assert_eq!(framing.text, b"QUJD");
    assert_eq!(framing.trailer, b"\r\n");
}

#[test]
fn split_without_bom_or_trailer() {
    let framing = split(b"QUJD");
    assert!(!framing.bom);
    assert_eq!(framing.text, b"QUJD");
    assert_eq!(framing.trailer, b"");
}

#[test]
fn split_trims_space_tab_cr_lf_only_at_the_end() {
    let framing = split(b"AA== \t\r\n \n");
    assert_eq!(framing.text, b"AA==");
    assert_eq!(framing.trailer, b" \t\r\n \n");
}

#[test]
fn split_keeps_inner_whitespace() {
    let framing = split(b"QU JD\n");
    assert_eq!(framing.text, b"QU JD");
    assert_eq!(framing.trailer, b"\n");
}

#[test]
fn split_reads_a_bom_only_at_byte_zero() {
    let mut file = b" ".to_vec();
    file.extend_from_slice(&with_bom(b"QUJD"));
    let framing = split(&file);
    assert!(!framing.bom);
    assert_eq!(framing.text, file.as_slice());
    assert_eq!(framing.trailer, b"");
}

#[test]
fn split_does_not_trim_form_feed() {
    let framing = split(b"QUJD\x0c");
    assert_eq!(framing.text, b"QUJD\x0c");
    assert_eq!(framing.trailer, b"");
    let framing = split(b"QUJD\x0c\r\n");
    assert_eq!(framing.text, b"QUJD\x0c");
    assert_eq!(framing.trailer, b"\r\n");
}

#[test]
fn split_handles_empty_input_and_bom_only_and_whitespace_only() {
    assert_eq!(split(&[]).text, b"");
    let bom_only = split(&BOM);
    assert!(bom_only.bom);
    assert_eq!(bom_only.text, b"");
    let blank = split(b" \r\n");
    assert_eq!(blank.text, b"");
    assert_eq!(blank.trailer, b" \r\n");
}

#[test]
fn decode_base64_accepts_canonical_text() {
    assert_eq!(decode_base64(b"QUJD", 0).unwrap(), b"ABC");
    assert_eq!(decode_base64(b"QQ==", 0).unwrap(), b"A");
}

#[test]
fn decode_base64_appends_missing_padding() {
    assert_eq!(decode_base64(b"QQ", 0).unwrap(), b"A");
    assert_eq!(decode_base64(b"QUI", 0).unwrap(), b"AB");
}

#[test]
fn decode_base64_rejects_a_text_of_4n_plus_1_characters() {
    assert!(matches!(decode_base64(b"QUJDQ", 0), Err(CodecError::Base64 { .. })));
}

#[test]
fn decode_base64_uses_the_standard_alphabet() {
    assert_eq!(decode_base64(b"+/+/", 0).unwrap(), [0xFB, 0xFF, 0xBF]);
    assert!(matches!(decode_base64(b"QU-D", 0), Err(CodecError::Base64 { .. })));
    assert!(matches!(decode_base64(b"QU_D", 0), Err(CodecError::Base64 { .. })));
}

#[test]
fn decode_base64_reports_the_file_offset_of_a_bad_byte() {
    assert!(matches!(decode_base64(b"QU-D", 3), Err(CodecError::Base64 { offset: 5, .. })));
}

#[test]
fn decode_base64_rejects_non_zero_trailing_bits() {
    assert!(matches!(decode_base64(b"QR==", 0), Err(CodecError::Base64 { .. })));
}

#[test]
fn decode_base64_rejects_embedded_padding() {
    assert!(matches!(decode_base64(b"QQ==QQ==", 0), Err(CodecError::Base64 { .. })));
}

#[test]
fn encode_base64_pads_to_a_multiple_of_four() {
    assert_eq!(encode_base64(b"A"), "QQ==");
    assert_eq!(encode_base64(b"AB"), "QUI=");
    assert_eq!(encode_base64(b"ABC"), "QUJD");
    let thirty_two = encode_base64(&[0x55; 32]);
    assert_eq!(thirty_two.len(), 44);
    assert!(thirty_two.ends_with('='));
    assert!(encode_base64(&[0x55; 64]).ends_with("=="));
    assert!(!encode_base64(&[0x55; 96]).contains('='));
}
