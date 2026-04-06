/*
 * packet.c
 *
 *  Created on: Apr 4, 2026
 *      Author: murad
 */

#include "packet.h"

static void write_u16_be(uint8_t *buffer, uint16_t value)
{
    buffer[0] = (uint8_t)(value >> 8);
    buffer[1] = (uint8_t)value;
}

static void write_u32_be(uint8_t *buffer, uint32_t value)
{
    buffer[0] = (uint8_t)(value >> 24);
    buffer[1] = (uint8_t)(value >> 16);
    buffer[2] = (uint8_t)(value >> 8);
    buffer[3] = (uint8_t)value;
}

static uint16_t read_u16_be(const uint8_t *buffer)
{
    return (uint16_t)(((uint16_t)buffer[0] << 8) | buffer[1]);
}

static uint32_t read_u32_be(const uint8_t *buffer)
{
    return ((uint32_t)buffer[0] << 24)
        | ((uint32_t)buffer[1] << 16)
        | ((uint32_t)buffer[2] << 8)
        | buffer[3];
}

static bool atlas_message_id_is_valid(uint8_t message_id)
{
    switch (message_id) {
    case ATLAS_MESSAGE_TELEMETRY:
    case ATLAS_MESSAGE_COMMAND:
    case ATLAS_MESSAGE_ACK:
    case ATLAS_MESSAGE_COMMAND_RESPONSE:
    case ATLAS_MESSAGE_EVENT:
        return true;
    default:
        return false;
    }
}

static bool atlas_small_fault_code_is_valid(uint8_t code)
{
    return (code >= 1u) && (code <= 4u);
}

static bool atlas_major_fault_code_is_valid(uint8_t code)
{
    return (code >= 1u) && (code <= 5u);
}

uint16_t atlas_crc16_ccitt_false(const uint8_t *data, uint16_t length)
{
    uint16_t crc;
    uint16_t i;

    if (data == 0) {
        return 0u;
    }

    crc = 0xFFFFu;
    for (i = 0u; i < length; ++i) {
        uint8_t bit;

        crc ^= (uint16_t)data[i] << 8;
        for (bit = 0u; bit < 8u; ++bit) {
            if ((crc & 0x8000u) != 0u) {
                crc = (uint16_t)((crc << 1) ^ 0x1021u);
            } else {
                crc <<= 1;
            }
        }
    }

    return crc;
}

uint16_t atlas_packet_frame_len(uint16_t payload_len)
{
    return (uint16_t)(ATLAS_PACKET_MIN_FRAME_LEN + payload_len);
}

bool atlas_mode_is_valid(uint8_t mode)
{
    switch (mode) {
    case ATLAS_MODE_IDLE:
    case ATLAS_MODE_NORMAL:
    case ATLAS_MODE_SAFE:
    case ATLAS_MODE_DIAGNOSTIC:
        return true;
    default:
        return false;
    }
}

const char *atlas_mode_name(atlas_mode_t mode)
{
    switch (mode) {
    case ATLAS_MODE_IDLE:
        return "IDLE";
    case ATLAS_MODE_NORMAL:
        return "NORMAL";
    case ATLAS_MODE_SAFE:
        return "SAFE";
    case ATLAS_MODE_DIAGNOSTIC:
        return "DIAGNOSTIC";
    default:
        return "UNKNOWN";
    }
}

atlas_packet_status_t atlas_encode_packet(
    const atlas_packet_t *packet,
    uint8_t *frame,
    uint16_t frame_capacity,
    uint16_t *frame_len
)
{
    uint16_t total_len;
    uint16_t crc;
    uint16_t crc_index;
    uint16_t i;

    if ((packet == 0) || (frame == 0) || (frame_len == 0)) {
        return ATLAS_PACKET_STATUS_NULL_POINTER;
    }

    if (!atlas_message_id_is_valid(packet->message_id)) {
        return ATLAS_PACKET_STATUS_INVALID_MESSAGE_ID;
    }

    if (packet->payload_len > ATLAS_MAX_PAYLOAD_LEN) {
        return ATLAS_PACKET_STATUS_PAYLOAD_TOO_LARGE;
    }

    total_len = atlas_packet_frame_len(packet->payload_len);
    if (frame_capacity < total_len) {
        return ATLAS_PACKET_STATUS_BUFFER_TOO_SMALL;
    }

    frame[0] = ATLAS_PACKET_SYNC_BYTE_0;
    frame[1] = ATLAS_PACKET_SYNC_BYTE_1;
    frame[2] = packet->message_id;
    write_u16_be(&frame[3], packet->payload_len);
    write_u16_be(&frame[5], packet->sequence);
    write_u32_be(&frame[7], packet->timestamp_ms);

    for (i = 0u; i < packet->payload_len; ++i) {
        frame[ATLAS_PACKET_PAYLOAD_OFFSET + i] = packet->payload[i];
    }

    crc = atlas_crc16_ccitt_false(&frame[2], (uint16_t)(ATLAS_PACKET_HEADER_LEN + packet->payload_len));
    crc_index = (uint16_t)(ATLAS_PACKET_PAYLOAD_OFFSET + packet->payload_len);
    write_u16_be(&frame[crc_index], crc);

    *frame_len = total_len;
    return ATLAS_PACKET_STATUS_OK;
}

atlas_packet_status_t atlas_decode_packet(
    const uint8_t *frame,
    uint16_t frame_len,
    atlas_packet_t *packet
)
{
    uint16_t payload_len;
    uint16_t expected_len;
    uint16_t crc_index;
    uint16_t expected_crc;
    uint16_t actual_crc;
    uint16_t i;

    if ((frame == 0) || (packet == 0)) {
        return ATLAS_PACKET_STATUS_NULL_POINTER;
    }

    if (frame_len < ATLAS_PACKET_MIN_FRAME_LEN) {
        return ATLAS_PACKET_STATUS_LENGTH_MISMATCH;
    }

    if ((frame[0] != ATLAS_PACKET_SYNC_BYTE_0) || (frame[1] != ATLAS_PACKET_SYNC_BYTE_1)) {
        return ATLAS_PACKET_STATUS_INVALID_SYNC;
    }

    if (!atlas_message_id_is_valid(frame[2])) {
        return ATLAS_PACKET_STATUS_INVALID_MESSAGE_ID;
    }

    payload_len = read_u16_be(&frame[3]);
    if (payload_len > ATLAS_MAX_PAYLOAD_LEN) {
        return ATLAS_PACKET_STATUS_PAYLOAD_TOO_LARGE;
    }

    expected_len = atlas_packet_frame_len(payload_len);
    if (frame_len != expected_len) {
        return ATLAS_PACKET_STATUS_LENGTH_MISMATCH;
    }

    crc_index = (uint16_t)(ATLAS_PACKET_PAYLOAD_OFFSET + payload_len);
    expected_crc = read_u16_be(&frame[crc_index]);
    actual_crc = atlas_crc16_ccitt_false(&frame[2], (uint16_t)(ATLAS_PACKET_HEADER_LEN + payload_len));
    if (expected_crc != actual_crc) {
        return ATLAS_PACKET_STATUS_CRC_MISMATCH;
    }

    packet->message_id = frame[2];
    packet->payload_len = payload_len;
    packet->sequence = read_u16_be(&frame[5]);
    packet->timestamp_ms = read_u32_be(&frame[7]);

    for (i = 0u; i < payload_len; ++i) {
        packet->payload[i] = frame[ATLAS_PACKET_PAYLOAD_OFFSET + i];
    }

    return ATLAS_PACKET_STATUS_OK;
}

atlas_packet_status_t atlas_build_telemetry_packet(
    const atlas_telemetry_t *telemetry,
    uint16_t sequence,
    uint32_t timestamp_ms,
    atlas_packet_t *packet
)
{
    if ((telemetry == 0) || (packet == 0)) {
        return ATLAS_PACKET_STATUS_NULL_POINTER;
    }

    if (!atlas_mode_is_valid((uint8_t)telemetry->mode)) {
        return ATLAS_PACKET_STATUS_INVALID_PARAMETER;
    }

    packet->message_id = ATLAS_MESSAGE_TELEMETRY;
    packet->payload_len = ATLAS_TELEMETRY_PAYLOAD_LEN;
    packet->sequence = sequence;
    packet->timestamp_ms = timestamp_ms;

    packet->payload[0] = (uint8_t)telemetry->mode;
    write_u16_be(&packet->payload[1], (uint16_t)telemetry->temperature_deci_c);
    write_u16_be(&packet->payload[3], telemetry->voltage_mv);
    write_u16_be(&packet->payload[5], telemetry->light_raw);
    packet->payload[7] = telemetry->status_flags;
    write_u16_be(&packet->payload[8], telemetry->fault_flags);

    return ATLAS_PACKET_STATUS_OK;
}

atlas_packet_status_t atlas_build_ack_packet(
    atlas_ack_code_t ack_code,
    uint16_t sequence,
    uint32_t timestamp_ms,
    atlas_packet_t *packet
)
{
    if (packet == 0) {
        return ATLAS_PACKET_STATUS_NULL_POINTER;
    }

    if ((ack_code != ATLAS_ACK_CODE_ACK) && (ack_code != ATLAS_ACK_CODE_NAK)) {
        return ATLAS_PACKET_STATUS_INVALID_PARAMETER;
    }

    packet->message_id = ATLAS_MESSAGE_ACK;
    packet->payload_len = 1u;
    packet->sequence = sequence;
    packet->timestamp_ms = timestamp_ms;
    packet->payload[0] = (uint8_t)ack_code;

    return ATLAS_PACKET_STATUS_OK;
}

atlas_packet_status_t atlas_build_command_response_packet(
    atlas_command_response_t response_code,
    uint16_t sequence,
    uint32_t timestamp_ms,
    atlas_packet_t *packet
)
{
    if (packet == 0) {
        return ATLAS_PACKET_STATUS_NULL_POINTER;
    }

    packet->message_id = ATLAS_MESSAGE_COMMAND_RESPONSE;
    packet->payload_len = 1u;
    packet->sequence = sequence;
    packet->timestamp_ms = timestamp_ms;
    packet->payload[0] = (uint8_t)response_code;

    return ATLAS_PACKET_STATUS_OK;
}

atlas_packet_status_t atlas_build_event_packet(
    atlas_event_code_t event_code,
    uint16_t sequence,
    uint32_t timestamp_ms,
    atlas_packet_t *packet
)
{
    if (packet == 0) {
        return ATLAS_PACKET_STATUS_NULL_POINTER;
    }

    packet->message_id = ATLAS_MESSAGE_EVENT;
    packet->payload_len = 1u;
    packet->sequence = sequence;
    packet->timestamp_ms = timestamp_ms;
    packet->payload[0] = (uint8_t)event_code;

    return ATLAS_PACKET_STATUS_OK;
}

atlas_packet_status_t atlas_parse_command_packet(
    const atlas_packet_t *packet,
    atlas_command_t *command
)
{
    if ((packet == 0) || (command == 0)) {
        return ATLAS_PACKET_STATUS_NULL_POINTER;
    }

    if (packet->message_id != ATLAS_MESSAGE_COMMAND) {
        return ATLAS_PACKET_STATUS_INVALID_MESSAGE_ID;
    }

    if (packet->payload_len == 0u) {
        return ATLAS_PACKET_STATUS_INVALID_PAYLOAD;
    }

    command->command_id = (atlas_command_id_t)packet->payload[0];
    command->args_len = (uint8_t)(packet->payload_len - 1u);

    switch (command->command_id) {
    case ATLAS_COMMAND_SET_MODE:
        if (packet->payload_len != 2u) {
            return ATLAS_PACKET_STATUS_INVALID_PAYLOAD;
        }
        if (!atlas_mode_is_valid(packet->payload[1])) {
            return ATLAS_PACKET_STATUS_INVALID_PARAMETER;
        }
        command->args[0] = packet->payload[1];
        break;

    case ATLAS_COMMAND_REQUEST_STATUS:
    case ATLAS_COMMAND_CLEAR_FAULTS:
        if (packet->payload_len != 1u) {
            return ATLAS_PACKET_STATUS_INVALID_PAYLOAD;
        }
        break;

    case ATLAS_COMMAND_SET_TELEMETRY_ENABLE:
        if (packet->payload_len != 2u) {
            return ATLAS_PACKET_STATUS_INVALID_PAYLOAD;
        }
        if ((packet->payload[1] != 0x00u) && (packet->payload[1] != 0x01u)) {
            return ATLAS_PACKET_STATUS_INVALID_PARAMETER;
        }
        command->args[0] = packet->payload[1];
        break;

    case ATLAS_COMMAND_SMALL_FAULT:
        if (packet->payload_len != 2u) {
            return ATLAS_PACKET_STATUS_INVALID_PAYLOAD;
        }
        if (!atlas_small_fault_code_is_valid(packet->payload[1])) {
            return ATLAS_PACKET_STATUS_INVALID_PARAMETER;
        }
        command->args[0] = packet->payload[1];
        break;

    case ATLAS_COMMAND_MAJOR_FAULT:
        if (packet->payload_len != 2u) {
            return ATLAS_PACKET_STATUS_INVALID_PAYLOAD;
        }
        if (!atlas_major_fault_code_is_valid(packet->payload[1])) {
            return ATLAS_PACKET_STATUS_INVALID_PARAMETER;
        }
        command->args[0] = packet->payload[1];
        break;

    default:
        return ATLAS_PACKET_STATUS_UNSUPPORTED_COMMAND;
    }

    return ATLAS_PACKET_STATUS_OK;
}


