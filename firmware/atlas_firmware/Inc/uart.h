/*
 * uart.h
 *
 *  Created on: Apr 3, 2026
 *      Author: murad
 */

#ifndef UART_H_
#define UART_H_

#include "stm32f4xx.h"
#include <stdint.h>

void uart2_rxtx_init(void);
void uart2_tx_init(void);
char uart2_read(void);


#endif /* UART_H_ */
