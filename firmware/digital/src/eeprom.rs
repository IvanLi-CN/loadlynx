use embassy_futures::yield_now;
use embedded_hal_async::i2c::I2c as EhI2c;
use heapless::Vec;

use loadlynx_calibration_format::{
    EEPROM_I2C_ADDR_7BIT, EEPROM_PAGE_SIZE_BYTES, EEPROM_PROFILE_BASE_ADDR, EEPROM_PROFILE_LEN,
};

use crate::i2c0::I2c0Bus;

// Calibration blob is fixed at base=0x0000 len=EEPROM_PROFILE_LEN (v3=1024).
// Presets are stored in the next non-overlapping region.
pub const EEPROM_PRESETS_BASE_ADDR: u16 = EEPROM_PROFILE_BASE_ADDR + (EEPROM_PROFILE_LEN as u16);
pub const EEPROM_PRESETS_LEN: usize = 256;

#[derive(Debug, Clone, Copy, defmt::Format)]
pub enum EepromError {
    I2c,
    Timeout,
    InvalidLength,
}

/// Minimal M24C64 (64 Kbit) EEPROM driver over embedded-hal-async I2C.
///
/// - 7-bit address: 0x50 (A0/A1/A2 strapped to GND).
/// - 16-bit word address.
/// - Page-write: we never write across `EEPROM_PAGE_SIZE_BYTES` boundaries.
#[derive(Clone, Copy)]
pub struct M24c64 {
    addr_7bit: u8,
}

impl M24c64 {
    pub const fn new(addr_7bit: u8) -> Self {
        Self { addr_7bit }
    }

    pub async fn read<I2C: EhI2c>(
        &self,
        i2c: &mut I2C,
        addr: u16,
        out: &mut [u8],
    ) -> Result<(), EepromError> {
        let a = addr.to_be_bytes();
        i2c.write_read(self.addr_7bit, &a, out)
            .await
            .map_err(|_| EepromError::I2c)
    }

    pub async fn write_chunk<I2C: EhI2c>(
        &self,
        i2c: &mut I2C,
        addr: u16,
        data: &[u8],
    ) -> Result<(), EepromError> {
        let mem_addr = addr.to_be_bytes();
        let mut buf: Vec<u8, { 2 + EEPROM_PAGE_SIZE_BYTES }> = Vec::new();
        let _ = buf.extend_from_slice(&mem_addr);
        let _ = buf.extend_from_slice(data);
        i2c.write(self.addr_7bit, &buf)
            .await
            .map_err(|_| EepromError::I2c)
    }

    pub async fn probe_ready<I2C: EhI2c>(
        &self,
        i2c: &mut I2C,
        probe_addr: u16,
        dummy: &mut [u8; 1],
    ) -> Result<(), EepromError> {
        let probe = probe_addr.to_be_bytes();
        i2c.write_read(self.addr_7bit, &probe, dummy)
            .await
            .map_err(|_| EepromError::I2c)
    }
}

/// EEPROM instance wired onto the shared I2C0 bus.
///
/// This wrapper intentionally locks the bus only for a single short transaction
/// at a time (page write, read, ready-probe), so other devices (e.g. touch) can
/// share I2C0 without starvation.
pub struct SharedM24c64 {
    bus: &'static I2c0Bus,
    dev: M24c64,
}

impl SharedM24c64 {
    pub fn new(bus: &'static I2c0Bus) -> Self {
        Self {
            bus,
            dev: M24c64::new(EEPROM_I2C_ADDR_7BIT),
        }
    }

    pub async fn read(&mut self, addr: u16, out: &mut [u8]) -> Result<(), EepromError> {
        let dev = self.dev;
        let mut guard = self.bus.lock().await;
        dev.read(&mut *guard, addr, out).await
    }

    pub async fn write(&mut self, addr: u16, data: &[u8]) -> Result<(), EepromError> {
        if data.is_empty() {
            return Ok(());
        }

        let mut cur_addr = addr as usize;
        let mut offset = 0usize;

        while offset < data.len() {
            let page_off = cur_addr % EEPROM_PAGE_SIZE_BYTES;
            let page_rem = EEPROM_PAGE_SIZE_BYTES - page_off;
            // Be conservative: write at most 16 bytes per cycle even if the
            // device supports 32-byte page writes.
            let chunk_len = (data.len() - offset).min(page_rem).min(16);

            let chunk = &data[offset..offset + chunk_len];
            let dev = self.dev;
            {
                let mut guard = self.bus.lock().await;
                dev.write_chunk(&mut *guard, cur_addr as u16, chunk).await?;
            }

            // Wait for internal write cycle completion via polling.
            self.wait_ready(cur_addr as u16).await?;

            cur_addr += chunk_len;
            offset += chunk_len;
        }
        Ok(())
    }

    pub async fn write_profile_blob(
        &mut self,
        blob: &[u8; EEPROM_PROFILE_LEN],
    ) -> Result<(), EepromError> {
        self.write(EEPROM_PROFILE_BASE_ADDR, blob).await
    }

    pub async fn read_profile_blob(&mut self) -> Result<[u8; EEPROM_PROFILE_LEN], EepromError> {
        let mut buf = [0u8; EEPROM_PROFILE_LEN];
        self.read(EEPROM_PROFILE_BASE_ADDR, &mut buf).await?;
        Ok(buf)
    }

    pub async fn clear_profile_blob(&mut self) -> Result<(), EepromError> {
        // Invalidate by filling with 0xFF; CRC will fail and we will fall back
        // to firmware factory defaults on next boot.
        let buf = [0xFFu8; EEPROM_PROFILE_LEN];
        self.write_profile_blob(&buf).await
    }

    pub async fn write_presets_blob(
        &mut self,
        blob: &[u8; EEPROM_PRESETS_LEN],
    ) -> Result<(), EepromError> {
        self.write(EEPROM_PRESETS_BASE_ADDR, blob).await
    }

    pub async fn read_presets_blob(&mut self) -> Result<[u8; EEPROM_PRESETS_LEN], EepromError> {
        let mut buf = [0u8; EEPROM_PRESETS_LEN];
        self.read(EEPROM_PRESETS_BASE_ADDR, &mut buf).await?;
        Ok(buf)
    }

    pub async fn clear_presets_blob(&mut self) -> Result<(), EepromError> {
        // Invalidate by filling with 0xFF; CRC will fail and we will fall back
        // to firmware defaults on next boot.
        let buf = [0xFFu8; EEPROM_PRESETS_LEN];
        self.write_presets_blob(&buf).await
    }

    async fn wait_ready(&mut self, probe_addr: u16) -> Result<(), EepromError> {
        // Typical tWR is a few ms; keep a generous timeout.
        const POLL_TIMEOUT_MS: u32 = 20;
        let start = crate::now_ms32();
        let mut dummy = [0u8; 1];
        loop {
            let dev = self.dev;
            let res = {
                let mut guard = self.bus.lock().await;
                dev.probe_ready(&mut *guard, probe_addr, &mut dummy).await
            };

            match res {
                Ok(()) => return Ok(()),
                Err(_) => {
                    if crate::now_ms32().wrapping_sub(start) >= POLL_TIMEOUT_MS {
                        return Err(EepromError::Timeout);
                    }
                    // Cooperative short delay.
                    for _ in 0..200 {
                        yield_now().await;
                    }
                }
            }
        }
    }
}
