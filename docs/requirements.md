# System Requirements - ATLAS

## Development Environment Requirements

### Firmware Development
- **ARM GCC Toolchain**: arm-none-eabi-gcc (v10.0 or later)
- **STM32CubeMX**: v6.0 or later for code generation
- **ST-Link V2 Programmer**: For firmware flashing to STM32 microcontroller
- **Build Tools**: Make or CMake
- **Debugging**: OpenOCD or ST-Link GDB server
- **IDE**: Optional (ST Microelectronics IDE, VS Code with extensions, or CLion)

### Ground Station Development
- **Rust**: v1.70 or later ([rustup](https://rustup.rs/))
- **Cargo**: Package manager (included with Rust)
- **Device Drivers**: USB/Serial drivers for ST-Link communication
- **OS Support**: Linux, macOS, Windows
- **Terminal**: ANSI-compatible terminal for CLI interaction

### Simulator Development
- **Rust**: v1.70 or later
- **Cargo**: v1.70 or later
- **Standard Library Dependencies**: tokio, rand (for realistic telemetry generation)

### General Tools
- **Git**: For version control
- **Editor/IDE**: VS Code, Vim, or preferred text editor
- **Serial Monitor**: minicom, PuTTY, or platform equivalent for debugging
- **Terminal**: bash, zsh, or compatible shell

---

## Hardware Requirements

### Embedded System (Firmware Target)
- **Microcontroller**: STM32F446RET6
- **Flash Memory**: 512KB on target MCU
- **RAM**: 128KB on target MCU
- **Serial Communication**: UART interface (3.3V logic levels)
- **Crystal Oscillator**: 8-32 MHz external oscillator
- **Programming**: ST-Link V2 or compatible programmer
- **Power Supply**: 3.3V regulated power
- **Initial Sensor Set**: temperature sensor, light sensor, and ADC-based voltage monitor or simulated battery-monitor source

### Communications Hardware
- **Serial Interface**: USB-to-Serial adapter (CH340, FT232, or equivalent)
- **Baud Rate Support**: 115200 bps minimum
- **Cable**: USB micro or USB-C (depending on adapter)

### Host System (Ground Station)
- **Processor**: Dual-core CPU, 1.5 GHz or faster
- **RAM**: 2GB minimum (4GB recommended)
- **Storage**: 50MB free space for logs and binaries
- **USB Ports**: 1 available for serial communication

---

## Software Requirements

### Firmware Requirements (C/Embedded)

#### Telemetry Collection
- Real-time sensor data acquisition at 10 Hz in NORMAL mode
- Initial telemetry fields: mode, temperature, voltage, light, status flags, fault flags
- Timestamp accuracy ±10ms
- Data buffering for graceful degradation under high load

#### Protocol Implementation
- Custom binary telemetry protocol with CRC-16 validation
- CRC variant: CCITT-FALSE, big-endian
- Multi-byte field byte order: big-endian
- Telemetry payload size: 10 bytes
- Maximum firmware payload size in v1: 64 bytes
- Transmission rate: default 100ms intervals in NORMAL mode
- Error detection and frame synchronization

#### State Machine
- Finite state machine with states: IDLE, NORMAL, SAFE, DIAGNOSTIC
- State transitions triggered by commands or sensor conditions
- Timeout mechanisms for safety (e.g., auto-disarm after 5 minutes)
- Fault detection and logging

#### Command Processing
- Command parsing from ground station
- Single outstanding command at a time in v1
- Explicit ACK / NAK plus command-response behavior
- No authentication in v1
- Response confirmation for critical commands

#### Fault Handling
- Watchdog timer monitoring
- Low voltage detection
- Temperature monitoring
- Graceful error recovery and fault logging

### Ground Station Requirements (Rust)

#### Serial Communication
- Non-blocking serial I/O
- Automatic baud rate detection
- Connection state management
- Configurable timeout periods

#### Protocol Parsing
- Efficient binary protocol decoder
- CRC validation and error detection
- Packet reassembly and buffering
- Logging of protocol errors

#### Telemetry Management
- Real-time data reception and display
- Data storage with timestamp
- CSV export capability
- Data filtering and search

#### Command Interface
- CLI for command transmission
- Command history and favorites
- Parameter validation before transmission
- Command execution status feedback

#### User Interface
- Real-time dashboard display
- Graphical telemetry visualization
- System status indicators
- Log viewer with filtering

#### Data Logging & Replay
- Persistent telemetry storage
- Replay simulation from logs
- Data format conversion utilities
- Compression for long-duration logs

### Simulator Requirements (Rust)

#### Telemetry Generation
- Realistic sensor data simulation
- Configurable noise and drift
- Multiple sensor types support
- Predefined flight profiles

#### Protocol Conformance
- Generates valid ATLAS protocol packets
- Correct CRC calculation
- Realistic timing patterns
- Configurable transmission rate

#### Integration
- Works with ground station without modification
- Supports all telemetry types
- Networkable for distributed testing

---

## Communication Protocol Requirements

### Serial Protocol
- **Interface**: UART (3-wire: TX, RX, GND)
- **Baud Rate**: 115200 bps
- **Data Bits**: 8
- **Stop Bits**: 1
- **Parity**: None
- **Flow Control**: None (handled by application layer)

### Packet Format
- **Frame Delimiter**: 0xAA 0x55 (synchronization bytes)
- **Header**: 4 bytes (includes length and type)
- **Payload**: 0-256 bytes (variable)
- **CRC-16**: 2 bytes (validation)
- **Total**: 8-262 bytes per packet

### Message Types
- Telemetry packets (firmware → ground)
- Command packets (ground → firmware)
- Acknowledgment packets
- Error/status packets

---

## Data Storage Requirements

### Firmware Data
- Telemetry buffering: 4KB minimum
- Command queue: 16 entries
- State history: Last 10 state transitions
- Fault log: Last 100 fault events (with timestamps)

### Ground Station Data
- Telemetry logs: Persistent storage in CSV/binary format
- Session history: Metadata for each ground station session
- Configuration: User preferences and calibration data
- Temporary buffers: Adequate for ≥1 hour of continuous telemetry

### Log Files
- Format: CSV (human-readable) or binary (efficient storage)
- Retention: Configurable (default 30 days)
- Compression: Optional gzip compression for archived logs
- Backup: Local and optional cloud backup

---

## Performance Requirements

### Latency
- Command transmission to execution: <100ms
- Telemetry reception to display: <500ms
- Protocol parsing: <10ms per packet

### Throughput
- Telemetry rate: ≥10 Hz (100ms intervals)
- Peak serial bandwidth: ≤115200 bps
- Ground station packet throughput: ≥100 packets/second

### Reliability
- Packet loss tolerability: ≤1% (with retransmission)
- Protocol error recovery: Automatic resynchronization
- Firmware uptime: >99.5% excluding intentional shutdowns

### Memory Usage
- Firmware: <200KB total (code + data)
- Ground Station: <150MB (runtime + logs)
- Simulator: <100MB (runtime)

---

## Security & Safety Requirements

### Data Integrity
- CRC validation on all packets
- Command validation and checksums
- Telemetry integrity checks

### System Safety
- Watchdog timer with automatic recovery
- Safe state on power loss
- Graceful shutdown mechanisms
- Emergency disarm capability

### Access Control
- Optional command authentication (reserved for future)
- Serial port access restrictions (OS-level)
- Log file encryption (optional for sensitive data)

---

## Testing Requirements

### Unit Testing
- Firmware: 80% code coverage minimum
- Ground Station: 75% code coverage minimum
- Protocol: 100% coverage for critical paths

### Integration Testing
- Firmware + Ground Station end-to-end tests
- Simulator + Ground Station compatibility
- Cross-platform ground station testing

### System Testing
- 8+ hour continuous operation tests
- Fault injection and recovery tests
- High-packet-loss scenarios (>10%)
- Memory leak detection

### Documentation Tests
- All examples executable and verified
- API documentation complete and accurate
- Architecture diagrams maintained current

---

## Documentation Requirements

### Required Documentation
- System architecture and design decisions
- API documentation for all modules
- Protocol specification with examples
- Getting started guide and tutorials
- Troubleshooting guide
- Development workflow documentation
- Test plan and results
- Change log and release notes

### Code Documentation
- Inline comments for complex logic
- Function/module documentation strings
- Type documentation for custom types
- Example code in documentation
