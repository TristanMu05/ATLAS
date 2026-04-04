#include "stm32f4xx.h"
#include <stdint.h>
#include <stdio.h>
#include "uart.h"

#define GPIOAEN (1U<<0)
#define GPIOA_5 (1u<<5)

#define LED_PIN (GPIOA_5)

char key;

int main(void)
{
	//enable clock access to gpioa
	RCC->AHB1ENR |= GPIOAEN;

	//set pa5 as output pin
	GPIOA->MODER |= (1U<<10);
	GPIOA->MODER &=~ (1U<<11);

	//initialize read and write for uart
	uart2_rxtx_init();

	while(1)
	{
		key = uart2_read();
		if (key == '1'){
			GPIOA->ODR |= LED_PIN;
			printf("Hello WORLD... \n\r");
		}else{
			GPIOA->ODR &=~ LED_PIN;
		}

	}
}
