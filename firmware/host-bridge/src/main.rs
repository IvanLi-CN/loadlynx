#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Executor;
use embassy_futures::yield_now;
use esp_backtrace as _; // panic handler + backtrace (defmt)
use esp_hal::{self as hal, main};
use static_cell::StaticCell;

// 简单异步任务（未启用时间驱动，使用合作式让出）
#[embassy_executor::task]
async fn ticker() {
    loop {
        info!("LoadLynx host-bridge tick");
        for _ in 0..1000 {
            yield_now().await;
        }
    }
}

static EXECUTOR: StaticCell<Executor> = StaticCell::new();

#[main]
fn main() -> ! {
    // 基本 HAL 初始化（默认时钟），根据板卡补充外设初始化
    let _peripherals = hal::init(hal::Config::default());

    info!("LoadLynx host-bridge alive");

    // 如需 Embassy 定时器（Timer::after），可通过 esp-hal-embassy 绑定底层计时器：
    // let timg0 = hal::timer::timg::TimerGroup::new(_peripherals.TIMG0);
    // esp_hal_embassy::init(timg0);

    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(ticker()).ok();
    })
}
