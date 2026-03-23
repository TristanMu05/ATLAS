use atlas_protocol::{crc16_ccitt_false, crc_matches};

/**
 * This test verifies that our crc16_ccitt_false produces the expected output for a known test vector.
 */
#[test]
fn crc_matches_standard_ccitt_false_test_vector() {
    let crc = crc16_ccitt_false(b"123456789");
    assert_eq!(crc, 0x29B1);
}

/**
 * This test verifies that our crc16_ccitt_false produces the expected output for an empty buffer, which should be the seed value.
 */
#[test]
fn crc_for_empty_buffer_uses_seed_value() {
    let crc = crc16_ccitt_false(&[]);
    assert_eq!(crc, 0xFFFF);
}

/**
 * This test will verify that our crc_matches function correctly identifies a valid CRC and rejects an invalid one.
 */
#[test]
fn crc_checker_accepts_valid_and_rejects_corrupted_data() {
    let payload = b"ATLAS";
    let correct_crc = crc16_ccitt_false(payload);
    assert!(crc_matches(payload, correct_crc));

    let mut corrupted = payload.to_vec();
    corrupted[0] ^= 0x01;
    assert!(!crc_matches(&corrupted, correct_crc));
}

