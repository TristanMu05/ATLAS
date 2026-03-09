# Protocol specification

## Overview
* Sync
* message ID
* length
* sequence counter
* timestamp
* payload
* CRC

## Sync
The sync word is the first part of the message and identifies the start of a new packet. It is a unique value that does not appear anywhere else in a message so it will identify the start of a new message and the end of the previous one. This is important because we will be receiving a stream of bytes and we need to be able to identify where each message starts and ends.

The sync word will be 2 bytes long and will be set to 0xAA55 that marks the start of a packet in the UART byte stream. The ground system scans incoming bytes until this value is detecteed, at which point packet parsing begins. The sync word is not included in the CRC calculation.

## Message ID
The message ID will be a unique identifier for each message being sent. The message id will tell us the type of packet were getting with 4 distinct values:
* 0x01: Telemetry data
* 0x02: Command response
* 0x03: ACK
* 0x04: Fault/Event

The Message ID is a 1-byte field identifying the packet type. This allows extensibility for future packet definitions without modifying the framing structure. 

## Length
This will be a 2 byte value that will indicate the length of the payload. This does not include the header or CRC fields and will allow us to know how many bytes to read for the payload. The length field will be important for parsing the message correctly and ensuring that we read the correct amount of data for each message.

## Sequence Counter
The sequence counter is a 2 byte value that will be incremented for each message sent. The sequence will wrap at 65535 as its a U16 value. The ground system uses this to detect dropped or duplicated packets

## Timestamp
The timestamp isa 4-byte UINT that will represent milliseconds since system boot. This will allow us to identify when a message was officially send and allow replayability on our ground control simulation.

## Payload
The payload will consist of the actual data being sent. The length of the payload will be determined by the length field, and the content will be determined by the message ID. For example, if the message ID is 0x01 (Telemetry data), the payload will contain various telemetry readings from the drone.

Potential telemetry data could include:
* Temperature (i16) 2 bytes
* Volatage (u16) 2 bytes
* Status Flags (u8) 1 byte

## CRC
The CRC (Cyclic Redundancy Check) will be a 2 byte value that will be used to verify the integrity of the message. It will be calculated based on the message ID, length, sequence counter, timestamp, and payload. The CRC will allow us to detect if any errors occurred during transmission and ensure that the data we receive is accurate.

The current implemnetation and algorithm is undecided. 


## Packet Layout (Byte Order)
| Field            | Size     | Description          |
| ---------------- | -------- | -------------------- |
| Sync             | 2 bytes  | 0xAA55               |
| Message ID       | 1 byte   | Packet type          |
| Length           | 2 bytes  | Payload length       |
| Sequence Counter | 2 bytes  | Incrementing counter |
| Timestamp        | 4 bytes  | ms since boot        |
| Payload          | Variable | Based on Message ID  |
| CRC              | 2 bytes  | CRC-16               |
