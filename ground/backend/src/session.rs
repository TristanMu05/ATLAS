use atlas_protocol::{decode_packet, encode_packet, Packet, MIN_FRAME_LEN, SYNC_WORD};
use atlas_simulator::simulate;
use serde::{Deserialize, Serialize};
use serialport::SerialPort;
use std::{
    collections::VecDeque,
    fs::{self, File},
    io::{self, BufWriter, Read, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::broadcast;

const MESSAGE_ID_TELEMETRY: u8 = 0x01;
const MESSAGE_ID_COMMAND: u8 = 0x02;
const MESSAGE_ID_ACK: u8 = 0x03;
const MESSAGE_ID_COMMAND_RESPONSE: u8 = 0x04;
const MESSAGE_ID_EVENT: u8 = 0x05;

const COMMAND_ID_SET_MODE: u8 = 0x01;
const COMMAND_ID_REQUEST_STATUS: u8 = 0x02;
const COMMAND_ID_CLEAR_FAULTS: u8 = 0x03;
const COMMAND_ID_SET_TELEMETRY_ENABLE: u8 = 0x04;
const COMMAND_ID_SMALL_FAULT: u8 = 0x05;
const COMMAND_ID_MAJOR_FAULT: u8 = 0x06;

const MAX_PAYLOAD_LEN: usize = 256;
const LIVE_RECONNECT_DELAY: Duration = Duration::from_millis(750);

struct LiveSessionLog {
    frame_writer: BufWriter<File>,
    error_writer: BufWriter<File>,
}

impl LiveSessionLog {
    fn create(port_name: &str) -> io::Result<Self> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_secs();
        let safe_port_name = sanitize_log_label(port_name);

        fs::create_dir_all("logs")?;

        let frame_writer = BufWriter::new(File::create(format!(
            "logs/live_{safe_port_name}_{now}_packets.atl"
        ))?);
        let error_writer = BufWriter::new(File::create(format!(
            "logs/live_{safe_port_name}_{now}_errors.txt"
        ))?);

        Ok(Self {
            frame_writer,
            error_writer,
        })
    }

    fn log_frame(&mut self, frame: &[u8]) -> io::Result<()> {
        let len = frame.len() as u32;
        self.frame_writer.write_all(&len.to_le_bytes())?;
        self.frame_writer.write_all(frame)?;
        self.frame_writer.flush()
    }

    fn log_error(&mut self, msg: impl std::fmt::Display) -> io::Result<()> {
        self.error_writer.write_all(format!("{msg}\n").as_bytes())?;
        self.error_writer.flush()
    }
}

fn sanitize_log_label(value: &str) -> String {
    let mut result = String::with_capacity(value.len());

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            result.push(ch);
        } else {
            result.push('_');
        }
    }

    if result.is_empty() {
        "session".to_string()
    } else {
        result
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionKind {
    Live,
    Simulation,
}

pub struct ActiveSession {
    pub kind: SessionKind,
    pub started_at: Instant,
    pub stop_flag: Arc<AtomicBool>,
    pub worker_alive: Arc<AtomicBool>,
    pub link_connected: Arc<AtomicBool>,
    pub command_tx: Option<mpsc::Sender<Vec<u8>>>,
    pub next_command_sequence: u16,
    pub label: String,
}

#[derive(Debug, Deserialize)]
pub struct CommandRequest {
    pub cmd: String,
    #[serde(default)]
    pub param: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum BackendMessage {
    FrameReceived {
        seq: u16,
        timestamp: u32,
        payload: Vec<u8>,
    },
    Stats {
        ok: u32,
        dropped: u32,
    },
    Error {
        kind: String,
        detail: String,
    },
    Ack {
        code: u8,
    },
    CommandResponse {
        code: u8,
    },
    Event {
        code: u8,
    },
    LinkState {
        state: String,
        detail: String,
    },
}

#[derive(Clone)]
pub struct LiveConfig {
    pub port_name: String,
    pub baud_rate: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LiveLinkState {
    Connecting,
    Connected,
    Reconnecting,
    Disconnected,
}

impl LiveLinkState {
    fn as_str(self) -> &'static str {
        match self {
            LiveLinkState::Connecting => "CONNECTING",
            LiveLinkState::Connected => "CONNECTED",
            LiveLinkState::Reconnecting => "RECONNECTING",
            LiveLinkState::Disconnected => "DISCONNECTED",
        }
    }
}

struct StreamStats {
    ok: u32,
    dropped: u32,
    expected_seq: Option<u16>,
}

impl StreamStats {
    fn new() -> Self {
        Self {
            ok: 0,
            dropped: 0,
            expected_seq: None,
        }
    }

    fn process_frame(
        &mut self,
        frame: &[u8],
        tx: &broadcast::Sender<String>,
        mut log: Option<&mut LiveSessionLog>,
    ) -> io::Result<()> {
        match decode_packet(frame) {
            Ok(packet) => {
                self.emit_packet(&packet, tx, log.as_deref_mut())?;
                self.record_sequence(packet.sequence, tx, log.as_deref_mut())?;
                self.emit_stats(tx)?;
                Ok(())
            }
            Err(error) => {
                self.dropped = self.dropped.saturating_add(1);
                if let Some(expected) = self.expected_seq {
                    self.expected_seq = Some(expected.wrapping_add(1));
                }

                self.emit_error(tx, "DecodeError", format!("{error:?}"), log.as_deref_mut())?;
                self.emit_stats(tx)
            }
        }
    }

    fn emit_packet(
        &mut self,
        packet: &Packet,
        tx: &broadcast::Sender<String>,
        log: Option<&mut LiveSessionLog>,
    ) -> io::Result<()> {
        match packet.message_id {
            MESSAGE_ID_TELEMETRY => self.emit(
                tx,
                BackendMessage::FrameReceived {
                    seq: packet.sequence,
                    timestamp: packet.timestamp,
                    payload: packet.payload.clone(),
                },
            ),
            MESSAGE_ID_ACK => {
                if let Some(code) = packet.payload.first().copied() {
                    self.emit(tx, BackendMessage::Ack { code })
                } else {
                    self.emit_error(tx, "Protocol", "ACK packet missing payload".to_string(), log)
                }
            }
            MESSAGE_ID_COMMAND_RESPONSE => {
                if let Some(code) = packet.payload.first().copied() {
                    self.emit(tx, BackendMessage::CommandResponse { code })
                } else {
                    self.emit_error(
                        tx,
                        "Protocol",
                        "Command response packet missing payload".to_string(),
                        log,
                    )
                }
            }
            MESSAGE_ID_EVENT => {
                if let Some(code) = packet.payload.first().copied() {
                    self.emit(tx, BackendMessage::Event { code })
                } else {
                    self.emit_error(tx, "Protocol", "Event packet missing payload".to_string(), log)
                }
            }
            other => self.emit_error(
                tx,
                "Protocol",
                format!("Unsupported message id 0x{other:02X}"),
                log,
            ),
        }
    }

    fn record_sequence(
        &mut self,
        sequence: u16,
        tx: &broadcast::Sender<String>,
        log: Option<&mut LiveSessionLog>,
    ) -> io::Result<()> {
        match self.expected_seq {
            None => {
                self.ok = self.ok.saturating_add(1);
                self.expected_seq = Some(sequence.wrapping_add(1));
            }
            Some(expected) if sequence == expected => {
                self.ok = self.ok.saturating_add(1);
                self.expected_seq = Some(expected.wrapping_add(1));
            }
            Some(expected) if sequence == expected.wrapping_sub(1) => {
                self.emit_error(
                    tx,
                    "Sequence",
                    format!("Duplicate packet: expected {expected}, got {sequence}"),
                    log,
                )?;
            }
            Some(expected) if sequence > expected => {
                self.ok = self.ok.saturating_add(1);
                self.dropped = self
                    .dropped
                    .saturating_add(sequence.wrapping_sub(expected) as u32);
                self.expected_seq = Some(sequence.wrapping_add(1));
                self.emit_error(
                    tx,
                    "Sequence",
                    format!("Skipped/dropped packet(s): expected {expected}, got {sequence}"),
                    log,
                )?;
            }
            Some(expected) => {
                self.emit_error(
                    tx,
                    "Sequence",
                    format!("Out of order packet: expected {expected}, got {sequence}"),
                    log,
                )?;
            }
        }

        Ok(())
    }

    fn emit_error(
        &self,
        tx: &broadcast::Sender<String>,
        kind: impl Into<String>,
        detail: String,
        log: Option<&mut LiveSessionLog>,
    ) -> io::Result<()> {
        let kind = kind.into();
        if let Some(log) = log {
            log.log_error(format!("[{kind}] {detail}"))?;
        }

        self.emit(
            tx,
            BackendMessage::Error {
                kind,
                detail,
            },
        )
    }

    fn emit_stats(&self, tx: &broadcast::Sender<String>) -> io::Result<()> {
        self.emit(
            tx,
            BackendMessage::Stats {
                ok: self.ok,
                dropped: self.dropped,
            },
        )
    }

    fn emit_link_state(
        &self,
        tx: &broadcast::Sender<String>,
        state: LiveLinkState,
        detail: String,
    ) -> io::Result<()> {
        self.emit(
            tx,
            BackendMessage::LinkState {
                state: state.as_str().to_string(),
                detail,
            },
        )
    }

    fn emit(&self, tx: &broadcast::Sender<String>, update: BackendMessage) -> io::Result<()> {
        let payload = serde_json::to_string(&update).map_err(io::Error::other)?;
        let _ = tx.send(payload);
        Ok(())
    }

    fn reset_sequence_tracking(&mut self) {
        self.expected_seq = None;
    }
}

fn emit_link_state_if_changed(
    stats: &StreamStats,
    tx: &broadcast::Sender<String>,
    last_reported_state: &mut Option<(LiveLinkState, String)>,
    state: LiveLinkState,
    detail: String,
    log: Option<&mut LiveSessionLog>,
) -> io::Result<()> {
    let should_emit = match last_reported_state.as_ref() {
        Some((previous_state, previous_detail)) => {
            *previous_state != state || previous_detail != &detail
        }
        None => true,
    };

    if should_emit {
        if let Some(log) = log {
            log.log_error(format!("[LinkState:{}] {}", state.as_str(), detail))?;
        }
        stats.emit_link_state(tx, state, detail.clone())?;
        *last_reported_state = Some((state, detail));
    }

    Ok(())
}

#[derive(Default)]
struct FrameExtractor {
    buffer: Vec<u8>,
}

impl FrameExtractor {
    fn push_bytes(
        &mut self,
        bytes: &[u8],
        stats: &mut StreamStats,
        tx: &broadcast::Sender<String>,
        mut log: Option<&mut LiveSessionLog>,
    ) -> io::Result<()> {
        self.buffer.extend_from_slice(bytes);

        loop {
            if self.buffer.len() < 2 {
                return Ok(());
            }

            let sync_index = self
                .buffer
                .windows(SYNC_WORD.len())
                .position(|window| window == SYNC_WORD);

            match sync_index {
                Some(0) => {}
                Some(index) => {
                    self.buffer.drain(..index);
                    stats.emit_error(
                        tx,
                        "InvalidSync",
                        format!("Discarded {index} stray byte(s) before sync"),
                        log.as_deref_mut(),
                    )?;
                }
                None => {
                    let retained = self.buffer.last().copied();
                    if !self.buffer.is_empty() {
                        self.buffer.clear();
                        if let Some(last_byte) = retained.filter(|byte| *byte == SYNC_WORD[0]) {
                            self.buffer.push(last_byte);
                        }
                        stats.emit_error(
                            tx,
                            "InvalidSync",
                            "Discarded buffered bytes while searching for sync".to_string(),
                            log.as_deref_mut(),
                        )?;
                    }
                    return Ok(());
                }
            }

            if self.buffer.len() < MIN_FRAME_LEN {
                return Ok(());
            }

            let payload_len = u16::from_be_bytes([self.buffer[3], self.buffer[4]]) as usize;
            if payload_len > MAX_PAYLOAD_LEN {
                self.buffer.drain(..2);
                stats.emit_error(
                    tx,
                    "LengthMismatch",
                    format!("Declared payload too large: {payload_len} bytes"),
                    log.as_deref_mut(),
                )?;
                continue;
            }

            let frame_len = MIN_FRAME_LEN + payload_len;
            if self.buffer.len() < frame_len {
                return Ok(());
            }

            let frame: Vec<u8> = self.buffer.drain(..frame_len).collect();
            if let Some(log) = log.as_deref_mut() {
                log.log_frame(&frame)?;
            }
            stats.process_frame(&frame, tx, log.as_deref_mut())?;
        }
    }
}

pub fn run_simulation_session(
    tx: broadcast::Sender<String>,
    stop_flag: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stats = StreamStats::new();

    let result = simulate(|frame| {
        if stop_flag.load(Ordering::Relaxed) {
            return Err(io::Error::other("session stopped"));
        }

        stats.process_frame(frame, &tx, None)
    });

    if let Err(error) = result {
        if error.to_string() != "session stopped" {
            let _ = stats.emit_error(&tx, "Simulation", error.to_string(), None);
        }
    }

    Ok(())
}

pub fn run_live_session(
    tx: broadcast::Sender<String>,
    stop_flag: Arc<AtomicBool>,
    command_rx: mpsc::Receiver<Vec<u8>>,
    config: LiveConfig,
    link_connected: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stats = StreamStats::new();
    let mut extractor = FrameExtractor::default();
    let mut read_buffer = [0u8; 256];
    let mut pending_commands = VecDeque::new();
    let mut port: Option<Box<dyn SerialPort>> = None;
    let mut last_reported_state: Option<(LiveLinkState, String)> = None;
    let mut has_connected_once = false;
    let mut session_log = match LiveSessionLog::create(&config.port_name) {
        Ok(log) => Some(log),
        Err(error) => {
            let _ = stats.emit_error(
                &tx,
                "Logging",
                format!("failed to initialize live logs: {error}"),
                None,
            );
            None
        }
    };

    emit_link_state_if_changed(
        &stats,
        &tx,
        &mut last_reported_state,
        LiveLinkState::Connecting,
        format!("opening {}", config.port_name),
        session_log.as_mut(),
    )?;

    while !stop_flag.load(Ordering::Relaxed) {
        while let Ok(frame) = command_rx.try_recv() {
            pending_commands.push_back(frame);
        }

        if port.is_none() {
            match open_live_port(&config) {
                Ok(opened_port) => {
                    port = Some(opened_port);
                    extractor = FrameExtractor::default();
                    stats.reset_sequence_tracking();
                    link_connected.store(true, Ordering::Relaxed);
                    has_connected_once = true;
                    emit_link_state_if_changed(
                        &stats,
                        &tx,
                        &mut last_reported_state,
                        LiveLinkState::Connected,
                        format!("connected to {}", config.port_name),
                        session_log.as_mut(),
                    )?;
                }
                Err(error) => {
                    link_connected.store(false, Ordering::Relaxed);
                    emit_link_state_if_changed(
                        &stats,
                        &tx,
                        &mut last_reported_state,
                        if has_connected_once {
                            LiveLinkState::Reconnecting
                        } else {
                            LiveLinkState::Connecting
                        },
                        format!("waiting for {}: {error}", config.port_name),
                        session_log.as_mut(),
                    )?;
                    std::thread::sleep(LIVE_RECONNECT_DELAY);
                    continue;
                }
            }
        }

        let mut connection_error: Option<String> = None;

        if let Some(active_port) = port.as_mut() {
            while let Some(frame) = pending_commands.pop_front() {
                if let Err(error) = active_port.write_all(&frame).and_then(|_| active_port.flush()) {
                    pending_commands.push_front(frame);
                    connection_error = Some(error.to_string());
                    break;
                } else if let Some(log) = session_log.as_mut() {
                    let _ = log.log_error(format!("[CommandTx] {:02X?}", frame));
                }
            }

            if connection_error.is_none() {
                match active_port.read(&mut read_buffer) {
                    Ok(read_len) if read_len > 0 => {
                        extractor.push_bytes(&read_buffer[..read_len], &mut stats, &tx, session_log.as_mut())?;
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == io::ErrorKind::TimedOut => {}
                    Err(error) => {
                        connection_error = Some(error.to_string());
                    }
                }
            }
        }

        if let Some(detail) = connection_error {
            port = None;
            extractor = FrameExtractor::default();
            link_connected.store(false, Ordering::Relaxed);
            emit_link_state_if_changed(
                &stats,
                &tx,
                &mut last_reported_state,
                LiveLinkState::Reconnecting,
                format!("lost {}: {detail}", config.port_name),
                session_log.as_mut(),
            )?;
            std::thread::sleep(LIVE_RECONNECT_DELAY);
        }
    }

    link_connected.store(false, Ordering::Relaxed);
    emit_link_state_if_changed(
        &stats,
        &tx,
        &mut last_reported_state,
        LiveLinkState::Disconnected,
        format!("stopped {}", config.port_name),
        session_log.as_mut(),
    )?;

    Ok(())
}

pub fn open_live_port(config: &LiveConfig) -> Result<Box<dyn SerialPort>, serialport::Error> {
    serialport::new(&config.port_name, config.baud_rate)
        .timeout(Duration::from_millis(100))
        .open()
}

pub fn build_command_frame(
    request: &CommandRequest,
    sequence: u16,
    timestamp_ms: u32,
) -> Result<Vec<u8>, String> {
    let command_name = request.cmd.trim().to_ascii_uppercase();
    let param = request.param.trim().to_ascii_uppercase();

    let payload = match command_name.as_str() {
        "SET_MODE" => vec![
            COMMAND_ID_SET_MODE,
            parse_mode_param(&param).ok_or_else(|| format!("Unsupported mode '{param}'"))?,
        ],
        "REQUEST_STATUS" => vec![COMMAND_ID_REQUEST_STATUS],
        "CLEAR_FAULTS" => vec![COMMAND_ID_CLEAR_FAULTS],
        "SET_TELEMETRY_ENABLE" | "SET_TELEM" | "TELEMETRY" => vec![
            COMMAND_ID_SET_TELEMETRY_ENABLE,
            parse_telemetry_param(&param)
                .ok_or_else(|| format!("Unsupported telemetry value '{param}'"))?,
        ],
        "SMALL_FAULT" | "INJECT_SMALL_FAULT" => vec![
            COMMAND_ID_SMALL_FAULT,
            parse_small_fault_param(&param)
                .ok_or_else(|| format!("Unsupported small fault value '{param}'"))?,
        ],
        "MAJOR_FAULT" | "INJECT_MAJOR_FAULT" => vec![
            COMMAND_ID_MAJOR_FAULT,
            parse_major_fault_param(&param)
                .ok_or_else(|| format!("Unsupported major fault value '{param}'"))?,
        ],
        other => return Err(format!("Unsupported command '{other}'")),
    };

    Ok(encode_packet(&Packet {
        message_id: MESSAGE_ID_COMMAND,
        sequence,
        timestamp: timestamp_ms,
        payload,
    }))
}

fn parse_mode_param(param: &str) -> Option<u8> {
    match param {
        "IDLE" => Some(0x00),
        "NORMAL" | "NOMINAL" => Some(0x01),
        "SAFE" => Some(0x02),
        "DIAGNOSTIC" => Some(0x03),
        _ => None,
    }
}

fn parse_telemetry_param(param: &str) -> Option<u8> {
    match param {
        "0" | "OFF" | "DISABLE" | "DISABLED" | "FALSE" => Some(0x00),
        "1" | "ON" | "ENABLE" | "ENABLED" | "TRUE" => Some(0x01),
        _ => None,
    }
}

fn parse_small_fault_param(param: &str) -> Option<u8> {
    match param {
        "1" | "CRC" | "CRC_ERROR" => Some(0x01),
        "2" | "SYNC" | "SYNC_ERROR" => Some(0x02),
        "3" | "LENGTH" | "LENGTH_ERROR" => Some(0x03),
        "4" | "SEQ" | "SEQUENCE" | "SEQ_ERROR" | "SEQUENCE_ERROR" => Some(0x04),
        _ => None,
    }
}

fn parse_major_fault_param(param: &str) -> Option<u8> {
    match param {
        "1" | "TEMP_SPIKE" | "OVER_TEMPERATURE" => Some(0x01),
        "2" | "VOLTAGE_SPIKE" | "HIGH_VOLTAGE" => Some(0x02),
        "3" | "LIGHT_SENSOR_FAILURE" | "LIGHT_FAIL" | "LIGHT_FAILURE" => Some(0x03),
        "4" | "TEMP_DIP" | "UNDER_TEMPERATURE" => Some(0x04),
        "5" | "VOLTAGE_DROP" | "LOW_VOLTAGE" => Some(0x05),
        _ => None,
    }
}
