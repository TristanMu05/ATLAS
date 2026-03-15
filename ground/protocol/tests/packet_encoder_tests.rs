use atlas_protocol::{encode_packet, Packet, SYNC_WORD};

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
 * This test verifies that the encoder produces a frame with the correct structure and field order.
 * We first define the packet as we plan to pass to our encode_packet function, 
 *  then build it out byte by byte and compare to verify our function got the same result.
 */
#[test]
fn encoder_builds_expected_frame_order() {
    let packet = Packet {
        message_id: 0x01,
        sequence: 0x002A,
        timestamp: 0x01020304,
        payload: vec![0x10, 0x20, 0x30],
    };

    let encoded = encode_packet(&packet);

    let mut expected = Vec::new();
    expected.extend_from_slice(&SYNC_WORD);
    expected.push(0x01);
    expected.extend_from_slice(&3u16.to_be_bytes());
    expected.extend_from_slice(&0x002Au16.to_be_bytes());
    expected.extend_from_slice(&0x01020304u32.to_be_bytes());
    expected.extend_from_slice(&[0x10, 0x20, 0x30]);
    let expected_crc = reference_crc16_ccitt_false(&expected[2..]);
    expected.extend_from_slice(&expected_crc.to_be_bytes());

    assert_eq!(encoded, expected);
}


/**
 * This test verifies that the encoder correctly calculates the payload length and logs in correctly.
 */
#[test]
fn encoder_sets_length_from_payload_size() {
    let packet = Packet {
        message_id: 0x04,
        sequence: 7,
        timestamp: 12345,
        payload: vec![0xAB; 260],
    };

    let encoded = encode_packet(&packet);
    let length_bytes = [encoded[3], encoded[4]];
    assert_eq!(u16::from_be_bytes(length_bytes), 260);
}


/**
 * This test verifies that the CRC is calculated only on the intended portions of our frame
 */
#[test]
fn encoder_crc_covers_message_to_payload_only() {
    let packet = Packet {
        message_id: 0x03,
        sequence: 0xBEEF,
        timestamp: 0xAABBCCDD,
        payload: vec![0xDE, 0xAD, 0xBE, 0xEF],
    };

    let encoded = encode_packet(&packet);
    let crc_from_frame = u16::from_be_bytes([
        encoded[encoded.len() - 2],
        encoded[encoded.len() - 1],
    ]);
    let crc_reference = reference_crc16_ccitt_false(&encoded[2..encoded.len() - 2]);

    assert_eq!(crc_from_frame, crc_reference);
}
