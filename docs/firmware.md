# Firmware Design

## Purpose

This document defines what the ATLAS firmware is responsible for, how it is structured, and which implementation decisions are already frozen for v1.

The firmware runs on an STM32F446RET6 and acts as the embedded side of the ATLAS system. Its job is to collect telemetry, maintain a safe operating state, and exchange packets with the ground system over UART.

## Firmware Responsibilities

The firmware is responsible for:

- Initializing the MCU, clocks, UART, timers, watchdog, ADC, and sensor interfaces.
- Reading the temperature sensor, light sensor, and voltage monitor input.
- Converting sampled data into protocol-compliant telemetry packets.
- Receiving commands from the ground system over UART.
- Validating commands before they affect system behavior.
- Maintaining the current operating mode and applying mode transitions.
- Detecting faults and moving the system into a safe state when required.
- Reporting telemetry, acknowledgments, command responses, and fault/event packets back to the ground system.

The firmware is not responsible for:

- Rendering telemetry dashboards or user interfaces.
- Long-term log storage and replay tooling.
- Host-side packet parsing beyond constructing and transmitting valid packets.

## Current v1 Hardware Decisions

- MCU target: `STM32F446RET6`
- Architecture: superloop, no RTOS
- Communications: UART at 115200 baud, 8N1, no flow control
- Sensor set:
  - external temperature sensor
  - external light sensor
  - ADC-based voltage monitor input
- Voltage source policy:
  - the `voltage` telemetry field represents millivolts
  - until a dedicated external battery monitor is finalized, the firmware may use an onboard analog reading or a simulated scaled value as the source for this field

## Primary Design Decision

### Superloop Instead of RTOS

The firmware uses a superloop architecture rather than an RTOS.

This is a good fit for the current workload:

- receive commands
- sample a limited number of sensors
- evaluate state and faults
- build packets
- transmit responses or telemetry

An RTOS would only be justified if the firmware grows into a system with competing real-time tasks, complex concurrency, or harder latency guarantees than a single loop can comfortably satisfy.

### Why the Superloop Fits This Project

- Execution order is explicit and easy to debug.
- Timing behavior is easier to analyze for a small system.
- Memory use stays lower than an RTOS-based design.
- State transitions and fault handling remain centralized.

### Tradeoffs

- Long-running operations can stall the loop if they are not bounded carefully.
- UART and sensor interactions must be non-blocking or tightly time-bounded.
- Interrupt handlers must stay minimal so the main loop remains in control of behavior.

## Firmware Execution Model

### Boot Sequence

On startup, the firmware should:

1. Configure clocks and core peripherals.
2. Initialize UART, timers, watchdog, ADC, and sensor drivers.
3. Initialize state, counters, buffers, and fault flags.
4. Perform basic self-checks.
5. Enter `IDLE` unless a fault requires `SAFE`.

### Main Superloop

Each loop iteration should follow this order:

1. Service watchdog.
2. Pull in any newly received UART bytes.
3. Parse complete packets from the receive buffer.
4. Validate and apply accepted commands.
5. Sample sensors whose acquisition time fits the loop budget.
6. Update derived values, status flags, and fault conditions.
7. Evaluate whether a mode transition is required.
8. Build outbound packets such as telemetry, ACK, command response, or fault/event packets.
9. Transmit any queued outbound packets.

Command handling happens before the next telemetry transmission so the system reports behavior after the most recent accepted input.

## Operating Modes

The firmware uses the system modes defined in [modes.md](modes.md).

| Mode | Purpose | Firmware Behavior |
| --- | --- | --- |
| `IDLE` | System is powered and waiting | Maintain communications, send reduced-rate telemetry, accept commands |
| `NORMAL` | Standard telemetry operation | Sample all required sensors, send telemetry on schedule, process valid commands |
| `SAFE` | Protect the system during faults | Reduce activity, keep critical telemetry alive, reject unsafe commands, report fault state |
| `DIAGNOSTIC` | Maintenance and self-test behavior | Run self-tests and controlled checks, then return to `IDLE` |

## Mode Transition Rules

The firmware uses an explicit state machine.

| From | To | Condition |
| --- | --- | --- |
| `IDLE` | `NORMAL` | `SET_MODE(NORMAL)` accepted and no blocking critical faults are active |
| `IDLE` | `DIAGNOSTIC` | `SET_MODE(DIAGNOSTIC)` accepted |
| `NORMAL` | `IDLE` | `SET_MODE(IDLE)` accepted |
| `NORMAL` | `SAFE` | Critical fault becomes active or `SET_MODE(SAFE)` is accepted |
| `DIAGNOSTIC` | `IDLE` | Diagnostic operations complete, timeout occurs, or `SET_MODE(IDLE)` is accepted |
| `SAFE` | `IDLE` | Critical threshold faults are no longer active and `SET_MODE(IDLE)` is accepted |
| `SAFE` | `NORMAL` | Not allowed directly in v1 |

Mode transitions are owned by one state-management module and must not be hidden inside low-level drivers.

## UART and Protocol Responsibilities

The firmware communicates with the ground system over UART using the packet framing defined in [protocol.md](protocol.md).

At minimum, the firmware must:

- maintain a receive buffer for incoming bytes
- detect the sync word
- validate payload length before parsing
- verify CRC before accepting a packet
- reject malformed or unsupported packets safely
- maintain a sequence counter for outgoing packets
- include a boot-relative timestamp in transmitted packets

Outbound packet classes for v1:

- telemetry packets
- ACK / NAK packets
- command response packets
- fault / event packets

Inbound packet class for v1:

- command packets

## Telemetry Responsibilities

The telemetry payload is frozen in v1 and defined in [protocol.md](protocol.md):

- mode
- temperature
- voltage
- light
- status flags
- fault flags

Telemetry units for v1:

- temperature: signed deci-degrees Celsius
- voltage: unsigned millivolts
- light: raw ADC counts

## Command Handling Responsibilities

The firmware does not accept arbitrary commands.

The initial v1 command set is:

- `SET_MODE`
- `REQUEST_STATUS`
- `CLEAR_FAULTS`
- `SET_TELEMETRY_ENABLE`

Command handling rules for v1:

- only one command may be outstanding at a time
- unsupported command IDs are explicitly rejected
- malformed command payloads are explicitly rejected
- ACK / NAK indicates packet-level acceptance for execution
- Command Response indicates execution outcome

## Fault Handling Strategy

The firmware detects and responds to at least these fault classes:

- malformed incoming packet
- CRC failure
- unsupported command
- sensor read failure
- out-of-range sensor value
- low voltage
- over-temperature
- UART receive overrun or receive buffer overflow
- internal timeout or stuck-loop condition

Recommended response policy:

| Fault Type | Typical Response |
| --- | --- |
| Bad packet or CRC failure | Drop packet, increment error counters, optionally emit fault/event packet |
| Unsupported command | Reject command and return NAK or command response as appropriate |
| Recoverable sensor read failure | Keep prior value if safe, set degraded status, latch fault bit, emit event if needed |
| Critical power or thermal fault | Transition to `SAFE` and report the fault immediately |
| Repeated internal timing failure | Latch internal timeout fault and rely on watchdog recovery if needed |

## Interrupt Strategy

Interrupts are appropriate for:

- UART receive buffering
- timer tick generation
- watchdog support
- hardware fault conditions that require immediate capture

Interrupts must not:

- parse packets
- make mode transition decisions
- run large sensor-processing routines
- assemble outbound packets

Those higher-level decisions belong in the main loop.

## Timing and Buffering Decisions

The following limits are frozen for v1:

- telemetry cadence in `NORMAL`: 100 ms
- telemetry cadence in `IDLE`: 500 ms
- telemetry cadence in `SAFE`: 200 ms
- telemetry cadence in `DIAGNOSTIC`: 100 ms while diagnostics are active
- loop budget target under nominal conditions: under 10 ms per pass
- firmware-supported maximum payload length: 64 bytes
- UART receive buffer size: 256 bytes minimum
- watchdog timeout target: 1000 ms

Sensor sampling policy for v1:

- temperature sampled once per telemetry period
- light sampled once per telemetry period
- voltage sampled once per telemetry period

## Suggested Firmware Module Breakdown

The firmware should be separated into these modules:

- `main`: startup and top-level loop
- `platform`: clocks, HAL setup, MCU-specific initialization
- `uart`: byte transport and RX/TX buffering
- `protocol`: packet encode, decode, CRC, framing
- `sensors`: sensor drivers and sample aggregation
- `state_machine`: mode ownership and transition rules
- `commands`: command validation and dispatch
- `faults`: fault detection, latching, and reporting
- `telemetry`: telemetry field assembly and packet scheduling

This keeps transport, protocol logic, and behavior decisions separated.

## Minimum Viable Firmware

The first firmware milestone should:

- boot reliably on the STM32F446RET6
- initialize UART, timers, watchdog, ADC, and the initial sensors
- send valid telemetry packets on the defined interval
- receive and validate the v1 command set
- maintain `IDLE`, `NORMAL`, and `SAFE`
- detect packet corruption and basic sensor failures
- expose enough status for the ground system to confirm correct behavior

Diagnostic extras, advanced authentication, and complex recovery workflows can come later.

## Remaining Open Items

These items are intentionally deferred until bring-up and characterization:

- exact sensor part numbers and electrical interface details
- final ADC scaling constants and calibration values
- numeric SAFE thresholds for temperature and voltage
- whether the voltage field stays simulated during early bring-up or is replaced by a dedicated hardware monitor immediately
- additional fault / event codes beyond the initial set in [protocol.md](protocol.md)

These are not protocol blockers. Firmware implementation can proceed now.

## Recommended Next Implementation Steps

1. Generate the STM32F446RET6 CubeMX / CubeIDE project.
2. Bring up clocks, UART, timer tick, watchdog, ADC, and GPIO heartbeat.
3. Implement protocol encode and decode to match [protocol.md](protocol.md) and the Rust packet crate.
4. Build telemetry packet assembly with the frozen 10-byte payload.
5. Add command parsing for the four v1 commands.
6. Characterize sensor readings on hardware and then fill in numeric SAFE thresholds.
