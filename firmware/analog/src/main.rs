#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embassy_stm32 as stm32;

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let _p = stm32::init(Default::default());
    info!("LoadLynx analog alive");

    // TODO: clocks/adc/pwm init per board; placeholder heartbeat
    loop {
        Timer::after(Duration::from_millis(1000)).await;
        info!("tick");
    }
}
