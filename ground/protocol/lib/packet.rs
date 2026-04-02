use crc16::{State, CCITT_FALSE};

pub fn crc16_ccitt_false(data: &[u8]) -> u16 {
    State::<CCITT_FALSE>::calculate(data)
}

pub fn crc_matches(data: &[u8], expected_crc: u16) -> bool {
    crc16_ccitt_false(data) == expected_crc
}


pub const SYNC_WORD: [u8; 2] = [0xAA, 0x55];
pub const FIXED_FIELDS_LEN: usize = 1 + 2 + 2 + 4; // message id + length + sequence + timestamp
pub const CRC_LEN: usize = 2;
pub const MIN_FRAME_LEN: usize = SYNC_WORD.len() + FIXED_FIELDS_LEN + CRC_LEN;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    pub message_id: u8,
    pub sequence: u16,
    pub timestamp: u32,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError{
    //packet too short
    PacketTooShort {
        min_len: usize,
        actual_len: usize,
    },
    //invalid sync
    InvalidSync {
        found: [u8; 2],
    },
    //length mismatch
    LengthMismatch {
        expected_len: usize,
        actual_len: usize,
    },
    //crc mismatch
    CrcMismatch {
        expected_crc: u16,
        actual_crc: u16,
    }
}

pub fn encode_packet(packet: &Packet) -> Vec<u8> {
    let payload_len = packet.payload.len();
    assert!(
        payload_len <= u16::MAX as usize,
        "payload length {} exceeds u16::MAX",
        payload_len
    );

    let mut frame = Vec::with_capacity(MIN_FRAME_LEN + payload_len);

    frame.extend_from_slice(&SYNC_WORD);
    frame.push(packet.message_id);
    frame.extend_from_slice(&(payload_len as u16).to_be_bytes());
    frame.extend_from_slice(&packet.sequence.to_be_bytes());
    frame.extend_from_slice(&packet.timestamp.to_be_bytes());
    frame.extend_from_slice(&packet.payload);

    let crc = crc16_ccitt_false(&frame[SYNC_WORD.len()..]);
    frame.extend_from_slice(&crc.to_be_bytes());
    frame
}

pub fn decode_packet(frame: &[u8]) -> Result<Packet, DecodeError> {
    // verify frame is a valid length
    if frame.len() < MIN_FRAME_LEN {
        return Err(DecodeError::PacketTooShort {
            min_len: MIN_FRAME_LEN,
            actual_len: frame.len(),
        });
    }
    // identify sync word
    let found_sync = [frame[0], frame[1]];
    // verify sync word
    if found_sync != SYNC_WORD {
        return Err(DecodeError::InvalidSync { found: found_sync });
    }

    // identify payload length
    let payload_len = u16::from_be_bytes([frame[3], frame[4]]) as usize;

    // compare expected length to actual length
    let expected_len = MIN_FRAME_LEN + payload_len;
    if frame.len() != expected_len {
        return Err(DecodeError::LengthMismatch {
            expected_len,
            actual_len: frame.len(),
        });
    }

    // identify crc
    let crc_index = expected_len - CRC_LEN;
    let expected_crc = u16::from_be_bytes([frame[crc_index], frame[crc_index + 1]]);
    let actual_crc = crc16_ccitt_false(&frame[SYNC_WORD.len()..crc_index]);
    if expected_crc != actual_crc {
        return Err(DecodeError::CrcMismatch {
            expected_crc,
            actual_crc,
        });
    }

    // extract fields
    let header_start = SYNC_WORD.len();
    let header_end = header_start + FIXED_FIELDS_LEN;
    let header = &frame[header_start..header_end];

    let message_id = header[0];
    let sequence = u16::from_be_bytes([header[3], header[4]]);
    let timestamp = u32::from_be_bytes([header[5], header[6], header[7], header[8]]);

    let payload = frame[header_end..crc_index].to_vec();

    Ok(Packet {
        message_id,
        sequence,
        timestamp,
        payload,
    }) 

}



