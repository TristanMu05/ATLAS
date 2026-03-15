use atlas_protocol::{decode_packet};
use std::fs::File;
use std::io;
use std::io::{ErrorKind, BufReader, Read};
use std::thread;
use std::time::{Duration};

pub fn replay() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("logs/sim_packets.atl")?;
    let mut reader = BufReader::new(file);
    let _result = read_from_file(&mut reader);
    Ok(())
} 


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
    let mut ok = 0;
    let mut dropped = 0;
    let mut expected_seq: Option<u16> = None;
    let mut last_timestamp: Option<u32> = None;
    let speed = 1.0;

    loop {
        match read_next_frame(reader)? {
            None => break,
            Some(frame) => {
                print!("frame: {:02X?}", frame);
                match decode_packet(&frame) {
                    Ok(packet) => {
                        if let Some(prev) = last_timestamp {
                            let delta_ms = packet.timestamp.saturating_sub(prev) as u64;
                            thread::sleep(Duration::from_millis((delta_ms as f64 / speed) as u64));
                        }
                        last_timestamp = Some(packet.timestamp);
                        match expected_seq {
                            None => { // First valid seq
                                ok+=1;
                                expected_seq = Some(packet.sequence.wrapping_add(1));
                            }
                            Some(expected) if packet.sequence == expected => { // Correct seq
                                ok+=1;
                                expected_seq = Some(expected.wrapping_add(1));
                            }
                            Some(expected) if packet.sequence == expected.wrapping_sub(1) => { // Duplicate seq
                                
                            }
                            Some(expected) if packet.sequence > expected => { // Skipped seq
                                ok+=1;
                                dropped += packet.sequence.wrapping_sub(expected);
                                expected_seq = Some(packet.sequence.wrapping_add(1));
                            }
                            Some(_expected) => { /* Out of order seq */ }
                        }
                    }
                    Err(_e) => {
                        expected_seq = expected_seq.map(|seq| seq.wrapping_add(1));
                        dropped+=1;
                    }
                }
            }
        }
        print!("\n");
    }

    println!(
        "Reader reported {} successful packets and {} failed packets", ok, dropped
    );

    Ok(())
}