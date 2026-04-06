# Protocol Specification

## Overview

ATLAS uses a framed binary packet protocol over UART for both firmware-to-ground and ground-to-firmware traffic.

All multi-byte fields use big-endian byte order.

## Transport

- Interface: UART
- Line format: 115200 baud, 8 data bits, no parity, 1 stop bit
- Flow control: none
- Sync word: `0xAA55`
- CRC: CRC-16/CCITT-FALSE, big-endian
- CRC coverage: message ID through end of payload
- Sync word is not included in the CRC calculation

## Frame Layout

| Field | Size | Description |
| --- | --- | --- |
| Sync | 2 bytes | `0xAA55` |
| Message ID | 1 byte | Packet type |
| Length | 2 bytes | Payload length in bytes |
| Sequence Counter | 2 bytes | Per-sender incrementing counter |
| Timestamp | 4 bytes | Milliseconds since sender boot |
| Payload | Variable | Packet-specific contents |
| CRC | 2 bytes | CRC-16/CCITT-FALSE |

## Message IDs

| Message ID | Direction | Meaning |
| --- | --- | --- |
| `0x01` | Firmware -> Ground | Telemetry |
| `0x02` | Ground -> Firmware | Command |
| `0x03` | Firmware -> Ground | ACK / NAK |
| `0x04` | Firmware -> Ground | Command Response |
| `0x05` | Firmware -> Ground | Fault / Event |

ACK and Command Response are separate on purpose:

- ACK / NAK answers "was this accepted as a command packet?"
- Command Response answers "what happened when firmware tried to execute it?"

## Telemetry Packet (`0x01`)

Telemetry uses a fixed 10-byte payload.

| Field | Type | Size | Units | Notes |
| --- | --- | --- | --- | --- |
| `mode` | `u8` | 1 byte | enum | Current firmware mode |
| `temperature` | `i16` | 2 bytes | 0.1 C | Signed temperature in deci-degrees Celsius |
| `voltage` | `u16` | 2 bytes | mV | Measured or simulated supply / battery monitor value |
| `light` | `u16` | 2 bytes | ADC counts | Raw ADC counts |
| `status_flags` | `u8` | 1 byte | bitfield | Current non-fault status bits |
| `fault_flags` | `u16` | 2 bytes | bitfield | Active or latched fault bits |

Total telemetry frame length is 23 bytes:

- 2 bytes sync
- 1 byte message ID
- 2 bytes length
- 2 bytes sequence
- 4 bytes timestamp
- 10 bytes payload
- 2 bytes CRC

### Mode Enumeration

| Value | Mode |
| --- | --- |
| `0x00` | `IDLE` |
| `0x01` | `NORMAL` |
| `0x02` | `SAFE` |
| `0x03` | `DIAGNOSTIC` |

### Status Flags (`u8`)

| Bit | Name | Meaning |
| --- | --- | --- |
| 0 | `TEMP_VALID` | Temperature sample in this packet is valid |
| 1 | `LIGHT_VALID` | Light sample in this packet is valid |
| 2 | `VOLTAGE_VALID` | Voltage sample in this packet is valid |
| 3 | `SENSORS_INITIALIZED` | Required sensor interfaces initialized successfully |
| 4 | `TELEMETRY_ENABLED` | Periodic telemetry transmission is enabled |
| 5 | `DEGRADED_OPERATION` | Firmware is using reduced capability but is still running |
| 6 | `RESERVED` | Reserved for future use |
| 7 | `RESERVED` | Reserved for future use |

### Fault Flags (`u16`)

Fault flags represent current active faults or latched recoverable faults.

| Bit | Name | Meaning |
| --- | --- | --- |
| 0 | `TEMP_SENSOR_FAULT` | Temperature sensor read or validation failure |
| 1 | `LIGHT_SENSOR_FAULT` | Light sensor read or validation failure |
| 2 | `VOLTAGE_MONITOR_FAULT` | Voltage monitor read or validation failure |
| 3 | `OVER_TEMPERATURE` | Temperature exceeded SAFE threshold |
| 4 | `LOW_VOLTAGE` | Voltage fell below SAFE threshold |
| 5 | `SENSOR_INIT_FAULT` | One or more required sensors failed initialization |
| 6 | `UART_RX_OVERRUN` | UART peripheral overrun occurred |
| 7 | `RX_BUFFER_OVERFLOW` | Firmware receive buffer overflowed |
| 8 | `INTERNAL_TIMEOUT` | Internal timing or stuck-loop timeout detected |
| 9 | `WATCHDOG_RESET_DETECTED` | Previous reset was attributed to watchdog recovery |
| 10 | `PLATFORM_INIT_FAULT` | Core platform initialization failed or degraded |
| 11 | `UNDER_TEMPERATURE` | Temperature dropped below the demo recovery threshold |
| 12 | `HIGH_VOLTAGE` | Voltage exceeded the demo recovery threshold |
| 13 | `RESERVED` | Reserved for future use |
| 14 | `RESERVED` | Reserved for future use |
| 15 | `RESERVED` | Reserved for future use |

Latching rules for v1:

- `OVER_TEMPERATURE` and `LOW_VOLTAGE` reflect current threshold state and clear only after the measured value returns to a safe range.
- Sensor, UART, buffer, timeout, watchdog, and platform fault bits are latched until `CLEAR_FAULTS` succeeds or the MCU reboots.

## Command Packet (`0x02`)

Command payloads begin with a 1-byte command ID followed by zero or more argument bytes.

V1 command handling supports one outstanding command at a time.

| Command ID | Name | Args | Meaning |
| --- | --- | --- | --- |
| `0x01` | `SET_MODE` | 1 byte mode enum | Request mode transition |
| `0x02` | `REQUEST_STATUS` | none | Request an immediate telemetry/status packet |
| `0x03` | `CLEAR_FAULTS` | none | Clear recoverable latched faults |
| `0x04` | `SET_TELEMETRY_ENABLE` | 1 byte: `0x00` disable, `0x01` enable | Enable or disable periodic telemetry |
| `0x05` | `SMALL_FAULT` | 1 byte code | Demo-only injection of malformed or sequence-fault frames |
| `0x06` | `MAJOR_FAULT` | 1 byte code | Demo-only injection of 5-second recovery faults |

Command rules for v1:

- Unsupported command IDs are explicitly rejected.
- Invalid payload length is explicitly rejected.
- `SET_MODE` to `SAFE` is always allowed.
- `SET_MODE` out of `SAFE` is allowed only when blocking critical faults are no longer active.
- `SET_TELEMETRY_ENABLE(0x00)` is rejected while in `SAFE`.

Demo extension values:

- `SMALL_FAULT(0x01)` injects a CRC error
- `SMALL_FAULT(0x02)` injects a sync error
- `SMALL_FAULT(0x03)` injects a length error
- `SMALL_FAULT(0x04)` injects a sequence gap
- `MAJOR_FAULT(0x01)` injects a 5-second over-temperature condition
- `MAJOR_FAULT(0x02)` injects a 5-second high-voltage condition
- `MAJOR_FAULT(0x03)` injects a 5-second light sensor failure
- `MAJOR_FAULT(0x04)` injects a 5-second under-temperature condition
- `MAJOR_FAULT(0x05)` injects a 5-second low-voltage condition

## ACK / NAK Packet (`0x03`)

ACK payload is 1 byte.

| Value | Name | Meaning |
| --- | --- | --- |
| `0x06` | `ACK` | Command packet was syntactically valid and accepted for execution |
| `0x15` | `NAK` | Command packet was rejected before execution |

V1 assumption:

- Because only one command may be outstanding at a time, ACK does not carry a command ID.

## Command Response Packet (`0x04`)

Command Response payload is 1 byte.

| Value | Name | Meaning |
| --- | --- | --- |
| `0x00` | `COMPLETED` | Command executed successfully |
| `0x01` | `REJECTED` | Command was rejected by policy or state |
| `0x02` | `INVALID_PARAMETER` | Command arguments were malformed or out of range |
| `0x03` | `INVALID_MODE` | Command is not allowed in the current mode |
| `0x04` | `FAULT_ACTIVE` | Active fault condition blocks the command |
| `0x05` | `EXECUTION_ERROR` | Command was accepted but failed during execution |
| `0x06` | `NOT_SUPPORTED` | Command ID is not implemented in firmware |
| `0x07` | `DEFERRED` | Reserved for future asynchronous command flows |

Response rules for v1:

- A successfully ACKed command must produce a Command Response.
- A NAKed command does not require a Command Response.

## Fault / Event Packet (`0x05`)

Fault / Event payload is 1 byte.

| Value | Name | Meaning |
| --- | --- | --- |
| `0x01` | `MALFORMED_PACKET` | Framing or packet shape invalid |
| `0x02` | `CRC_FAILURE` | CRC check failed |
| `0x03` | `UNSUPPORTED_COMMAND` | Command ID is not supported |
| `0x04` | `COMMAND_INVALID_IN_MODE` | Command was blocked by current mode |
| `0x05` | `TEMP_SENSOR_READ_FAIL` | Temperature sensor read failed |
| `0x06` | `LIGHT_SENSOR_READ_FAIL` | Light sensor read failed |
| `0x07` | `VOLTAGE_MONITOR_READ_FAIL` | Voltage monitor read failed |
| `0x08` | `SAFE_MODE_ENTERED` | Firmware transitioned into SAFE mode |
| `0x09` | `SAFE_MODE_EXITED` | Firmware left SAFE mode |
| `0x0A` | `INTERNAL_TIMEOUT` | Internal timeout or stuck-loop condition detected |
| `0x0B` | `UART_RX_OVERRUN` | UART overrun was detected |
| `0x0C` | `WATCHDOG_RESET_DETECTED` | Watchdog-based recovery was detected after boot |
| `0x0D` | `UNDER_TEMPERATURE` | Temperature dropped below the demo recovery threshold |
| `0x0E` | `HIGH_VOLTAGE` | Voltage exceeded the demo recovery threshold |
| `0x0F` | `LOW_VOLTAGE` | Voltage dropped below the recovery threshold |
| `0x10` | `OVER_TEMPERATURE` | Temperature exceeded the recovery threshold |

Future fault / event codes may be added, but they must be documented in this file before firmware uses them.
