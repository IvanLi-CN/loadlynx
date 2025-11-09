#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_stm32 as stm32;
use embassy_stm32::bind_interrupts;
use embassy_stm32::usart::{Config as UartConfig, Uart};

bind_interrupts!(struct Irqs {
    USART3 => stm32::usart::InterruptHandler<stm32::peripherals::USART3>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let p = stm32::init(Default::default());

    info!("LoadLynx analog alive; UART3 echo mode");

    let mut uart_cfg = UartConfig::default();
    uart_cfg.baudrate = 115_200;

    let mut uart = Uart::new(
        p.USART3, p.PC11, p.PC10, Irqs, p.DMA1_CH1, p.DMA1_CH2, uart_cfg,
    )
    .unwrap();

    let mut byte = [0u8; 1];
    loop {
        if uart.read(&mut byte).await.is_ok() {
            let _ = uart.write(&byte).await;
        }
    }
}
