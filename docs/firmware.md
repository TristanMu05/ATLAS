# Firmware Design

## Purpose

This document defines what the ATLAS firmware is responsible for, how it should be structured, and which design decisions still need to be made before implementation.

The firmware runs on an STM32 microcontroller and acts as the embedded side of the ATLAS system. Its job is to collect telemetry, maintain a safe operating state, and exchange packets with the ground system over UART.

## Firmware Responsibilities

The firmware is responsible for:

- Initializing the MCU, clocks, UART, timers, watchdog, and sensor interfaces.
- Reading onboard sensor data such as temperature, light level, and power-related values.
- Converting sampled data into protocol-compliant telemetry packets.
- Receiving commands from the ground system over UART.
- Validating commands before they affect system behavior.
- Maintaining the current operating mode and applying mode transitions.
- Detecting faults and moving the system into a safe state when required.
- Reporting status, acknowledgments, and fault events back to the ground system.

The firmware is not responsible for:

- Rendering telemetry dashboards or user interfaces.
- Long-term log storage and replay tooling.
- Host-side packet parsing beyond constructing and transmitting valid packets.

## Primary Design Decision

### Superloop Instead of RTOS

The current design uses a superloop architecture rather than an RTOS.

This is a good fit if the firmware remains small, deterministic, and easy to reason about. The expected workload is straightforward:

- receive commands
- sample a limited number of sensors
- evaluate state and faults
- build a packet
- transmit a response or telemetry frame

An RTOS would only be justified if the firmware grows into a system with competing real-time tasks, complex driver concurrency, or hard latency guarantees that become difficult to satisfy in a single loop.

### Why the Superloop Fits This Project

- Execution order is explicit and easy to debug.
- Timing behavior is easier to analyze for a small system.
- Memory use stays lower than an RTOS-based design.
- State transitions and fault handling remain centralized.

### Tradeoffs

- Long-running operations can stall the loop if they are not designed carefully.
- UART and sensor interactions must be kept non-blocking or tightly bounded.
- Interrupt handlers must remain minimal so the main loop keeps ownership of system behavior.

## Firmware Execution Model

The firmware should follow a predictable loop with bounded work in each pass.

### Boot Sequence

On startup, the firmware should:

1. Configure clocks and core peripherals.
2. Initialize UART, timers, watchdog, and sensor drivers.
3. Initialize state, counters, buffers, and fault flags.
4. Perform basic self-checks.
5. Enter `IDLE` mode unless a fault requires `SAFE` mode.

### Main Superloop

Each loop iteration should follow this order:

1. Service watchdog.
2. Pull in any newly received UART bytes.
3. Parse complete packets from the receive buffer.
4. Validate and apply accepted commands.
5. Sample sensors whose acquisition time fits the loop budget.
6. Update derived values, status flags, and fault conditions.
7. Evaluate whether a mode transition is required.
8. Build outbound packets such as telemetry, ACK, or fault/event packets.
9. Transmit any queued outbound packets.

This order matters. Command handling should happen before the next telemetry transmission so the system reports behavior after the most recent accepted input.

## Operating Modes

The firmware should use the system modes defined in [modes.md](modes.md).

| Mode | Purpose | Firmware Behavior |
| --- | --- | --- |
| `IDLE` | System is powered and waiting | Maintain communications, sample only essential housekeeping data, accept mode-change commands |
| `NORMAL` | Standard telemetry operation | Sample all required sensors, send telemetry on schedule, process valid commands |
| `SAFE` | Protect the system during faults | Reduce activity, keep critical telemetry alive, reject unsafe commands, report fault state |
| `DIAGNOSTIC` | Maintenance and self-test behavior | Run self-tests, expose diagnostic status, allow controlled verification commands |

## State and Mode Transition Rules

The firmware needs a small, explicit state machine. At minimum:

- `IDLE -> NORMAL` when commanded and system health is acceptable.
- `NORMAL -> SAFE` when critical thresholds are violated or a serious internal error occurs.
- `SAFE -> IDLE` only after the fault condition clears and recovery is allowed.
- `IDLE -> DIAGNOSTIC` when diagnostic mode is explicitly requested.
- `DIAGNOSTIC -> IDLE` when diagnostic operations complete or timeout.

Mode transitions should never be implicit or hidden inside low-level drivers. They should be owned by one state-management module so behavior stays consistent.

## UART Communication Responsibilities

The firmware communicates with the ground system over UART using the packet framing defined in [protocol.md](protocol.md).

At minimum, the firmware must:

- maintain a receive buffer for incoming bytes
- detect the sync word
- validate payload length before allocation or parsing
- verify CRC before accepting a packet
- reject malformed or unsupported packets safely
- maintain a sequence counter for outgoing packets
- include a timestamp or loop-relative time value in transmitted packets

The firmware should support at least these outbound packet classes:

- telemetry packets
- acknowledgment packets
- command response packets
- fault or event packets

## Telemetry Responsibilities

The firmware should produce telemetry that is small, consistent, and useful for operations. The minimum telemetry set should include:

- mode or state
- uptime or timestamp
- sequence counter
- temperature reading
- voltage or power reading
- light sensor reading
- status flags
- fault flags

Optional telemetry can be added later, but the first version should prioritize fields that directly support health monitoring and safe operation.

## Command Handling Responsibilities

The firmware should not accept arbitrary commands. It needs a constrained command set with clear validation rules.

The initial command categories should be:

- change operating mode
- request immediate status or diagnostic data
- clear recoverable faults
- reset counters or session state
- start or stop a telemetry behavior if that concept exists

Every accepted command should produce one of the following outcomes:

- explicit acknowledgment
- explicit rejection with reason
- deferred response if execution is asynchronous

## Fault Handling Strategy

Fault handling must be designed early because it shapes the rest of the firmware structure.

The firmware should detect and respond to at least these fault classes:

- malformed incoming packet
- CRC failure
- unsupported command
- sensor read failure
- out-of-range sensor value
- low voltage
- over-temperature
- internal timeout or stuck-loop condition

Recommended response policy:

| Fault Type | Typical Response |
| --- | --- |
| Bad packet or CRC failure | Drop packet, increment error counter, optionally report comms error |
| Unsupported command | Reject command and return error response |
| Recoverable sensor read failure | Keep prior value if safe, mark degraded status, report event |
| Critical power or thermal fault | Transition to `SAFE` mode and report fault immediately |
| Repeated internal timing failure | Trigger watchdog or reset path depending on severity |

## Interrupt Strategy

The firmware will still need interrupts even with a superloop architecture. The key rule is that interrupts should be short and should not contain business logic.

Interrupts are appropriate for:

- UART receive buffering
- timer tick generation
- watchdog support
- hardware fault conditions that require immediate capture

Interrupts should not:

- perform packet parsing
- make mode transition decisions
- run large sensor processing routines
- assemble outbound packets

Those higher-level decisions belong in the main loop.

## Buffering and Timing

The firmware needs explicit timing and buffering limits instead of vague real-time goals.

Minimum expectations:

- Telemetry cadence should be fixed and documented.
- Loop execution time should stay below the telemetry period with margin.
- UART receive buffering must tolerate short bursts without dropping frames.
- Sensor reads should be bounded so one slow device does not block the entire loop.

The exact numbers are still undecided, but they must be defined before implementation is considered complete.

## Suggested Firmware Module Breakdown

The firmware should eventually be separated into clear modules similar to the following:

- `main`: startup and top-level loop
- `platform`: clocks, HAL setup, MCU-specific initialization
- `uart`: byte transport and RX/TX buffering
- `protocol`: packet encode, decode, CRC, framing
- `sensors`: sensor drivers and sample aggregation
- `state_machine`: mode ownership and transition rules
- `commands`: command validation and dispatch
- `faults`: fault detection, latching, and reporting
- `telemetry`: telemetry field assembly and packet scheduling

This breakdown keeps transport, protocol logic, and behavior decisions separated.

## Minimum Viable Firmware

The first firmware milestone does not need every feature in the full requirements document.

A realistic minimum viable firmware should:

- boot reliably on the chosen STM32 target
- sample the initial sensor set
- send telemetry packets on a fixed interval
- receive and validate at least a small set of commands
- maintain `IDLE`, `NORMAL`, and `SAFE` modes
- detect packet corruption and basic sensor failures
- expose enough status for the ground system to confirm correct behavior

Diagnostic extras, advanced authentication, and complex recovery workflows can come later.

## What Still Needs To Be Defined

This is the most important section for implementation planning. The firmware design is not complete until these decisions are made.

### Hardware Decisions

- Exact STM32 part number
- Exact sensors and electrical interfaces
- ADC versus digital sensor choices
- Power-monitoring hardware and measurable thresholds
- Whether non-volatile storage is available or required

### Protocol Decisions

- Final CRC algorithm and byte order
- Final payload layouts for each packet type
- Maximum packet length supported by firmware buffers
- Whether command IDs and error codes need their own spec section

### Behavioral Decisions

- Exact telemetry period
- Which commands are required in v1
- Safe-mode entry and exit conditions
- Which faults are latched versus auto-cleared
- Whether unsupported commands are ignored or explicitly rejected
- What the firmware should do after repeated communication loss

### Timing and Reliability Decisions

- Main loop budget
- Sensor sampling rates per device
- UART timeout policy
- Watchdog timeout value
- Retry behavior for transient sensor failures

### Verification Decisions

- How sensor drivers will be tested without hardware
- How packet encoding will be validated against the Rust protocol crate
- What bench-level tests are required before flight-like or long-duration runs

## Recommended Next Steps

The next firmware planning work should be:

1. Define the exact STM32 target and initial sensor list.
2. Freeze the telemetry packet payload for v1.
3. Define the initial command set and response codes.
4. Write the mode transition rules as a small state table.
5. Define the critical thresholds that force `SAFE` mode.
6. Specify the watchdog, timing budget, and telemetry rate.

Once those are written down, implementation can proceed without guessing.
