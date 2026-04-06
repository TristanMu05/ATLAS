/*
 * packet.h
 *
 *  Created on: Apr 4, 2026
 *      Author: murad
 */

#ifndef PACKET_H_
#define PACKET_H_

#include <stdbool.h>
#include <stdint.h>

enum {
    ATLAS_PACKET_SYNC_BYTE_0 = 0xAAu,
    ATLAS_PACKET_SYNC_BYTE_1 = 0x55u,
    ATLAS_PACKET_SYNC_LEN = 2u,
    ATLAS_PACKET_HEADER_LEN = 9u,
    ATLAS_PACKET_PAYLOAD_OFFSET = 11u,
    ATLAS_PACKET_CRC_LEN = 2u,
    ATLAS_PACKET_MIN_FRAME_LEN = 13u,
    ATLAS_MAX_PAYLOAD_LEN = 64u,
    ATLAS_MAX_COMMAND_ARGS = 63u,
    ATLAS_MAX_FRAME_LEN = ATLAS_PACKET_MIN_FRAME_LEN + ATLAS_MAX_PAYLOAD_LEN,
    ATLAS_TELEMETRY_PAYLOAD_LEN = 10u
};

typedef enum {
    ATLAS_MODE_IDLE = 0x00u,
    ATLAS_MODE_NORMAL = 0x01u,
    ATLAS_MODE_SAFE = 0x02u,
    ATLAS_MODE_DIAGNOSTIC = 0x03u
} atlas_mode_t;

typedef enum {
    ATLAS_MESSAGE_TELEMETRY = 0x01u,
    ATLAS_MESSAGE_COMMAND = 0x02u,
    ATLAS_MESSAGE_ACK = 0x03u,
    ATLAS_MESSAGE_COMMAND_RESPONSE = 0x04u,
    ATLAS_MESSAGE_EVENT = 0x05u
} atlas_message_id_t;

typedef enum {
    ATLAS_COMMAND_SET_MODE = 0x01u,
    ATLAS_COMMAND_REQUEST_STATUS = 0x02u,
    ATLAS_COMMAND_CLEAR_FAULTS = 0x03u,
    ATLAS_COMMAND_SET_TELEMETRY_ENABLE = 0x04u,
    ATLAS_COMMAND_SMALL_FAULT = 0x05u,
    ATLAS_COMMAND_MAJOR_FAULT = 0x06u
} atlas_command_id_t;

typedef enum {
    ATLAS_ACK_CODE_ACK = 0x06u,
    ATLAS_ACK_CODE_NAK = 0x15u
} atlas_ack_code_t;

typedef enum {
    ATLAS_COMMAND_RESPONSE_COMPLETED = 0x00u,
    ATLAS_COMMAND_RESPONSE_REJECTED = 0x01u,
    ATLAS_COMMAND_RESPONSE_INVALID_PARAMETER = 0x02u,
    ATLAS_COMMAND_RESPONSE_INVALID_MODE = 0x03u,
    ATLAS_COMMAND_RESPONSE_FAULT_ACTIVE = 0x04u,
    ATLAS_COMMAND_RESPONSE_EXECUTION_ERROR = 0x05u,
    ATLAS_COMMAND_RESPONSE_NOT_SUPPORTED = 0x06u,
    ATLAS_COMMAND_RESPONSE_DEFERRED = 0x07u
} atlas_command_response_t;

typedef enum {
    ATLAS_EVENT_MALFORMED_PACKET = 0x01u,
    ATLAS_EVENT_CRC_FAILURE = 0x02u,
    ATLAS_EVENT_UNSUPPORTED_COMMAND = 0x03u,
    ATLAS_EVENT_COMMAND_INVALID_IN_MODE = 0x04u,
    ATLAS_EVENT_TEMP_SENSOR_READ_FAIL = 0x05u,
    ATLAS_EVENT_LIGHT_SENSOR_READ_FAIL = 0x06u,
    ATLAS_EVENT_VOLTAGE_MONITOR_READ_FAIL = 0x07u,
    ATLAS_EVENT_SAFE_MODE_ENTERED = 0x08u,
    ATLAS_EVENT_SAFE_MODE_EXITED = 0x09u,
    ATLAS_EVENT_INTERNAL_TIMEOUT = 0x0Au,
    ATLAS_EVENT_UART_RX_OVERRUN = 0x0Bu,
    ATLAS_EVENT_WATCHDOG_RESET_DETECTED = 0x0Cu,
    ATLAS_EVENT_UNDER_TEMPERATURE = 0x0Du,
    ATLAS_EVENT_HIGH_VOLTAGE = 0x0Eu,
    ATLAS_EVENT_LOW_VOLTAGE = 0x0Fu,
    ATLAS_EVENT_OVER_TEMPERATURE = 0x10u
} atlas_event_code_t;

typedef enum {
    ATLAS_STATUS_TEMP_VALID = 1u << 0,
    ATLAS_STATUS_LIGHT_VALID = 1u << 1,
    ATLAS_STATUS_VOLTAGE_VALID = 1u << 2,
    ATLAS_STATUS_SENSORS_INITIALIZED = 1u << 3,
    ATLAS_STATUS_TELEMETRY_ENABLED = 1u << 4,
    ATLAS_STATUS_DEGRADED_OPERATION = 1u << 5
} atlas_status_flag_t;

typedef enum {
    ATLAS_FAULT_TEMP_SENSOR = 1u << 0,
    ATLAS_FAULT_LIGHT_SENSOR = 1u << 1,
    ATLAS_FAULT_VOLTAGE_MONITOR = 1u << 2,
    ATLAS_FAULT_OVER_TEMPERATURE = 1u << 3,
    ATLAS_FAULT_LOW_VOLTAGE = 1u << 4,
    ATLAS_FAULT_SENSOR_INIT = 1u << 5,
    ATLAS_FAULT_UART_RX_OVERRUN = 1u << 6,
    ATLAS_FAULT_RX_BUFFER_OVERFLOW = 1u << 7,
    ATLAS_FAULT_INTERNAL_TIMEOUT = 1u << 8,
    ATLAS_FAULT_WATCHDOG_RESET_DETECTED = 1u << 9,
    ATLAS_FAULT_PLATFORM_INIT = 1u << 10,
    ATLAS_FAULT_UNDER_TEMPERATURE = 1u << 11,
    ATLAS_FAULT_HIGH_VOLTAGE = 1u << 12
} atlas_fault_flag_t;

typedef enum {
    ATLAS_PACKET_STATUS_OK = 0,
    ATLAS_PACKET_STATUS_NULL_POINTER,
    ATLAS_PACKET_STATUS_INVALID_MESSAGE_ID,
    ATLAS_PACKET_STATUS_PAYLOAD_TOO_LARGE,
    ATLAS_PACKET_STATUS_BUFFER_TOO_SMALL,
    ATLAS_PACKET_STATUS_INVALID_SYNC,
    ATLAS_PACKET_STATUS_LENGTH_MISMATCH,
    ATLAS_PACKET_STATUS_CRC_MISMATCH,
    ATLAS_PACKET_STATUS_INVALID_PAYLOAD,
    ATLAS_PACKET_STATUS_INVALID_PARAMETER,
    ATLAS_PACKET_STATUS_UNSUPPORTED_COMMAND
} atlas_packet_status_t;

typedef struct {
    uint8_t message_id;
    uint16_t payload_len;
    uint16_t sequence;
    uint32_t timestamp_ms;
    uint8_t payload[ATLAS_MAX_PAYLOAD_LEN];
} atlas_packet_t;

typedef struct {
    atlas_mode_t mode;
    int16_t temperature_deci_c;
    uint16_t voltage_mv;
    uint16_t light_raw;
    uint8_t status_flags;
    uint16_t fault_flags;
} atlas_telemetry_t;

typedef struct {
    atlas_command_id_t command_id;
    uint8_t args_len;
    uint8_t args[ATLAS_MAX_COMMAND_ARGS];
} atlas_command_t;

uint16_t atlas_crc16_ccitt_false(const uint8_t *data, uint16_t length);
uint16_t atlas_packet_frame_len(uint16_t payload_len);
bool atlas_mode_is_valid(uint8_t mode);
const char *atlas_mode_name(atlas_mode_t mode);

atlas_packet_status_t atlas_encode_packet(
    const atlas_packet_t *packet,
    uint8_t *frame,
    uint16_t frame_capacity,
    uint16_t *frame_len
);

atlas_packet_status_t atlas_decode_packet(
    const uint8_t *frame,
    uint16_t frame_len,
    atlas_packet_t *packet
);

atlas_packet_status_t atlas_build_telemetry_packet(
    const atlas_telemetry_t *telemetry,
    uint16_t sequence,
    uint32_t timestamp_ms,
    atlas_packet_t *packet
);

atlas_packet_status_t atlas_build_ack_packet(
    atlas_ack_code_t ack_code,
    uint16_t sequence,
    uint32_t timestamp_ms,
    atlas_packet_t *packet
);

atlas_packet_status_t atlas_build_command_response_packet(
    atlas_command_response_t response_code,
    uint16_t sequence,
    uint32_t timestamp_ms,
    atlas_packet_t *packet
);

atlas_packet_status_t atlas_build_event_packet(
    atlas_event_code_t event_code,
    uint16_t sequence,
    uint32_t timestamp_ms,
    atlas_packet_t *packet
);

atlas_packet_status_t atlas_parse_command_packet(
    const atlas_packet_t *packet,
    atlas_command_t *command
);

#endif /* PACKET_H_ */
