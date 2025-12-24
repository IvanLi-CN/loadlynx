use core::{
    ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use esp_hal::{Async, i2c::master::I2c};
use static_cell::StaticCell;

/// Async I2C0 peripheral type (GPIO8=SDA, GPIO9=SCL @ 400kHz).
pub type I2c0 = I2c<'static, Async>;

/// Shared I2C0 bus mutex for short, serialized transactions.
pub type I2c0Bus = Mutex<CriticalSectionRawMutex, I2c0>;

static I2C0_BUS_CELL: StaticCell<I2c0Bus> = StaticCell::new();
static I2C0_BUS_PTR: AtomicPtr<I2c0Bus> = AtomicPtr::new(ptr::null_mut());

/// Initialize the global shared I2C0 bus mutex.
///
/// Must be called exactly once during boot before `bus()` is used.
pub fn init(i2c0: I2c0) -> &'static I2c0Bus {
    let bus = I2C0_BUS_CELL.init(Mutex::new(i2c0));
    I2C0_BUS_PTR.store(bus as *const _ as *mut _, Ordering::Release);
    bus
}

/// Get the global shared I2C0 bus mutex.
///
/// Panics if `init()` was not called yet.
pub fn bus() -> &'static I2c0Bus {
    let ptr = I2C0_BUS_PTR.load(Ordering::Acquire);
    assert!(!ptr.is_null(), "I2C0 bus not initialized");
    // Safety: pointer is initialized exactly once in `init()` and then stays valid forever.
    unsafe { &*ptr }
}
