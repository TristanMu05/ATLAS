use atlas_protocol::{decode_packet};
use std::fs::File;
use std::io;
use std::io::{ErrorKind, BufReader, Read};



fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open("../../ground/logger/logs/sim_packets.atl")?;
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
    let mut pass = 0;
    let mut fail = 0;
    let mut seq = 0;

    loop {
        match read_next_frame(reader)? {
            None => break,
            Some(frame) => {
                print!("frame: {:02X?}", frame);
                match decode_packet(&frame) {
                    Ok(packet) =>{
                        if packet.sequence != seq {
                            fail+=1;
                            seq = packet.sequence + 1;
                            print!("\npackets out of order. Expected={seq}, got={:?}", packet.sequence);
                        }else{
                            pass+=1;
                            seq+=1;
                        }
                    }
                    Err(e) => {
                        print!("\n{:?}", e);
                        fail+=1;
                    }
                }
            }
        }
        print!("\n");
    }

    println!(
        "Reader reported {} successful packets and {} failed packets", pass, fail
    );

    Ok(())
}