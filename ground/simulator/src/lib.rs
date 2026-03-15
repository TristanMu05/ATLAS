use atlas_protocol::{encode_packet, Packet};
use rand::{Rng, RngExt};
use std::time::{Instant, Duration};
use std::thread;

pub fn simulate<F>(mut on_frame: F) -> Result<(), Box<dyn std::error::Error>> 
    where F: FnMut(&[u8]) -> std::io::Result<()>,
    {
    let sim_start = Instant::now();
    let mut rng = rand::rng();
    let mut prev_frame = None;

    // loop
    let mut seq: u16 = 0;
    for _ in 0..1000 {
        // simulate time delay between packets
        let base_ms: i64 = 100; //nominal 10hz
        let jitter_ms: i64 = 30; //+/- 30ms
        let delta = rng.random_range(-jitter_ms..=jitter_ms);
        let sleep_ms = (base_ms + delta).max(1) as u64;
        thread::sleep(Duration::from_millis(sleep_ms));

        // simulate a packet with varying payloads
        let payload_len = 4;
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
                    err = rng.random_range(1..=3);
                    match err {
                        1 => continue,
                        2 => on_frame(&encoded_next)?,
                        3 => {
                            prev_frame = Some(encoded_next);
                            continue;
                        }
                        _ => {},
                    }
                }, // corrupt sequence and redo crc to pass decode
                3 => {
                    let len = encoded_next.len(); // corrupt crc
                    encoded_next[len - 1] = 0x00;
                },
                4 => encoded_next[4] = 0x00, // corrupt length
                _ => {},
            }
        }

        on_frame(&encoded_next)?;
        if let Some(frame) = prev_frame { // send the previous frame if it exists for a seq error
            on_frame(&frame)?;
            prev_frame = None;
        }
    }

    if let Some(frame) = prev_frame {
        on_frame(&frame)?;
    }

    Ok(())
}
