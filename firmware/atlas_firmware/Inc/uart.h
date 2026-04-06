/*
 * uart.h
 *
 *  Created on: Apr 3, 2026
 *      Author: murad
 */

#ifndef UART_H_
#define UART_H_

#include "stm32f4xx.h"
#include <stdbool.h>
#include <stdint.h>

void uart2_rxtx_init(void);
void uart2_tx_init(void);
char uart2_read(void);
bool uart2_try_read(uint8_t *byte);
bool uart2_take_rx_overflow(void);
void uart2_write_buffer(const uint8_t *data, uint16_t length);


#endif /* UART_H_ */
