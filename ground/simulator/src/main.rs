use atlas_protocol::{decode_packet, encode_packet, Packet};
use rand::{Rng, RngExt};
use core::panic;
use std::time::{Instant, Duration};
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sim_start = Instant::now();
    let mut rng = rand::rng();

    let file = File::create("sim_packets.atl")?;
    let mut w  = BufWriter::new(file);


    //track packet success rate:
    let mut ok = 0usize;
    let mut dropped = 0usize;

    // loop
    let mut seq: u16 = 1;
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
        let err = rng.random_range(0..=10);
        if err == 5 {
            encoded_next[0] = 0x00;
        }

        let _write_result = write_to_file(&encoded_next, &mut w);

        // decode and identify if a packet is corrupted

        match decode_packet(&encoded_next) {
            Ok(decoded_next) => {
                ok += 1;
                if decoded_next != next {
                    println!("roundtrip mismatch: sent={:?}, got={:?}", next, decoded_next);
                }
            }
            Err(_e) => {
                dropped += 1;
                println!("Dropped corrupted packet");
            }
        }
        println!("ok={}, dropped={}", ok, dropped);
    }

    Ok(())
}

use std::fs::File;
use std::io::{BufWriter, Write};

fn write_to_file(frame: &Vec<u8>, w: &mut BufWriter<File>) -> std::io::Result<()> {
    let len = frame.len() as u32;
    w.write_all(&len.to_le_bytes())?;
    w.write_all(&frame)?;

    w.flush()?;
    Ok(())
}


use std::io::{self, Read, ErrorKind};
use std::io::BufReader;
fn read_next_frame(r: &mut BufReader<File>) -> std::io::Result<Option<Vec<u8>>> {
    let mut len_buf: [u8; 4] = [0u8; 4];
    match r.read(&mut len_buf)? {
        0 => return Ok(None),
        4 => {},
        _ => return Err(io::Error::new(ErrorKind::UnexpectedEof, "truncated length field")),
    }
    
    let len = u32::from_le_bytes(len_buf) as usize;
    let mut frame = vec![0u8; len];
    r.read_exact(&mut frame)?;

    Ok(Some(frame))
}

fn read_from_file(reader: &mut BufReader<File>) -> std::io::Result<()> {
    let mut pass = 0;
    let mut fail = 0;
    loop {
        match read_next_frame(reader)? {
            None => break,
            Some(frame) => {
                println!("frame: {:02X?}", frame);
                match decode_packet(&frame) {
                    Ok(_pkt) => {
                        pass += 1;
                    }
                    Err(_e) => {
                        fail += 1;
                    }
                }
            }
        }
    }

    println!(
        "Reader reported {} successful packets and {} failed packets", pass, fail
    );

    Ok(())
}
