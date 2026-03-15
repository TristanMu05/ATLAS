use atlas_simulator::simulate;
use std::fs::File;
use std::io::{BufWriter, Write};
use atlas_protocol::{decode_packet};

/*
 * 
 * The logger is going to log our data, either from an input port accepting data from
 * the stm32, or our simulator if that port is not on.
 * 
 */
fn main() -> Result<(), Box<dyn std::error::Error>> {

    let mode = std::env::args().nth(1).unwrap_or("sim".to_string());

    match mode.as_str() {
        "live" => live()?,
        "sim" => simulation()?,
        _ => return Err("usage: logger <live|sim>".into()),
    }

    Ok(())
}

use std::time::{SystemTime, UNIX_EPOCH};
fn live() -> Result<(), Box<dyn std::error::Error>> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();

    let file = File::create(format!("logs/live_{now}_logs.atl"))?;
    let err_file = File::create(format!("logs/live_{now}_errors.txt"))?;
    let mut writer = BufWriter::new(file);
    let mut err_writer = BufWriter::new(err_file);
    let mut ok = 0;
    let mut dropped = 0;
    let mut last_valid_seq = 0;

    // start listening to live port

    // log frame

    // decode and identify errors

    // update last valid sync

    Ok(())
}


fn simulation() -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create("logs/sim_packets.atl")?;
    let mut writer  = BufWriter::new(file);
    let errors = File::create("logs/sim_errors.txt")?;
    let mut err_writer = BufWriter::new(errors);
    let mut ok = 0;
    let mut dropped = 0;
    let mut seq = 0;
    let mut error_counter = (0,0,0,0,0); // packet too short, invalid sync, length mismatch, crc mismatch, seq error

    simulate(|frame| {
        println!("frame: {:?}", frame);
        log_frame(&mut writer, frame)?;

        match decode_packet(&frame) {
            Ok(packet) => {
                if packet.sequence != seq {
                    let _ = log_error(&mut err_writer, format!("Sequece out of order. Expected: {seq}, recieved: {0}", packet.sequence));
                    dropped += 1;
                    seq = packet.sequence + 1;
                }else { 
                    seq += 1;
                    ok += 1; }
            }
            Err(e) => {
                dropped += 1;
                let _  = log_error(&mut err_writer, format!("{:?}", e));
                match e {
                    _ => (),
                }
            }
        }
        println!("ok={}, dropped={}", ok, dropped);


        Ok(())
    })?;
    let _ = log_error(&mut err_writer, format!("\nSIMULATION RESULTS:\nSuccessful packets: {ok}\nDropped packets: {dropped}"));
    Ok(())
}

fn log_frame(w: &mut BufWriter<File>, frame: &[u8]) -> std::io::Result<()> {
    let len = frame.len() as u32;
    w.write_all(&len.to_le_bytes())?;
    w.write_all(&frame)?;

    w.flush()?;
    Ok(())
}

fn log_error(w: &mut BufWriter<File>, msg: impl std::fmt::Display) -> std::io::Result<()> {
    w.write_all(format!("{msg}\n").as_bytes())?;
    w.flush()?;
    Ok(())
}


    
