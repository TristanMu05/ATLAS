use atlas_simulator::simulate;
use std::fs::File;
use std::io::{BufWriter, Write};
use atlas_protocol::{decode_packet};
use std::net::UdpSocket;

/*
 * 
 * The logger is going to log our data, either from an input port accepting data from
 * the stm32, or our simulator if that port is not on.
 * 
 
#[allow(dead_code)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
 
    let mode = std::env::args().nth(1).unwrap_or("sim".to_string());

    match mode.as_str() {
        "live" => live()?,
        "sim" => simulation(|update| {
            // Send to UI TODO
            Ok(())
        })?,
        _ => return Err("usage: logger <live|sim>".into()),
    }

    Ok(())
}
*/
use std::time::{SystemTime, UNIX_EPOCH};
pub fn live() -> Result<(), Box<dyn std::error::Error>> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();

    std::fs::create_dir_all("logs")?;
    let file = File::create(format!("logs/live_{now}_logs.atl"))?;
    let err_file = File::create(format!("logs/live_{now}_errors.txt"))?;
    let mut writer = BufWriter::new(file);
    let mut err_writer = BufWriter::new(err_file);
    let mut ok = 0;
    let mut dropped = 0;
    let mut expected_seq: Option<u16> = None;
    let mut stats: ErrorStats = ErrorStats::default();

    // start listening to live port
    let stream = UdpSocket::bind("[IP_ADDRESS]")?;
    stream.set_broadcast(true)?;
    
    loop {
        // Look for end of connection
        // to-do
        // look for sync bytes
        let mut buf = [0u8; 256];
        let Ok((amt, _src)) = stream.recv_from(&mut buf) else { todo!() };
        let frame = &buf[..amt];

        // Log frame
        log_frame(&mut writer, frame)?;

        // Decode and identify errors, log errors
        match decode_packet(&frame) {
            Ok(packet) => {
                match expected_seq {
                    None => { // First valid sequence
                        ok += 1;
                        expected_seq = Some(packet.sequence.wrapping_add(1));
                    }
                    Some(expected) if packet.sequence == expected => { // Expected sequence came through
                        ok+=1;
                        expected_seq = Some(expected.wrapping_add(1));
                    }
                    Some(expected) if packet.sequence == expected.wrapping_sub(1) => { // Imidiate duplicate sequence
                        log_event(
                            &mut err_writer,
                            &mut stats,
                            LogEvent::Sequence(SequenceError::Duplicate),
                            format!("duplicate packet: expected {expected}, got {}", packet.sequence)
                        )?;
                    }
                    Some(expected) if packet.sequence > expected => { // Skipped sequence
                        ok+=1;
                        dropped += packet.sequence.wrapping_sub(expected);
                        log_event(
                            &mut err_writer,
                            &mut stats,
                            LogEvent::Sequence(SequenceError::Skipped),
                            format!("Skipped/dropped packet(s): expected {expected}, got {}", packet.sequence),
                        )?;
                    }
                    Some(expected) => { // out of order sequence
                        log_event(
                            &mut err_writer,
                            &mut stats,
                            LogEvent::Sequence(SequenceError::OutOfOrder),
                            format!("Out of order packet: expected {expected}, got {}", packet.sequence)
                        )?;
                    }

                }
            }
            Err(e) => {
                let event = LogEvent::from(&e);
                log_event(&mut err_writer, &mut stats, event, format!("{:?}", e))?;
                expected_seq = expected_seq.map(|seq| seq.wrapping_add(1));
                dropped+=1;
            }
        };

        // Need to report this is information to the backend to send to the UI

        println!("ok: {}, dropped: {}, expected_seq: {:?}", ok, dropped, expected_seq);
    }

    let _ = log_error(&mut err_writer, format!(
        "\nLIVE LOGGING RESULTS:\nTotal frames: {}\nSuccessful frames: {}\nFailed frames: {}\nBreakdown: {}\n",
        ok+dropped,
        ok,
        dropped,
        stats.to_string()
    ));
    Ok(())
}

pub fn simulation<F>(mut on_update: F) -> Result<(), Box<dyn std::error::Error>> 
where F: FnMut(LoggerUpdate) -> std::io::Result<()> {
    std::fs::create_dir_all("logs")?;
    let file = File::create("logs/sim_packets.atl")?;
    let mut writer  = BufWriter::new(file);
    let errors = File::create("logs/sim_errors.txt")?;
    let mut err_writer = BufWriter::new(errors);
    let mut ok = 0;
    let mut dropped = 0;
    let mut expected_seq: Option<u16> = None;
    let mut stats: ErrorStats = ErrorStats::default();

    simulate(|frame| {
        println!("frame: {:?}", frame);
        log_frame(&mut writer, frame)?;

        match decode_packet(&frame) {
            Ok(packet) => {
                on_update(LoggerUpdate::FrameReceived { seq: packet.sequence, timestamp: packet.timestamp, payload: packet.payload })?;
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
                        log_event(
                            &mut err_writer,
                            &mut stats,
                            LogEvent::Sequence(SequenceError::Duplicate),
                            format!("Duplicate packet: expected {expected}, got {}", packet.sequence)
                        )?;
                        on_update(LoggerUpdate::Error { kind: "...".to_string(), detail: format!("Duplicate packet: expected {expected}, got {}", packet.sequence),})?;
                    }
                    Some(expected) if packet.sequence > expected => { // Skipped seq
                        ok+=1;
                        dropped += packet.sequence.wrapping_sub(expected);
                        log_event(
                            &mut err_writer,
                            &mut stats,
                            LogEvent::Sequence(SequenceError::Skipped),
                            format!("Skipped/dropped packet(s): expected {expected}, got {}", packet.sequence)
                        )?;
                        expected_seq = Some(packet.sequence.wrapping_add(1));
                        on_update(LoggerUpdate::Error { kind: "...".to_string(), detail: format!("Skipped/dropped packet(s): expected {expected}, got {}", packet.sequence),})?;
                    }
                    Some(expected) => { // Out of order seq
                        log_event(
                            &mut err_writer,
                            &mut stats,
                            LogEvent::Sequence(SequenceError::OutOfOrder),
                            format!("Out-of-order packet: expected {expected}, got {}", packet.sequence)
                        )?;
                        on_update(LoggerUpdate::Error { kind: "...".to_string(), detail: format!("Out of order packet: expected {expected}, got {}", packet.sequence),})?;

                    }
                }
            }
            Err(e) => {
                let event = LogEvent::from(&e); 
                log_event(&mut err_writer, &mut stats, event, format!("{:?}", e))?;
                expected_seq = expected_seq.map(|seq| seq.wrapping_add(1));
                dropped+=1;
                on_update(LoggerUpdate::Error { kind: "...".to_string(), detail: format!("{:?}", e),})?;
            }
        };
        on_update(LoggerUpdate::Stats { ok: ok as u32, dropped: dropped as u32 })?;
        println!("ok={}, dropped={}", ok, dropped);


        Ok(())
    })?;
    let _ = log_error(&mut err_writer, format!(
        "\nSIMULATION RESULTS:\nTotal frames: {}\nSuccessful frames: {}\nFailed frames: {}\nBreakdown: {}\n",
        ok+dropped,
        ok,
        dropped,
        stats.to_string()
    ));
    Ok(())
}

fn log_frame(w: &mut BufWriter<File>, frame: &[u8]) -> std::io::Result<()> {
    let len = frame.len() as u32;
    w.write_all(&len.to_le_bytes())?;
    w.write_all(&frame)?;

    w.flush()?;
    Ok(())
}

#[allow(dead_code)]
fn log_error(w: &mut BufWriter<File>, msg: impl std::fmt::Display) -> std::io::Result<()> {
    w.write_all(format!("{msg}\n").as_bytes())?;
    w.flush()?;
    Ok(())
}

use atlas_protocol::DecodeError;
#[derive(Debug)]
enum LogEvent {
    PacketTooShort,
    InvalidSync,
    LengthMismatch,
    CrcMismatch,
    Sequence(SequenceError),
}

#[derive(Debug)]
enum SequenceError {
    Skipped,
    Duplicate,
    OutOfOrder,
}

#[derive(Default,Debug)]
struct ErrorStats {
    packet_too_short: usize,
    invalid_sync: usize,
    length_mismatch: usize,
    crc_mismatch: usize,
    skipped: usize,
    duplicate: usize,
    out_of_order: usize,
}

impl ErrorStats {
    fn record(&mut self, event: &LogEvent) {
        match event {
            LogEvent::PacketTooShort => self.packet_too_short += 1,
            LogEvent::InvalidSync => self.invalid_sync += 1,
            LogEvent::LengthMismatch => self.length_mismatch += 1,
            LogEvent::CrcMismatch => self.crc_mismatch += 1,
            LogEvent::Sequence(seq_err) => match seq_err {
                SequenceError::Skipped => self.skipped += 1,
                SequenceError::Duplicate => self.duplicate += 1,
                SequenceError::OutOfOrder => self.out_of_order += 1,
            }
        }
    }
    fn to_string(&self) -> String {
        format!(
            "Packet too short: {}\n
            Invalid sync: {}\n
            Length mismatch: {}\n
            CRC mismatch: {}\n
            Skipped: {}\n
            Duplicate: {}\n
            Out of order: {}",
            self.packet_too_short,
            self.invalid_sync,
            self.length_mismatch,
            self.crc_mismatch,
            self.skipped,
            self.duplicate,
            self.out_of_order,
        )
    }
}

impl From<&DecodeError> for LogEvent {
    fn from(err: &DecodeError) -> Self {
        match err {
            DecodeError::PacketTooShort { .. } => LogEvent::PacketTooShort,
            DecodeError::InvalidSync { .. } => LogEvent::InvalidSync,
            DecodeError::LengthMismatch { .. } => LogEvent::LengthMismatch,
            DecodeError::CrcMismatch { .. } => LogEvent::CrcMismatch,
        }
    }
}

fn log_event(w: &mut BufWriter<File>, stats: &mut ErrorStats, event: LogEvent, detail: impl std::fmt::Display,) -> std::io::Result<()> {
    stats.record(&event);
    w.write_all(format!("[{:?}] {detail}\n", event).as_bytes())?;
    w.flush()?;
    Ok(())
}

use serde::Serialize;
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum LoggerUpdate {
    FrameReceived { seq: u16, timestamp: u32, payload: Vec<u8> },
    Stats { ok: u32, dropped: u32 },
    Error { kind: String, detail: String },
}