use std::time::{Instant, Duration};
use std::thread;
use atlas_simulator::simulate;
use std::fs::File;
use std::io::{BufWriter, Write, self, Read, ErrorKind, BufReader};
use atlas_protocol::{decode_packet, encode_packet, Packet};

/*
 * 
 * The logger is going to log our data, either from an input port accepting data from
 * the stm32, or our simulator if that port is not on.
 * 
 */
fn main() -> Result<(), Box<dyn std::error::Error>> {
    simulation()?;
    Ok(())
}


fn simulation() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create("sim_packets.atl")?;
    let mut writer  = BufWriter::new(file);
    simulate(|frame| {
        println!("frame: {:?}", frame);
        log_frame(&mut writer, frame)?;
        Ok(())
    })?;


/*
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
    */
    Ok(())
}

fn log_frame(w: &mut BufWriter<File>, frame: &[u8]) -> std::io::Result<()> {
    let len = frame.len() as u32;
    w.write_all(&len.to_le_bytes())?;
    w.write_all(&frame)?;

    w.flush()?;
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

    
