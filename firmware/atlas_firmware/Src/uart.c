/*
 * uart.c
 *
 *  Created on: Apr 3, 2026
 *      Author: murad
 */
#include "uart.h"

#define GPIOAEN (1U<<0)
#define UART2EN (1U<<17)

#define SYS_FREQ 16000000
#define APB1_CLK SYS_FREQ

#define UART_BAUDRATE 115200

#define CR1_TE (1U<<3)
#define CR1_RE (1U<<2)
#define CR1_RXNEIE (1U<<5)
#define SR_TXE (1U<<7)
#define SR_RXNE (1U<<5)
#define SR_ORE (1U<<3)
#define CR1_UE (1U<<13)
#define UART2_RX_BUFFER_SIZE 256U

static void uart_set_baudrate(USART_TypeDef * USARTx, uint32_t PeriphClk, uint32_t BaudRate);
static uint16_t compute_uart_bd(uint32_t PeriphClk, uint32_t BaudRate);
static void uart2_write(int ch);
static volatile uint8_t uart2_rx_buffer[UART2_RX_BUFFER_SIZE];
static volatile uint16_t uart2_rx_head = 0U;
static volatile uint16_t uart2_rx_tail = 0U;
static volatile bool uart2_rx_overflow = false;

int __io_putchar(int ch){
	uart2_write(ch);
	return ch;
}

void USART2_IRQHandler(void){
	uint32_t status = USART2->SR;

	if ((status & (SR_RXNE | SR_ORE)) != 0U){
		uint8_t byte = (uint8_t)USART2->DR;
		uint16_t next_head = (uint16_t)((uart2_rx_head + 1U) % UART2_RX_BUFFER_SIZE);

		if (next_head == uart2_rx_tail){
			uart2_rx_overflow = true;
		}else{
			uart2_rx_buffer[uart2_rx_head] = byte;
			uart2_rx_head = next_head;
		}
	}
}


void uart2_rxtx_init(void){
	// CONFIGURE UART GPIO PIN
	//enable clock access to GPIOA
	RCC->AHB1ENR |= GPIOAEN;

	//set pa2 mode to alternate function mode
	GPIOA->MODER &=~ (1U<<4);
	GPIOA->MODER |= (1U<<5);

	// set pa2 alternate function type to UART_TX (AF07)
	GPIOA->AFR[0] |= (1U<<8);
	GPIOA->AFR[0] |= (1U<<9);
	GPIOA->AFR[0] |= (1U<<10);
	GPIOA->AFR[0] &=~ (1U<<11);

	// set pa3 mode to alt function mode
	GPIOA->MODER &=~ (1U<<6);
	GPIOA->MODER |= (1U<<7);

	// set pa3 alt function typt to UART_RX (AF07)
	GPIOA->AFR[0] |= (1U<<12);
	GPIOA->AFR[0] |= (1U<<13);
	GPIOA->AFR[0] |= (1U<<14);
	GPIOA->AFR[0] &=~ (1U<<15);

	//CONFIGURE UART MODULE
	//Enable clock access to UART2
	RCC->APB1ENR |= UART2EN;

	//configure baudrate
	uart_set_baudrate(USART2, APB1_CLK, UART_BAUDRATE);

	//configure the transfer direction
	USART2->CR1 = CR1_TE | CR1_RE;
	USART2->CR1 |= CR1_RXNEIE;

	//enable UART module
	USART2->CR1 |= CR1_UE;

	NVIC_EnableIRQ(USART2_IRQn);
}

char uart2_read(void){
	//make sure the recieve data register is not empty
	while(!(USART2->SR & SR_RXNE)){}

	// READ DATA
	return USART2->DR;
}

bool uart2_try_read(uint8_t *byte){
	if (byte == 0){
		return false;
	}

	if (uart2_rx_head == uart2_rx_tail){
		return false;
	}

	*byte = uart2_rx_buffer[uart2_rx_tail];
	uart2_rx_tail = (uint16_t)((uart2_rx_tail + 1U) % UART2_RX_BUFFER_SIZE);
	return true;
}

bool uart2_take_rx_overflow(void){
	bool overflowed = uart2_rx_overflow;
	uart2_rx_overflow = false;
	return overflowed;
}

void uart2_tx_init(void){
	// CONFIGURE UART GPIO PIN
	//enable clock access to GPIOA
	RCC->AHB1ENR |= GPIOAEN;

	//set pa2 mode to alternate function mode
	GPIOA->MODER &=~ (1U<<4);
	GPIOA->MODER |= (1U<<5);

	// set pa2 alternate function type to UART_TX (AF07)
	GPIOA->AFR[0] |= (1U<<8);
	GPIOA->AFR[0] |= (1U<<9);
	GPIOA->AFR[0] |= (1U<<10);
	GPIOA->AFR[0] &=~ (1U<<11);

	//CONFIGURE UART MODULE
	//Enable clock access to UART2
	RCC->APB1ENR |= UART2EN;

	//configure baudrate
	uart_set_baudrate(USART2, APB1_CLK, UART_BAUDRATE);

	//configure the transfer direction
	USART2->CR1 = CR1_TE;

	//enable UART module
	USART2->CR1 |= CR1_UE;
}

void uart2_write_buffer(const uint8_t *data, uint16_t length){
	uint16_t index;

	if (data == 0){
		return;
	}

	for (index = 0U; index < length; ++index){
		uart2_write(data[index]);
	}
}

static void uart2_write(int ch){
	//Make sure transmit data register is empty
	while(!(USART2->SR & SR_TXE)){}

	//Write to transmit data register
	USART2->DR = (ch & 0xFF);

}

static void uart_set_baudrate(USART_TypeDef *USARTx, uint32_t PeriphClk, uint32_t BaudRate){
	USARTx->BRR = compute_uart_bd(PeriphClk, BaudRate);
}

static uint16_t compute_uart_bd(uint32_t PeriphClk, uint32_t BaudRate){
	return ((PeriphClk + (BaudRate/2U))/ BaudRate);
}

