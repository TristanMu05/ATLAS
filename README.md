# ATLAS - Avionic Telemetry Logging and Analysis System

A comprehensive avionics data acquisition and analysis system consisting of embedded firmware, ground station software, and comprehensive documentation.

## Features

- **Real-time Telemetry**: Stream avionic sensor data from embedded systems
- **Protocol Compliance**: Custom binary protocol with CRC validation
- **State Machine Control**: Robust finite state machine for system operations
- **Ground Station**: Feature-rich Rust-based dashboard and command interface
- **Simulation Support**: Built-in simulator for testing and development
- **Comprehensive Logging**: Full telemetry data recording and replay capabilities
- **Command Interface**: Remote command dispatcher with protocol validation

## Project Structure

```
ATLAS/
├── README.md                 # This file
├── LICENSE                   # Project license
├── .gitignore               # Git ignore rules
├── .editorconfig            # Editor configuration
│
├── docs/                    # Project documentation
│   ├── proposal.md          # Project proposal
│   ├── requirements.md      # System requirements
│   ├── use_scenarios.md     # Usage scenarios
│   ├── modes.md             # Operating modes
│   ├── protocol.md          # Telemetry protocol specification
│   ├── commands.md          # Command set documentation
│   ├── architecture_overview.md
│   ├── firmware_design.md
│   ├── state_machine.md
│   ├── test_plan.md
│   ├── demo_script.md
│   └── weekly_journal/      # Development journal
│
├── firmware/                # Embedded firmware (STM32)
│   ├── README.md
│   ├── stm32/              # STM32CubeMX project
│   │   ├── Core/
│   │   ├── Drivers/
│   │   ├── Middlewares/
│   │   ├── Startup/
│   │   └── atlas.ioc
│   ├── src/                # Firmware source code
│   │   ├── main.c
│   │   ├── telemetry.c/h
│   │   ├── commands.c/h
│   │   ├── protocol.c/h
│   │   ├── state_machine.c/h
│   │   └── faults.c/h
│   └── include/
│
├── ground/                 # Ground station (Rust)
│   ├── README.md
│   ├── Cargo.toml
│   ├── Cargo.lock
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── serial/         # Serial communication
│       ├── protocol/       # Protocol parsing
│       ├── telemetry/      # Telemetry handling
│       ├── commands/       # Command dispatcher
│       ├── logger/         # Data logging and replay
│       └── ui/             # Dashboard interface
│
├── simulator/              # Telemetry simulator (Rust)
│   ├── README.md
│   ├── Cargo.toml
│   └── src/
│
├── tools/                  # Utility tools
│   ├── crc_check/
│   ├── packet_inspector/
│   └── log_converter/
│
└── tests/                  # Integration tests
```

## Prerequisites

### Firmware Development
- ARM GCC toolchain
- STM32CubeMX
- ST-Link V2 programmer

### Ground Station & Simulator
- Rust 1.70+ ([rustup](https://rustup.rs/))
- Cargo
- Serial port access

### Documentation
- Markdown viewer or IDE with Markdown support

## Getting Started

### Building the Firmware

```bash
cd firmware
# Use STM32CubeMX to generate project, then build with ARM GCC
```

### Running the Ground Station

```bash
cd ground
cargo build --release
cargo run --release
```

### Running the Simulator

```bash
cd simulator
cargo run --release
```

## Documentation

See the [docs/](docs/) directory for detailed documentation:

- [Project Proposal](docs/proposal.md)
- [System Requirements](docs/requirements.md)
- [Protocol Specification](docs/protocol.md)
- [Architecture Overview](docs/architecture_overview.md)
- [Test Plan](docs/test_plan.md)

## Development

### Running Tests

```bash
# Firmware tests
cd firmware && make test

# Ground station tests
cd ground && cargo test

# Protocol tests
cargo test --test protocol_tests
```

### Development Workflow

1. See [docs/weekly_journal/](docs/weekly_journal/) for ongoing development notes
2. Follow the architecture outlined in [docs/architecture_overview.md](docs/architecture_overview.md)
3. Reference [docs/state_machine.md](docs/state_machine.md) for system behavior

## Tools

Utilities for protocol analysis and data conversion:

- **CRC Check**: Validate protocol checksums
- **Packet Inspector**: Analyze telemetry packets
- **Log Converter**: Convert logged data formats

## License

See [LICENSE](LICENSE) file for details.

## Support

For issues, questions, or contributions, please refer to the project documentation or contact the development team.

│   ├── fault_injection.rs
│   └── replay_tests.rs
│
└── scripts/
    ├── build_firmware.sh
    ├── flash_stm32.sh
    ├── run_ground.sh
    └── format_all.sh

