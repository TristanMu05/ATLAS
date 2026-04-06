use atlas_protocol::{encode_packet, Packet};
use rand::RngExt;
use std::time::{Instant, Duration};
use std::thread;

fn telemetry_payload(sequence: u16, timestamp_ms: u32) -> Vec<u8> {
    let mode = 0x01u8;
    let temperature_deci_c = 245i16 + ((sequence % 6) as i16);
    let voltage_mv = 3700u16.saturating_sub((sequence % 12) * 4);
    let light_raw = 900u16 + (timestamp_ms as u16 % 180);
    let status_flags = 0x1Fu8;
    let fault_flags = 0u16;

    let mut payload = Vec::with_capacity(10);
    payload.push(mode);
    payload.extend_from_slice(&temperature_deci_c.to_be_bytes());
    payload.extend_from_slice(&voltage_mv.to_be_bytes());
    payload.extend_from_slice(&light_raw.to_be_bytes());
    payload.push(status_flags);
    payload.extend_from_slice(&fault_flags.to_be_bytes());
    payload
}

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

        let elapsed_ms = sim_start.elapsed().as_millis() as u32;

        // build our simulated packet
        let next = Packet {
            message_id: 0x01,
            sequence: seq,
            timestamp: elapsed_ms,
            payload: telemetry_payload(seq, elapsed_ms),
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
