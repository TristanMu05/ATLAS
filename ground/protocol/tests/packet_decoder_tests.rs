use atlas_protocol::{decode_packet, DecodeError, Packet, SYNC_WORD};

/**
 * This is a direct implementation of the crc16_ccitt_false algorithm to ensure our encoder is calculating as intended
 */
fn reference_crc16_ccitt_false(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            crc = if (crc & 0x8000) != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

/**
 * Helper function to build a valid frame for a given packet to be used in our decoder tests as what we should expect from a successful decode.
 */
fn build_valid_frame(packet: &Packet) -> Vec<u8> {
    let mut frame = Vec::new();
    frame.extend_from_slice(&SYNC_WORD);
    frame.push(packet.message_id);
    frame.extend_from_slice(&(packet.payload.len() as u16).to_be_bytes());
    frame.extend_from_slice(&packet.sequence.to_be_bytes());
    frame.extend_from_slice(&packet.timestamp.to_be_bytes());
    frame.extend_from_slice(&packet.payload);
    let crc = reference_crc16_ccitt_false(&frame[2..]);
    frame.extend_from_slice(&crc.to_be_bytes());
    frame
}


/**
 * This test builds a packet, encodes it with a valid frame structure, then verifies with our decoder that we can successfully decode a valid frame.
 */
#[test]
fn decoder_parses_valid_frame() {
    let expected = Packet {
        message_id: 0x02,
        sequence: 0x1234,
        timestamp: 0xABCDEF01,
        payload: vec![1, 2, 3, 4, 5],
    };
    let frame = build_valid_frame(&expected);

    let decoded = decode_packet(&frame).expect("frame should decode");
    assert_eq!(decoded, expected);
}

/**
 * This test verifies that our decoder correctly identifies and rejects a frame with an invalid sync word we intentionally corrupt.
 */
#[test]
fn decoder_rejects_bad_sync() {
    let packet = Packet {
        message_id: 0x01,
        sequence: 1,
        timestamp: 2,
        payload: vec![0x42],
    };
    let mut frame = build_valid_frame(&packet);
    frame[0] = 0x00;

    let result = decode_packet(&frame);
    assert_eq!(
        result,
        Err(DecodeError::InvalidSync {
            found: [0x00, SYNC_WORD[1]]
        })
    );
}

/**
 * This test intentionally corrupts the payload length and verifies our decoder correctly identifies this and rejects it.
 */
#[test]
fn decoder_rejects_length_mismatch() {
    let packet = Packet {
        message_id: 0x04,
        sequence: 9,
        timestamp: 10,
        payload: vec![0xAA, 0xBB, 0xCC],
    };
    let mut frame = build_valid_frame(&packet);
    frame[3] = 0x00;
    frame[4] = 0x04; // Declared payload is 4 bytes, actual payload is 3 bytes.

    let result = decode_packet(&frame);
    assert_eq!(
        result,
        Err(DecodeError::LengthMismatch {
            expected_len: frame.len() + 1,
            actual_len: frame.len()
        })
    );
}

/**
 * This test intentionally corrupts the payload and verifies our decoder correctly identifies the CRC mismatch with the frame value.
 */
#[test]
fn decoder_rejects_crc_mismatch() {
    let packet = Packet {
        message_id: 0x03,
        sequence: 99,
        timestamp: 123,
        payload: vec![0x10, 0x20, 0x30],
    };
    let mut frame = build_valid_frame(&packet);
    let payload_start = 11;
    frame[payload_start] ^= 0xFF;

    let result = decode_packet(&frame);
    assert!(matches!(result, Err(DecodeError::CrcMismatch { .. })));
}
