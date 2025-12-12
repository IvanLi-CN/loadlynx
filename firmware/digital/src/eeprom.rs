use embassy_futures::yield_now;
use esp_hal::Async;
use esp_hal::i2c::master::I2c;
use heapless::Vec;

use loadlynx_calibration_format::{
    EEPROM_I2C_ADDR_7BIT, EEPROM_PAGE_SIZE_BYTES, EEPROM_PROFILE_BASE_ADDR, EEPROM_PROFILE_LEN,
};

#[derive(Debug, Clone, Copy, defmt::Format)]
pub enum EepromError {
    I2c,
    Timeout,
    InvalidLength,
}

/// Minimal M24C64 (64 Kbit) EEPROM driver over esp-hal async I2C.
///
/// - 7-bit address: 0x50 (A0/A1/A2 strapped to GND).
/// - 16-bit word address.
/// - Page-write: we never write across `EEPROM_PAGE_SIZE_BYTES` boundaries.
pub struct M24c64 {
    i2c: I2c<'static, Async>,
    addr_7bit: u8,
}

impl M24c64 {
    pub fn new(i2c: I2c<'static, Async>) -> Self {
        Self {
            i2c,
            addr_7bit: EEPROM_I2C_ADDR_7BIT,
        }
    }

    pub fn into_inner(self) -> I2c<'static, Async> {
        self.i2c
    }

    pub async fn read(&mut self, addr: u16, out: &mut [u8]) -> Result<(), EepromError> {
        let a = addr.to_be_bytes();
        self.i2c
            .write_read_async(self.addr_7bit, &a, out)
            .await
            .map_err(|_| EepromError::I2c)
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

            let mem_addr = (cur_addr as u16).to_be_bytes();
            let mut buf: Vec<u8, { 2 + EEPROM_PAGE_SIZE_BYTES }> = Vec::new();
            let _ = buf.extend_from_slice(&mem_addr);
            let _ = buf.extend_from_slice(&data[offset..offset + chunk_len]);

            self.i2c
                .write_async(self.addr_7bit, &buf)
                .await
                .map_err(|_| EepromError::I2c)?;

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

    async fn wait_ready(&mut self, probe_addr: u16) -> Result<(), EepromError> {
        // Typical tWR is a few ms; keep a generous timeout.
        const POLL_TIMEOUT_MS: u32 = 20;
        let start = crate::now_ms32();
        let mut dummy = [0u8; 1];
        let probe = probe_addr.to_be_bytes();
        loop {
            match self
                .i2c
                .write_read_async(self.addr_7bit, &probe, &mut dummy)
                .await
            {
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
