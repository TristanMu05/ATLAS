#include "stm32f4xx.h"
#include <stdbool.h>
#include <math.h>
#include <stdint.h>
#include "packet.h"
#include "uart.h"

#define GPIOAEN (1U<<0)
#define GPIOBEN (1U<<1)
#define GPIOCEN (1U<<2)
#define ADC1EN (1U<<8)
#define SYS_CORE_CLOCK_HZ 16000000U

#define LIGHT_SENSOR_PIN 0U
#define LIGHT_SENSOR_CHANNEL 0U
#define TEMP_SENSOR_PIN 1U
#define TEMP_SENSOR_CHANNEL 1U
#define VREFINT_CHANNEL 17U

#define LIGHT_R_PIN (1U<<9)
#define LIGHT_G_PIN (1U<<7)
#define LIGHT_B_PIN (1U<<6)

#define TEMP_R_PIN (1U<<5)
#define TEMP_G_PIN (1U<<6)
#define TEMP_B_PIN (1U<<7)

#define LIGHT_SAMPLE_COUNT 8U
#define TEMP_SAMPLE_COUNT 16U
#define VREFINT_SAMPLE_COUNT 4U
#define LIGHT_BASELINE_SAMPLES 128U
#define TEMP_BASELINE_SAMPLES 256U
#define SENSOR_SAMPLE_DELAY_CYCLES 250U
#define VREFINT_STARTUP_DELAY_CYCLES 2000U
#define SENSOR_UPDATE_PERIOD_MS 50U
#define VOLTAGE_UPDATE_PERIOD_MS 250U

#define LIGHT_STABLE_THRESHOLD 32U
#define LIGHT_CYAN_THRESHOLD 96U
#define LIGHT_GREEN_THRESHOLD 224U
#define LIGHT_YELLOW_THRESHOLD 448U

#define TEMP_BASELINE_DECI_C 250
#define TEMP_CALIBRATION_OFFSET_DECI_C 10
#define THERMISTOR_SERIES_RESISTOR_OHMS 10000.0f
#define THERMISTOR_NOMINAL_RESISTANCE_OHMS 10000.0f
#define THERMISTOR_BETA_COEFFICIENT 3950.0f
#define THERMISTOR_NOMINAL_TEMP_K 298.15f
#define THERMISTOR_KELVIN_OFFSET 273.15f
#define ADC_MAX_COUNTS 4095U
#define ADC_MAX_COUNTS_F 4095.0f
#define TEMP_DECI_C_MIN (-400)
#define TEMP_DECI_C_MAX 1250
#define TEMP_LED_STABLE_DECI_C 10
#define TEMP_LED_CYAN_DECI_C 20
#define TEMP_LED_GREEN_DECI_C 35
#define TEMP_LED_YELLOW_DECI_C 60
#define DEFAULT_VDDA_MV 3300U
#define VREFINT_TYPICAL_MV 1210U
#ifndef ADC_CCR_TSVREFE
#define ADC_CCR_TSVREFE (1U << 23)
#endif

typedef struct {
	atlas_mode_t mode;
	uint8_t status_flags;
	uint16_t fault_flags;
	uint16_t next_sequence;
	uint32_t last_telemetry_ms;
	uint8_t active_major_fault;
	uint32_t major_fault_until_ms;
	uint32_t light_baseline_adc;
	uint32_t temp_baseline_adc;
	uint32_t last_environment_sample_ms;
	uint32_t last_voltage_sample_ms;
	uint16_t cached_light_raw;
	int16_t cached_temperature_deci_c;
	uint16_t cached_voltage_mv;
} atlas_firmware_state_t;

typedef enum {
	ATLAS_DEMO_SMALL_FAULT_NONE = 0u,
	ATLAS_DEMO_SMALL_FAULT_CRC = 1u,
	ATLAS_DEMO_SMALL_FAULT_SYNC = 2u,
	ATLAS_DEMO_SMALL_FAULT_LENGTH = 3u,
	ATLAS_DEMO_SMALL_FAULT_SEQUENCE = 4u
} atlas_demo_small_fault_t;

typedef enum {
	ATLAS_DEMO_MAJOR_FAULT_NONE = 0u,
	ATLAS_DEMO_MAJOR_FAULT_TEMP_SPIKE = 1u,
	ATLAS_DEMO_MAJOR_FAULT_VOLTAGE_SPIKE = 2u,
	ATLAS_DEMO_MAJOR_FAULT_LIGHT_SENSOR_FAILURE = 3u,
	ATLAS_DEMO_MAJOR_FAULT_TEMP_DIP = 4u,
	ATLAS_DEMO_MAJOR_FAULT_VOLTAGE_DROP = 5u
} atlas_demo_major_fault_t;

static volatile uint32_t system_millis = 0U;
static uint8_t rx_frame[ATLAS_MAX_FRAME_LEN];
static uint8_t tx_frame[ATLAS_MAX_FRAME_LEN];
static uint16_t rx_frame_len = 0U;

void SystemInit(void)
{
	/* Enable CP10 and CP11 so hard-float math works before main(). */
	SCB->CPACR |= ((3UL << 20U) | (3UL << 22U));
	__DSB();
	__ISB();
}

void SysTick_Handler(void)
{
	system_millis++;
}

static void led_init(void)
{
	RCC->AHB1ENR |= GPIOAEN | GPIOBEN | GPIOCEN;

	GPIOA->MODER &= ~((3U << (5U * 2U)) | (3U << (6U * 2U)) | (3U << (7U * 2U)) | (3U << (9U * 2U)));
	GPIOA->MODER |= ((1U << (5U * 2U)) | (1U << (6U * 2U)) | (1U << (7U * 2U)) | (1U << (9U * 2U)));
	GPIOA->OTYPER &= ~(TEMP_R_PIN | TEMP_G_PIN | TEMP_B_PIN | LIGHT_R_PIN);
	GPIOA->PUPDR &= ~((3U << (5U * 2U)) | (3U << (6U * 2U)) | (3U << (7U * 2U)) | (3U << (9U * 2U)));

	GPIOB->MODER &= ~(3U << (6U * 2U));
	GPIOB->MODER |= (1U << (6U * 2U));
	GPIOB->OTYPER &= ~LIGHT_B_PIN;
	GPIOB->PUPDR &= ~(3U << (6U * 2U));

	GPIOC->MODER &= ~(3U << (7U * 2U));
	GPIOC->MODER |= (1U << (7U * 2U));
	GPIOC->OTYPER &= ~LIGHT_G_PIN;
	GPIOC->PUPDR &= ~(3U << (7U * 2U));

	GPIOA->BSRR = ((uint32_t)(TEMP_R_PIN | TEMP_G_PIN | TEMP_B_PIN | LIGHT_R_PIN) << 16);
	GPIOB->BSRR = ((uint32_t)LIGHT_B_PIN << 16);
	GPIOC->BSRR = ((uint32_t)LIGHT_G_PIN << 16);
}

static void set_led_for_mode(atlas_mode_t mode)
{
	(void)mode;
}

static void systick_init(void)
{
	SysTick->LOAD = (SYS_CORE_CLOCK_HZ / 1000U) - 1U;
	SysTick->VAL = 0U;
	SysTick->CTRL = SysTick_CTRL_CLKSOURCE_Msk
		| SysTick_CTRL_TICKINT_Msk
		| SysTick_CTRL_ENABLE_Msk;
}

static uint32_t atlas_millis(void)
{
	return system_millis;
}

static void delay_cycles(volatile uint32_t count)
{
	while (count-- > 0U) {
	}
}

static void light_rgb_write(uint32_t red_on, uint32_t green_on, uint32_t blue_on)
{
	GPIOA->BSRR = red_on ? LIGHT_R_PIN : (LIGHT_R_PIN << 16);
	GPIOC->BSRR = green_on ? LIGHT_G_PIN : (LIGHT_G_PIN << 16);
	GPIOB->BSRR = blue_on ? LIGHT_B_PIN : (LIGHT_B_PIN << 16);
}

static void temp_rgb_write(uint32_t red_on, uint32_t green_on, uint32_t blue_on)
{
	GPIOA->BSRR =
		(red_on ? TEMP_R_PIN : (TEMP_R_PIN << 16))
		| (green_on ? TEMP_G_PIN : (TEMP_G_PIN << 16))
		| (blue_on ? TEMP_B_PIN : (TEMP_B_PIN << 16));
}

static void update_light_led(uint32_t light_baseline, uint16_t light_current)
{
	uint32_t shadow_delta = (light_baseline > light_current) ? (light_baseline - light_current) : 0U;

	if (shadow_delta < LIGHT_STABLE_THRESHOLD) {
		light_rgb_write(0U, 0U, 1U);
	} else if (shadow_delta < LIGHT_CYAN_THRESHOLD) {
		light_rgb_write(0U, 1U, 1U);
	} else if (shadow_delta < LIGHT_GREEN_THRESHOLD) {
		light_rgb_write(0U, 1U, 0U);
	} else if (shadow_delta < LIGHT_YELLOW_THRESHOLD) {
		light_rgb_write(1U, 1U, 0U);
	} else {
		light_rgb_write(1U, 0U, 0U);
	}
}

static void update_temp_led(int16_t temperature_deci_c)
{
	int16_t temp_delta_deci_c = temperature_deci_c - TEMP_BASELINE_DECI_C;
	if (temp_delta_deci_c < 0) {
		temp_delta_deci_c = (int16_t)(-temp_delta_deci_c);
	}

	if (temp_delta_deci_c < TEMP_LED_STABLE_DECI_C) {
		temp_rgb_write(0U, 0U, 1U);
	} else if (temp_delta_deci_c < TEMP_LED_CYAN_DECI_C) {
		temp_rgb_write(0U, 1U, 1U);
	} else if (temp_delta_deci_c < TEMP_LED_GREEN_DECI_C) {
		temp_rgb_write(0U, 1U, 0U);
	} else if (temp_delta_deci_c < TEMP_LED_YELLOW_DECI_C) {
		temp_rgb_write(1U, 1U, 0U);
	} else {
		temp_rgb_write(1U, 0U, 0U);
	}
}

static void apply_major_fault_led_override(const atlas_firmware_state_t *state)
{
	switch ((atlas_demo_major_fault_t)state->active_major_fault) {
	case ATLAS_DEMO_MAJOR_FAULT_TEMP_SPIKE:
		temp_rgb_write(1U, 0U, 0U);
		break;
	case ATLAS_DEMO_MAJOR_FAULT_TEMP_DIP:
		temp_rgb_write(1U, 1U, 0U);
		break;
	case ATLAS_DEMO_MAJOR_FAULT_LIGHT_SENSOR_FAILURE:
		light_rgb_write(1U, 0U, 0U);
		break;
	case ATLAS_DEMO_MAJOR_FAULT_VOLTAGE_SPIKE:
		light_rgb_write(1U, 0U, 1U);
		temp_rgb_write(1U, 0U, 1U);
		break;
	case ATLAS_DEMO_MAJOR_FAULT_VOLTAGE_DROP:
		light_rgb_write(1U, 1U, 0U);
		temp_rgb_write(1U, 1U, 0U);
		break;
	case ATLAS_DEMO_MAJOR_FAULT_NONE:
	default:
		break;
	}
}

static void analog_sensor_init(void)
{
	RCC->AHB1ENR |= GPIOAEN;
	RCC->APB2ENR |= ADC1EN;

	GPIOA->MODER &= ~((3U << (LIGHT_SENSOR_PIN * 2U)) | (3U << (TEMP_SENSOR_PIN * 2U)));
	GPIOA->MODER |= ((3U << (LIGHT_SENSOR_PIN * 2U)) | (3U << (TEMP_SENSOR_PIN * 2U)));
	GPIOA->PUPDR &= ~((3U << (LIGHT_SENSOR_PIN * 2U)) | (3U << (TEMP_SENSOR_PIN * 2U)));

	ADC1->CR1 = 0U;
	ADC1->CR2 = 0U;
	ADC1->SMPR2 &= ~((7U << (LIGHT_SENSOR_CHANNEL * 3U)) | (7U << (TEMP_SENSOR_CHANNEL * 3U)));
	ADC1->SMPR2 |= ((7U << (LIGHT_SENSOR_CHANNEL * 3U)) | (7U << (TEMP_SENSOR_CHANNEL * 3U)));
	ADC1->SMPR1 &= ~(7U << ((VREFINT_CHANNEL - 10U) * 3U));
	ADC1->SMPR1 |= (7U << ((VREFINT_CHANNEL - 10U) * 3U));
	ADC1->SQR1 &= ~(0xFU << 20);
	ADC->CCR |= ADC_CCR_TSVREFE;
	delay_cycles(VREFINT_STARTUP_DELAY_CYCLES);
	ADC1->CR2 |= (1U << 10);
	ADC1->CR2 |= (1U << 0);
}

static uint16_t adc1_read_channel(uint32_t channel)
{
	ADC1->SQR3 = (channel & 0x1FU);
	ADC1->CR2 |= (1U << 30);

	while ((ADC1->SR & (1U << 1)) == 0U) {
	}

	return (uint16_t)ADC1->DR;
}

static uint16_t adc1_read_average(uint32_t channel, uint32_t samples)
{
	uint32_t total = 0U;
	uint32_t sample_index;

	for (sample_index = 0U; sample_index < samples; ++sample_index) {
		total += adc1_read_channel(channel);
		delay_cycles(SENSOR_SAMPLE_DELAY_CYCLES);
	}

	return (uint16_t)(total / samples);
}

static void capture_sensor_baselines(atlas_firmware_state_t *state)
{
	state->light_baseline_adc = adc1_read_average(LIGHT_SENSOR_CHANNEL, LIGHT_BASELINE_SAMPLES);
	state->temp_baseline_adc = adc1_read_average(TEMP_SENSOR_CHANNEL, TEMP_BASELINE_SAMPLES);
}

static uint32_t absolute_delta_u32(uint32_t baseline, uint16_t current)
{
	return (baseline > current) ? (baseline - current) : ((uint32_t)current - baseline);
}

static int16_t clamp_temperature_deci_c(int32_t value)
{
	if (value < TEMP_DECI_C_MIN) {
		return TEMP_DECI_C_MIN;
	}

	if (value > TEMP_DECI_C_MAX) {
		return TEMP_DECI_C_MAX;
	}

	return (int16_t)value;
}

static int16_t estimate_temperature_deci_c(const atlas_firmware_state_t *state, uint16_t temp_current)
{
	float adc_counts = (float)temp_current;
	float thermistor_resistance;
	float inverse_temp_k;
	float temperature_c;
	int32_t estimated;

	(void)state;

	if ((adc_counts <= 0.0f) || (adc_counts >= ADC_MAX_COUNTS_F)) {
		return TEMP_BASELINE_DECI_C;
	}

	//thermistor_resistance = THERMISTOR_SERIES_RESISTOR_OHMS * adc_counts / (ADC_MAX_COUNTS_F - adc_counts);
	thermistor_resistance = THERMISTOR_SERIES_RESISTOR_OHMS * (ADC_MAX_COUNTS_F / adc_counts - 1.0f);
	if (thermistor_resistance <= 0.0f) {
		return TEMP_BASELINE_DECI_C;
	}

	inverse_temp_k =
		(1.0f / THERMISTOR_NOMINAL_TEMP_K)
		+ (logf(thermistor_resistance / THERMISTOR_NOMINAL_RESISTANCE_OHMS) / THERMISTOR_BETA_COEFFICIENT);
	temperature_c = (1.0f / inverse_temp_k) - THERMISTOR_KELVIN_OFFSET;
	estimated = (temperature_c >= 0.0f)
		? (int32_t)((temperature_c * 10.0f) + 0.5f)
		: (int32_t)((temperature_c * 10.0f) - 0.5f);
	estimated += TEMP_CALIBRATION_OFFSET_DECI_C;

	return clamp_temperature_deci_c(estimated);
}

static uint16_t estimate_vdda_mv(void)
{
	uint16_t vrefint_counts = adc1_read_average(VREFINT_CHANNEL, VREFINT_SAMPLE_COUNT);
	uint32_t supply_mv;

	if (vrefint_counts == 0U) {
		return DEFAULT_VDDA_MV;
	}

	supply_mv = (((uint32_t)VREFINT_TYPICAL_MV * ADC_MAX_COUNTS) + (vrefint_counts / 2U)) / vrefint_counts;
	return (uint16_t)supply_mv;
}

static void refresh_environment_cache(
	atlas_firmware_state_t *state,
	uint32_t timestamp_ms,
	bool force
)
{
	uint16_t light_current;
	uint16_t temp_current;
	uint32_t light_delta;
	int16_t estimated_temperature;

	if ((!force) && ((timestamp_ms - state->last_environment_sample_ms) < SENSOR_UPDATE_PERIOD_MS)) {
		return;
	}

	light_current = adc1_read_average(LIGHT_SENSOR_CHANNEL, LIGHT_SAMPLE_COUNT);
	temp_current = adc1_read_average(TEMP_SENSOR_CHANNEL, TEMP_SAMPLE_COUNT);
	light_delta = absolute_delta_u32(state->light_baseline_adc, light_current);
	estimated_temperature = estimate_temperature_deci_c(state, temp_current);

	if (light_current > state->light_baseline_adc) {
		state->light_baseline_adc = ((state->light_baseline_adc * 255U) + light_current) / 256U;
	} else if (light_delta < LIGHT_STABLE_THRESHOLD) {
		state->light_baseline_adc = ((state->light_baseline_adc * 127U) + light_current) / 128U;
	}

	update_light_led(state->light_baseline_adc, light_current);
	update_temp_led(estimated_temperature);
	apply_major_fault_led_override(state);

	state->cached_light_raw = light_current;
	state->cached_temperature_deci_c = estimated_temperature;
	state->last_environment_sample_ms = timestamp_ms;
}

static void refresh_voltage_cache(
	atlas_firmware_state_t *state,
	uint32_t timestamp_ms,
	bool force
)
{
	if ((!force) && ((timestamp_ms - state->last_voltage_sample_ms) < VOLTAGE_UPDATE_PERIOD_MS)) {
		return;
	}

	state->cached_voltage_mv = estimate_vdda_mv();
	state->last_voltage_sample_ms = timestamp_ms;
}

static uint16_t next_sequence(atlas_firmware_state_t *state)
{
	uint16_t sequence = state->next_sequence;
	state->next_sequence++;
	return sequence;
}

static uint32_t telemetry_period_ms(atlas_mode_t mode)
{
	switch (mode) {
	case ATLAS_MODE_IDLE:
		return 500U;
	case ATLAS_MODE_SAFE:
		return 200U;
	case ATLAS_MODE_NORMAL:
	case ATLAS_MODE_DIAGNOSTIC:
	default:
		return 100U;
	}
}

static bool blocking_critical_faults_active(const atlas_firmware_state_t *state)
{
	if (state->active_major_fault != ATLAS_DEMO_MAJOR_FAULT_NONE) {
		return true;
	}

	return (state->fault_flags & (
		ATLAS_FAULT_OVER_TEMPERATURE
		| ATLAS_FAULT_LOW_VOLTAGE
		| ATLAS_FAULT_UNDER_TEMPERATURE
		| ATLAS_FAULT_HIGH_VOLTAGE
	)) != 0U;
}

static uint16_t active_major_fault_mask(const atlas_firmware_state_t *state)
{
	switch ((atlas_demo_major_fault_t)state->active_major_fault) {
	case ATLAS_DEMO_MAJOR_FAULT_TEMP_SPIKE:
		return ATLAS_FAULT_OVER_TEMPERATURE;
	case ATLAS_DEMO_MAJOR_FAULT_VOLTAGE_SPIKE:
		return ATLAS_FAULT_HIGH_VOLTAGE;
	case ATLAS_DEMO_MAJOR_FAULT_LIGHT_SENSOR_FAILURE:
		return ATLAS_FAULT_LIGHT_SENSOR;
	case ATLAS_DEMO_MAJOR_FAULT_TEMP_DIP:
		return ATLAS_FAULT_UNDER_TEMPERATURE;
	case ATLAS_DEMO_MAJOR_FAULT_VOLTAGE_DROP:
		return ATLAS_FAULT_LOW_VOLTAGE;
	case ATLAS_DEMO_MAJOR_FAULT_NONE:
	default:
		return 0U;
	}
}

static void refresh_degraded_status(atlas_firmware_state_t *state)
{
	if (state->fault_flags != 0U) {
		state->status_flags |= ATLAS_STATUS_DEGRADED_OPERATION;
	} else {
		state->status_flags &= (uint8_t)~ATLAS_STATUS_DEGRADED_OPERATION;
	}
}

static void clear_active_major_fault(atlas_firmware_state_t *state)
{
	switch ((atlas_demo_major_fault_t)state->active_major_fault) {
	case ATLAS_DEMO_MAJOR_FAULT_LIGHT_SENSOR_FAILURE:
		state->status_flags |= ATLAS_STATUS_LIGHT_VALID;
		break;
	case ATLAS_DEMO_MAJOR_FAULT_NONE:
	default:
		break;
	}

	state->fault_flags &= (uint16_t)~active_major_fault_mask(state);
	state->active_major_fault = ATLAS_DEMO_MAJOR_FAULT_NONE;
	state->major_fault_until_ms = 0U;
	refresh_degraded_status(state);
}

static void expire_major_fault_if_needed(atlas_firmware_state_t *state, uint32_t timestamp_ms)
{
	if ((state->active_major_fault != ATLAS_DEMO_MAJOR_FAULT_NONE)
		&& (timestamp_ms >= state->major_fault_until_ms)) {
		clear_active_major_fault(state);
	}
}

static void force_safe_mode(atlas_firmware_state_t *state, atlas_event_code_t *event_code)
{
	if (state->mode != ATLAS_MODE_SAFE) {
		state->mode = ATLAS_MODE_SAFE;
		set_led_for_mode(state->mode);
		*event_code = ATLAS_EVENT_SAFE_MODE_ENTERED;
	}
}

static void activate_major_fault(
	atlas_firmware_state_t *state,
	atlas_demo_major_fault_t fault_kind,
	uint32_t timestamp_ms,
	atlas_event_code_t *event_code
)
{
	clear_active_major_fault(state);
	state->active_major_fault = (uint8_t)fault_kind;
	state->major_fault_until_ms = timestamp_ms + 5000U;

	switch (fault_kind) {
	case ATLAS_DEMO_MAJOR_FAULT_TEMP_SPIKE:
		state->fault_flags |= ATLAS_FAULT_OVER_TEMPERATURE;
		break;
	case ATLAS_DEMO_MAJOR_FAULT_VOLTAGE_SPIKE:
		state->fault_flags |= ATLAS_FAULT_HIGH_VOLTAGE;
		break;
	case ATLAS_DEMO_MAJOR_FAULT_LIGHT_SENSOR_FAILURE:
		state->fault_flags |= ATLAS_FAULT_LIGHT_SENSOR;
		state->status_flags &= (uint8_t)~ATLAS_STATUS_LIGHT_VALID;
		break;
	case ATLAS_DEMO_MAJOR_FAULT_TEMP_DIP:
		state->fault_flags |= ATLAS_FAULT_UNDER_TEMPERATURE;
		break;
	case ATLAS_DEMO_MAJOR_FAULT_VOLTAGE_DROP:
		state->fault_flags |= ATLAS_FAULT_LOW_VOLTAGE;
		break;
	case ATLAS_DEMO_MAJOR_FAULT_NONE:
	default:
		break;
	}

	refresh_degraded_status(state);
	force_safe_mode(state, event_code);
}

static void firmware_state_init(atlas_firmware_state_t *state)
{
	state->mode = ATLAS_MODE_IDLE;
	state->status_flags = ATLAS_STATUS_TEMP_VALID
		| ATLAS_STATUS_LIGHT_VALID
		| ATLAS_STATUS_VOLTAGE_VALID
		| ATLAS_STATUS_TELEMETRY_ENABLED;
	state->fault_flags = 0U;
	state->next_sequence = 0U;
	state->last_telemetry_ms = 0U;
	state->active_major_fault = ATLAS_DEMO_MAJOR_FAULT_NONE;
	state->major_fault_until_ms = 0U;
	state->light_baseline_adc = 0U;
	state->temp_baseline_adc = 0U;
	state->last_environment_sample_ms = 0U;
	state->last_voltage_sample_ms = 0U;
	state->cached_light_raw = 0U;
	state->cached_temperature_deci_c = TEMP_BASELINE_DECI_C;
	state->cached_voltage_mv = DEFAULT_VDDA_MV;
	set_led_for_mode(state->mode);
}

static void build_live_telemetry(atlas_firmware_state_t *state, atlas_telemetry_t *telemetry, uint32_t timestamp_ms)
{
	refresh_environment_cache(state, timestamp_ms, false);
	refresh_voltage_cache(state, timestamp_ms, false);

	telemetry->mode = state->mode;
	telemetry->temperature_deci_c = state->cached_temperature_deci_c;
	telemetry->voltage_mv = state->cached_voltage_mv;
	telemetry->light_raw = state->cached_light_raw;

	switch ((atlas_demo_major_fault_t)state->active_major_fault) {
	case ATLAS_DEMO_MAJOR_FAULT_TEMP_SPIKE:
		telemetry->temperature_deci_c = 1050;
		break;
	case ATLAS_DEMO_MAJOR_FAULT_VOLTAGE_SPIKE:
		telemetry->voltage_mv = 5200U;
		break;
	case ATLAS_DEMO_MAJOR_FAULT_LIGHT_SENSOR_FAILURE:
		telemetry->light_raw = 0U;
		break;
	case ATLAS_DEMO_MAJOR_FAULT_TEMP_DIP:
		telemetry->temperature_deci_c = -200;
		break;
	case ATLAS_DEMO_MAJOR_FAULT_VOLTAGE_DROP:
		telemetry->voltage_mv = 2800U;
		break;
	case ATLAS_DEMO_MAJOR_FAULT_NONE:
	default:
		break;
	}

	telemetry->status_flags = state->status_flags;
	telemetry->fault_flags = state->fault_flags;
}

static void transmit_packet(const atlas_packet_t *packet, atlas_firmware_state_t *state)
{
	uint16_t frame_len = 0U;

	if (atlas_encode_packet(packet, tx_frame, sizeof(tx_frame), &frame_len) != ATLAS_PACKET_STATUS_OK) {
		state->fault_flags |= ATLAS_FAULT_PLATFORM_INIT;
		state->status_flags |= ATLAS_STATUS_DEGRADED_OPERATION;
		return;
	}

	uart2_write_buffer(tx_frame, frame_len);
}

static void send_ack(atlas_firmware_state_t *state, atlas_ack_code_t ack_code, uint32_t timestamp_ms)
{
	atlas_packet_t packet;

	if (atlas_build_ack_packet(ack_code, next_sequence(state), timestamp_ms, &packet) == ATLAS_PACKET_STATUS_OK) {
		transmit_packet(&packet, state);
	}
}

static void send_command_response(
	atlas_firmware_state_t *state,
	atlas_command_response_t response,
	uint32_t timestamp_ms
)
{
	atlas_packet_t packet;

	if (atlas_build_command_response_packet(response, next_sequence(state), timestamp_ms, &packet) == ATLAS_PACKET_STATUS_OK) {
		transmit_packet(&packet, state);
	}
}

static void send_event(atlas_firmware_state_t *state, atlas_event_code_t event_code, uint32_t timestamp_ms)
{
	atlas_packet_t packet;

	if (atlas_build_event_packet(event_code, next_sequence(state), timestamp_ms, &packet) == ATLAS_PACKET_STATUS_OK) {
		transmit_packet(&packet, state);
	}
}

static void send_telemetry(atlas_firmware_state_t *state, uint32_t timestamp_ms)
{
	atlas_telemetry_t telemetry;
	atlas_packet_t packet;

	build_live_telemetry(state, &telemetry, timestamp_ms);
	if (atlas_build_telemetry_packet(&telemetry, next_sequence(state), timestamp_ms, &packet) == ATLAS_PACKET_STATUS_OK) {
		transmit_packet(&packet, state);
	}
}

static void send_raw_frame(const uint8_t *frame, uint16_t frame_len)
{
	uart2_write_buffer(frame, frame_len);
}

static void inject_small_fault(
	atlas_firmware_state_t *state,
	atlas_demo_small_fault_t fault_kind,
	uint32_t timestamp_ms
)
{
	atlas_telemetry_t telemetry;
	atlas_packet_t packet;
	uint16_t frame_len = 0U;

	if (fault_kind == ATLAS_DEMO_SMALL_FAULT_SEQUENCE) {
		state->next_sequence++;
		send_telemetry(state, timestamp_ms);
		return;
	}

	build_live_telemetry(state, &telemetry, timestamp_ms);
	if (atlas_build_telemetry_packet(&telemetry, state->next_sequence, timestamp_ms, &packet) != ATLAS_PACKET_STATUS_OK) {
		return;
	}

	if (atlas_encode_packet(&packet, tx_frame, sizeof(tx_frame), &frame_len) != ATLAS_PACKET_STATUS_OK) {
		state->fault_flags |= ATLAS_FAULT_PLATFORM_INIT;
		refresh_degraded_status(state);
		return;
	}

	switch (fault_kind) {
	case ATLAS_DEMO_SMALL_FAULT_CRC:
		tx_frame[frame_len - 1U] ^= 0x01U;
		break;
	case ATLAS_DEMO_SMALL_FAULT_SYNC:
		tx_frame[0] = 0x00U;
		tx_frame[1] = 0x00U;
		break;
	case ATLAS_DEMO_SMALL_FAULT_LENGTH:
		tx_frame[3] = 0x01U;
		tx_frame[4] = 0x01U;
		break;
	case ATLAS_DEMO_SMALL_FAULT_NONE:
	case ATLAS_DEMO_SMALL_FAULT_SEQUENCE:
	default:
		break;
	}

	send_raw_frame(tx_frame, frame_len);
}

static atlas_command_response_t apply_mode_request(
	atlas_firmware_state_t *state,
	atlas_mode_t requested_mode,
	atlas_event_code_t *event_code
)
{
	atlas_mode_t previous_mode = state->mode;

	if (requested_mode == previous_mode) {
		return ATLAS_COMMAND_RESPONSE_COMPLETED;
	}

	if (requested_mode == ATLAS_MODE_SAFE) {
		state->mode = ATLAS_MODE_SAFE;
		set_led_for_mode(state->mode);
		if (previous_mode != ATLAS_MODE_SAFE) {
			*event_code = ATLAS_EVENT_SAFE_MODE_ENTERED;
		}
		return ATLAS_COMMAND_RESPONSE_COMPLETED;
	}

	switch (previous_mode) {
	case ATLAS_MODE_IDLE:
		if (requested_mode == ATLAS_MODE_NORMAL) {
			if (blocking_critical_faults_active(state)) {
				return ATLAS_COMMAND_RESPONSE_FAULT_ACTIVE;
			}
			state->mode = requested_mode;
			set_led_for_mode(state->mode);
			return ATLAS_COMMAND_RESPONSE_COMPLETED;
		}
		if (requested_mode == ATLAS_MODE_DIAGNOSTIC) {
			state->mode = requested_mode;
			set_led_for_mode(state->mode);
			return ATLAS_COMMAND_RESPONSE_COMPLETED;
		}
		return ATLAS_COMMAND_RESPONSE_INVALID_MODE;

	case ATLAS_MODE_NORMAL:
		if (requested_mode == ATLAS_MODE_IDLE) {
			state->mode = requested_mode;
			set_led_for_mode(state->mode);
			return ATLAS_COMMAND_RESPONSE_COMPLETED;
		}
		return ATLAS_COMMAND_RESPONSE_INVALID_MODE;

	case ATLAS_MODE_DIAGNOSTIC:
		if (requested_mode == ATLAS_MODE_IDLE) {
			state->mode = requested_mode;
			set_led_for_mode(state->mode);
			return ATLAS_COMMAND_RESPONSE_COMPLETED;
		}
		return ATLAS_COMMAND_RESPONSE_INVALID_MODE;

	case ATLAS_MODE_SAFE:
		if (requested_mode == ATLAS_MODE_IDLE) {
			if (blocking_critical_faults_active(state)) {
				return ATLAS_COMMAND_RESPONSE_FAULT_ACTIVE;
			}
			state->mode = requested_mode;
			set_led_for_mode(state->mode);
			*event_code = ATLAS_EVENT_SAFE_MODE_EXITED;
			return ATLAS_COMMAND_RESPONSE_COMPLETED;
		}
		return ATLAS_COMMAND_RESPONSE_INVALID_MODE;

	default:
		return ATLAS_COMMAND_RESPONSE_REJECTED;
	}
}

static atlas_command_response_t execute_command(
	atlas_firmware_state_t *state,
	const atlas_command_t *command,
	uint32_t timestamp_ms,
	bool *send_status_now,
	atlas_event_code_t *event_code,
	atlas_demo_small_fault_t *small_fault
)
{
	switch (command->command_id) {
	case ATLAS_COMMAND_SET_MODE:
	{
		atlas_command_response_t response = apply_mode_request(state, (atlas_mode_t)command->args[0], event_code);

		if ((response == ATLAS_COMMAND_RESPONSE_INVALID_MODE) && (*event_code == 0U)) {
			*event_code = ATLAS_EVENT_COMMAND_INVALID_IN_MODE;
		}

		return response;
	}

	case ATLAS_COMMAND_REQUEST_STATUS:
		*send_status_now = true;
		return ATLAS_COMMAND_RESPONSE_COMPLETED;

	case ATLAS_COMMAND_CLEAR_FAULTS:
		state->fault_flags &= (uint16_t)(
			ATLAS_FAULT_OVER_TEMPERATURE
			| ATLAS_FAULT_LOW_VOLTAGE
			| ATLAS_FAULT_UNDER_TEMPERATURE
			| ATLAS_FAULT_HIGH_VOLTAGE
			| active_major_fault_mask(state)
		);
		refresh_degraded_status(state);
		return ATLAS_COMMAND_RESPONSE_COMPLETED;

	case ATLAS_COMMAND_SET_TELEMETRY_ENABLE:
		if ((command->args[0] == 0x00U) && (state->mode == ATLAS_MODE_SAFE)) {
			*event_code = ATLAS_EVENT_COMMAND_INVALID_IN_MODE;
			return ATLAS_COMMAND_RESPONSE_INVALID_MODE;
		}
		if (command->args[0] == 0x01U) {
			state->status_flags |= ATLAS_STATUS_TELEMETRY_ENABLED;
		} else {
			state->status_flags &= (uint8_t)~ATLAS_STATUS_TELEMETRY_ENABLED;
		}
		return ATLAS_COMMAND_RESPONSE_COMPLETED;

	case ATLAS_COMMAND_SMALL_FAULT:
		*small_fault = (atlas_demo_small_fault_t)command->args[0];
		*send_status_now = true;
		return ATLAS_COMMAND_RESPONSE_COMPLETED;

	case ATLAS_COMMAND_MAJOR_FAULT:
		activate_major_fault(state, (atlas_demo_major_fault_t)command->args[0], timestamp_ms, event_code);
		*send_status_now = true;

		if (*event_code == 0U) {
			switch ((atlas_demo_major_fault_t)command->args[0]) {
			case ATLAS_DEMO_MAJOR_FAULT_TEMP_SPIKE:
				*event_code = ATLAS_EVENT_OVER_TEMPERATURE;
				break;
			case ATLAS_DEMO_MAJOR_FAULT_VOLTAGE_SPIKE:
				*event_code = ATLAS_EVENT_HIGH_VOLTAGE;
				break;
			case ATLAS_DEMO_MAJOR_FAULT_LIGHT_SENSOR_FAILURE:
				*event_code = ATLAS_EVENT_LIGHT_SENSOR_READ_FAIL;
				break;
			case ATLAS_DEMO_MAJOR_FAULT_TEMP_DIP:
				*event_code = ATLAS_EVENT_UNDER_TEMPERATURE;
				break;
			case ATLAS_DEMO_MAJOR_FAULT_VOLTAGE_DROP:
				*event_code = ATLAS_EVENT_LOW_VOLTAGE;
				break;
			case ATLAS_DEMO_MAJOR_FAULT_NONE:
			default:
				break;
			}
		}

		return ATLAS_COMMAND_RESPONSE_COMPLETED;

	default:
		return ATLAS_COMMAND_RESPONSE_NOT_SUPPORTED;
	}
}

static void handle_command_packet(atlas_firmware_state_t *state, const atlas_packet_t *packet, uint32_t timestamp_ms)
{
	atlas_command_t command;
	atlas_packet_status_t parse_status = atlas_parse_command_packet(packet, &command);

	if (parse_status != ATLAS_PACKET_STATUS_OK) {
		send_ack(state, ATLAS_ACK_CODE_NAK, timestamp_ms);

		if (parse_status == ATLAS_PACKET_STATUS_UNSUPPORTED_COMMAND) {
			send_event(state, ATLAS_EVENT_UNSUPPORTED_COMMAND, timestamp_ms);
		} else {
			send_event(state, ATLAS_EVENT_MALFORMED_PACKET, timestamp_ms);
		}
		return;
	}

	send_ack(state, ATLAS_ACK_CODE_ACK, timestamp_ms);

	{
		bool send_status_now = false;
		atlas_event_code_t event_code = 0U;
		atlas_demo_small_fault_t small_fault = ATLAS_DEMO_SMALL_FAULT_NONE;
		atlas_command_response_t response = execute_command(
			state,
			&command,
			timestamp_ms,
			&send_status_now,
			&event_code,
			&small_fault
		);

		send_command_response(state, response, timestamp_ms);

		if (event_code != 0U) {
			send_event(state, event_code, timestamp_ms);
		}

		if ((response == ATLAS_COMMAND_RESPONSE_COMPLETED) && (small_fault != ATLAS_DEMO_SMALL_FAULT_NONE)) {
			inject_small_fault(state, small_fault, timestamp_ms);
		}

		if (send_status_now) {
			send_telemetry(state, timestamp_ms);
		}
	}
}

static void process_rx_byte(atlas_firmware_state_t *state, uint8_t byte, uint32_t timestamp_ms)
{
	if (rx_frame_len == 0U) {
		if (byte == ATLAS_PACKET_SYNC_BYTE_0) {
			rx_frame[rx_frame_len++] = byte;
		}
		return;
	}

	if (rx_frame_len == 1U) {
		if (byte == ATLAS_PACKET_SYNC_BYTE_1) {
			rx_frame[rx_frame_len++] = byte;
		} else if (byte == ATLAS_PACKET_SYNC_BYTE_0) {
			rx_frame[0] = byte;
		} else {
			rx_frame_len = 0U;
		}
		return;
	}

	if (rx_frame_len >= ATLAS_MAX_FRAME_LEN) {
		state->fault_flags |= ATLAS_FAULT_RX_BUFFER_OVERFLOW;
		state->status_flags |= ATLAS_STATUS_DEGRADED_OPERATION;
		rx_frame_len = 0U;
		send_event(state, ATLAS_EVENT_MALFORMED_PACKET, timestamp_ms);
		return;
	}

	rx_frame[rx_frame_len++] = byte;

	if (rx_frame_len < 5U) {
		return;
	}

	{
		uint16_t payload_len = (uint16_t)(((uint16_t)rx_frame[3] << 8) | rx_frame[4]);
		uint16_t expected_len;

		if (payload_len > ATLAS_MAX_PAYLOAD_LEN) {
			state->fault_flags |= ATLAS_FAULT_RX_BUFFER_OVERFLOW;
			state->status_flags |= ATLAS_STATUS_DEGRADED_OPERATION;
			rx_frame_len = 0U;
			send_event(state, ATLAS_EVENT_MALFORMED_PACKET, timestamp_ms);
			return;
		}

		expected_len = atlas_packet_frame_len(payload_len);
		if (rx_frame_len < expected_len) {
			return;
		}

		if (rx_frame_len > expected_len) {
			state->fault_flags |= ATLAS_FAULT_RX_BUFFER_OVERFLOW;
			state->status_flags |= ATLAS_STATUS_DEGRADED_OPERATION;
			rx_frame_len = 0U;
			send_event(state, ATLAS_EVENT_MALFORMED_PACKET, timestamp_ms);
			return;
		}
	}

	{
		atlas_packet_t packet;
		atlas_packet_status_t decode_status = atlas_decode_packet(rx_frame, rx_frame_len, &packet);

		rx_frame_len = 0U;

		if (decode_status == ATLAS_PACKET_STATUS_OK) {
			if (packet.message_id == ATLAS_MESSAGE_COMMAND) {
				handle_command_packet(state, &packet, timestamp_ms);
			} else {
				send_event(state, ATLAS_EVENT_MALFORMED_PACKET, timestamp_ms);
			}
			return;
		}

		if (decode_status == ATLAS_PACKET_STATUS_CRC_MISMATCH) {
			send_event(state, ATLAS_EVENT_CRC_FAILURE, timestamp_ms);
		} else {
			send_event(state, ATLAS_EVENT_MALFORMED_PACKET, timestamp_ms);
		}
	}
}

int main(void)
{
	atlas_firmware_state_t state;
	uint32_t boot_ms;

	led_init();
	analog_sensor_init();
	uart2_rxtx_init();
	systick_init();
	firmware_state_init(&state);
	capture_sensor_baselines(&state);
	state.status_flags |= ATLAS_STATUS_SENSORS_INITIALIZED;
	boot_ms = atlas_millis();
	refresh_environment_cache(&state, boot_ms, true);
	refresh_voltage_cache(&state, boot_ms, true);
	state.last_telemetry_ms = boot_ms;
	send_telemetry(&state, boot_ms);

	while(1)
	{
		uint32_t now_ms = atlas_millis();
		uint8_t rx_byte;

		expire_major_fault_if_needed(&state, now_ms);
		refresh_environment_cache(&state, now_ms, false);
		refresh_voltage_cache(&state, now_ms, false);

		if (uart2_take_rx_overflow()) {
			state.fault_flags |= ATLAS_FAULT_UART_RX_OVERRUN;
			refresh_degraded_status(&state);
			send_event(&state, ATLAS_EVENT_UART_RX_OVERRUN, now_ms);
		}

		while (uart2_try_read(&rx_byte)) {
			process_rx_byte(&state, rx_byte, now_ms);
			now_ms = atlas_millis();
			expire_major_fault_if_needed(&state, now_ms);
			refresh_environment_cache(&state, now_ms, false);
			refresh_voltage_cache(&state, now_ms, false);
		}

		if (((state.status_flags & ATLAS_STATUS_TELEMETRY_ENABLED) != 0U)
			&& ((now_ms - state.last_telemetry_ms) >= telemetry_period_ms(state.mode))) {
			state.last_telemetry_ms = now_ms;
			send_telemetry(&state, now_ms);
		}
	}
}
