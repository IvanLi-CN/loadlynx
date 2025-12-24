use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use esp_hal::{Async, i2c::master::I2c};

/// Async I2C0 peripheral type (GPIO8=SDA, GPIO9=SCL @ 400kHz).
pub type I2c0 = I2c<'static, Async>;

/// Shared I2C0 bus mutex for short, serialized transactions.
pub type I2c0Bus = Mutex<CriticalSectionRawMutex, I2c0>;
