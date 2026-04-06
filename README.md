# ATLAS — Avionic Telemetry Logging and Analysis System

ATLAS is a senior capstone avionics data acquisition and analysis system. It streams real-time sensor telemetry from an STM32F446RET6 microcontroller over UART to a Rust backend server, which broadcasts data via WebSocket to a React dashboard. The system supports live hardware sessions, a built-in simulator, fault injection, and binary session logging.

---

## Architecture

| Layer | Technology | Role |
|---|---|---|
| Firmware | C (STM32F446RET6) | Sensor acquisition, state machine, UART TX/RX |
| Protocol | Rust (`ground/protocol`) | Binary framing, CRC-16/CCITT-FALSE, encode/decode |
| Backend | Rust + Axum (`ground/backend`) | Serial bridge, command dispatch, WebSocket broadcast |
| UI | React + Vite + TypeScript | Real-time dashboard, graphs, command console |
| Simulator | Rust (`ground/simulator`) | Synthetic telemetry with fault injection |
| Logger | Rust (`ground/logger`) | Binary `.atl` session recording |
| Replay | Rust (`ground/replay`) | Playback of logged sessions *(in progress — not wired to UI)* |

---

## Project Structure

```
ATLAS/
├── docs/
│   ├── requirements.md       # System requirements
│   ├── protocol.md           # Binary protocol specification
│   ├── modes.md              # Operating mode definitions
│   ├── use_scenarios.md      # Key usage scenarios
│   └── firmware.md           # Firmware design document
│
├── firmware/
│   └── atlas_firmware/       # STM32CubeIDE project
│       ├── Inc/              # packet.h, uart.h, adc.h, normal.h
│       └── Src/              # main.c, packet.c, uart.c, adc.c, normal.c
│
├── ground/
│   ├── backend/              # Axum HTTP + WebSocket bridge (main entry point)
│   ├── protocol/             # Packet encode/decode and CRC — shared library
│   ├── simulator/            # Synthetic telemetry generator with fault injection
│   ├── logger/               # Binary .atl session logging
│   └── replay/               # Session playback (work in progress)
│
├── ui/                       # React + Vite + Tailwind dashboard
│   └── src/
│       ├── components/       # TelemetryPanel, CommandConsole, FaultPanel, GraphView, ReplaySystem
│       ├── contexts/         # TelemetryProvider — global WebSocket state and command dispatch
│       └── lib/              # types.ts — shared TypeScript interfaces
│
└── tests/                    # Placeholder (protocol tests live in ground/protocol/tests/)
```

---

## Prerequisites

### Firmware
- STM32CubeIDE (or ARM GCC + Make)
- ST-Link V2 programmer

### Backend and Tools
- Rust 1.70+ — install via [rustup.rs](https://rustup.rs/)

### UI
- Node.js 18+ and npm

### Hardware Serial (Windows + WSL)
When running live hardware on Windows, the backend runs inside WSL and the STM32 USB-serial device is forwarded using `usbipd-win`. See the [Live Hardware Setup](#live-hardware-setup-windows--wsl) section below.

---

## Getting Started

### 1. Start the Backend

**Simulator mode (no hardware required):**
```bash
cargo run --manifest-path ground/backend/Cargo.toml
```
Then click **SIMULATOR MODE** in the UI to start a simulated telemetry session.

**Live hardware mode (see full setup below for WSL):**
```bash
export ATLAS_SERIAL_PORT=/dev/ttyACM0   # adjust to your port
export ATLAS_SERIAL_BAUD=115200
cargo run --manifest-path ground/backend/Cargo.toml
```

The backend listens on `http://0.0.0.0:3000` by default. The UI connects to it automatically.

### 2. Start the UI

```bash
cd ui
npm install
npm run dev
```

Open `http://localhost:5173` in your browser. The dashboard connects to the backend WebSocket automatically on load.

---

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `ATLAS_BIND_ADDR` | `0.0.0.0:3000` | Backend HTTP/WebSocket bind address |
| `ATLAS_SERIAL_PORT` | `COM7` | Serial port for live hardware session |
| `ATLAS_SERIAL_BAUD` | `115200` | Serial baud rate |

---

## Live Hardware Setup (Windows + WSL)

The backend must run inside WSL to access the STM32 serial device on Windows. Use `usbipd-win` to forward the USB device.

**1. Attach the STM32 USB device to WSL** (run in Windows admin PowerShell):
```powershell
usbipd attach --wsl --busid <busid> --auto-attach
```
Find the correct `busid` with `usbipd list`.

**2. Confirm the device in WSL:**
```bash
ls -l /dev/ttyACM0
```

**3. Start the backend in WSL:**
```bash
cd /mnt/c/Users/<you>/revival_project/ATLAS/ground/backend
export ATLAS_SERIAL_PORT=/dev/ttyACM0
export ATLAS_SERIAL_BAUD=115200
cargo run
```

**4. Start the UI in Windows PowerShell:**
```powershell
cd ui
npm run dev
```

If the serial device disconnects, re-run the `usbipd attach` command and restart the backend. The UI will reconnect automatically once the backend re-establishes the serial link.

---

## Session Logs

Each live session writes two files to `ground/backend/logs/`:

| File | Contents |
|---|---|
| `live_<port>_<timestamp>_packets.atl` | Raw binary frames (length-prefixed) |
| `live_<port>_<timestamp>_errors.txt` | Protocol error log with summary statistics |

---

## Running Tests

Protocol unit tests (encoder, decoder, CRC):
```bash
cargo test --manifest-path ground/protocol/Cargo.toml
```

UI build validation (TypeScript compile check):
```bash
cd ui && npm run build
```

---

## Operating Modes

| Mode | Telemetry Rate | Description |
|---|---|---|
| IDLE | 500 ms | Powered, waiting for commands |
| NORMAL | 100 ms | Primary operation, full telemetry |
| SAFE | 200 ms | Reduced functionality, active critical fault |
| DIAGNOSTIC | 100 ms | Self-test and maintenance |

See [docs/modes.md](docs/modes.md) for transition rules.

---

## Fault Injection

The command console supports injecting faults for testing:

| Command | Param | Effect |
|---|---|---|
| `SMALL_FAULT` | `1` | Inject CRC error |
| `SMALL_FAULT` | `2` | Inject bad sync byte |
| `SMALL_FAULT` | `3` | Inject length mismatch |
| `SMALL_FAULT` | `4` | Inject sequence gap |
| `MAJOR_FAULT` | `1` | Temperature spike (forces SAFE) |
| `MAJOR_FAULT` | `2` | Voltage spike (forces SAFE) |
| `MAJOR_FAULT` | `3` | Light sensor failure (forces SAFE) |
| `MAJOR_FAULT` | `4` | Temperature dip (forces SAFE) |
| `MAJOR_FAULT` | `5` | Voltage drop (forces SAFE) |

---

## Documentation

- [System Requirements](docs/requirements.md)
- [Protocol Specification](docs/protocol.md)
- [Operating Modes](docs/modes.md)
- [Use Scenarios](docs/use_scenarios.md)
- [Firmware Design](docs/firmware.md)

---

## License

See [LICENSE](LICENSE) for details.
