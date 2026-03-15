use atlas_protocol::{encode_packet, Packet};
use rand::{Rng, RngExt};
use std::time::{Instant, Duration};
use std::thread;
use atlas_protocol::crc16_ccitt_false;

pub fn simulate<F>(mut on_frame: F) -> Result<(), Box<dyn std::error::Error>> 
    where F: FnMut(&[u8]) -> std::io::Result<()>,
    {
    let sim_start = Instant::now();
    let mut rng = rand::rng();

    // loop
    let mut seq: u16 = 0;
    for _ in 0..100 {
        // simulate time delay between packets
        let base_ms: i64 = 100; //nominal 10hz
        let jitter_ms: i64 = 30; //+/- 30ms
        let delta = rng.random_range(-jitter_ms..=jitter_ms);
        let sleep_ms = (base_ms + delta).max(1) as u64;
        thread::sleep(Duration::from_millis(sleep_ms));

        // simulate a packet with varying payloads
        let payload_len = rng.random_range(0..=5);
        let mut payload = vec![0u8; payload_len];
        rng.fill_bytes(&mut payload);

        // build our simulated packet
        let next = Packet {
            message_id: 0x01,
            sequence: seq,
            timestamp: sim_start.elapsed().as_millis() as u32,
            payload,
        };

        seq += 1; // sequence

        // encode packet
        let mut encoded_next = encode_packet(&next);

        // 1/10 packets are corrupted
        let mut err = rng.random_range(0..10);
        
        if err == 5 {
            err = rng.random_range(1..=4);
            match err {
                1 => encoded_next[0] = 0x00, // corrupt sync
                2 => {
                    encoded_next[6] += 1;
                    let crc_index = encoded_next.len() - 2;
                    let new_crc = crc16_ccitt_false(&encoded_next[2..crc_index]);
                    encoded_next[crc_index..].copy_from_slice(&new_crc.to_be_bytes());
                    seq+=1;
                    continue;
                }, // corrupt sequence and redo crc to pass decode
                3 => {
                    let len = encoded_next.len(); // corrupt crc
                    encoded_next[len - 1] = 0x00;
                },
                4 => encoded_next[3] = 0x10, // corrupt length
                _ => {},
            }
        }

        on_frame(&encoded_next)?;
    }

    Ok(())
}