//! Flat big-endian memory for the PPC engine.
//!
//! Programs are loaded at `BASE_ADDR` (GameCube cached-RAM convention).
//! Accesses below the base or past the buffer return [`MemError`].

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Default load base — matches Dolphin's MEM1 cached mirror.
pub const BASE_ADDR: u32 = 0x8000_0000;
/// Default RAM size: 16 MiB. Configurable later via settings.
pub const RAM_SIZE: usize = 16 * 1024 * 1024;

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum MemError {
    #[error("memory access at 0x{addr:08X} out of bounds (len {len})")]
    OutOfBounds { addr: u32, len: u32 },
    #[error("misaligned {size}-byte access at 0x{addr:08X}")]
    Misaligned { addr: u32, size: u32 },
}

/// Big-endian flat memory.
#[derive(Debug)]
pub struct Memory {
    pub base: u32,
    pub bytes: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Self { base: BASE_ADDR, bytes: vec![0; RAM_SIZE] }
    }

    #[inline]
    fn offset(&self, addr: u32, size: u32) -> Result<usize, MemError> {
        if addr < self.base {
            return Err(MemError::OutOfBounds { addr, len: size });
        }
        let off = (addr - self.base) as usize;
        if off
            .checked_add(size as usize)
            .map(|e| e > self.bytes.len())
            .unwrap_or(true)
        {
            return Err(MemError::OutOfBounds { addr, len: size });
        }
        Ok(off)
    }

    pub fn read_u8(&self, addr: u32) -> Result<u8, MemError> {
        let o = self.offset(addr, 1)?;
        Ok(self.bytes[o])
    }
    pub fn read_u16(&self, addr: u32) -> Result<u16, MemError> {
        let o = self.offset(addr, 2)?;
        Ok(u16::from_be_bytes([self.bytes[o], self.bytes[o + 1]]))
    }
    pub fn read_u32(&self, addr: u32) -> Result<u32, MemError> {
        let o = self.offset(addr, 4)?;
        Ok(u32::from_be_bytes([
            self.bytes[o], self.bytes[o + 1], self.bytes[o + 2], self.bytes[o + 3],
        ]))
    }
    pub fn read_u64(&self, addr: u32) -> Result<u64, MemError> {
        let o = self.offset(addr, 8)?;
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&self.bytes[o..o + 8]);
        Ok(u64::from_be_bytes(buf))
    }

    pub fn write_u8(&mut self, addr: u32, v: u8) -> Result<(), MemError> {
        let o = self.offset(addr, 1)?;
        self.bytes[o] = v;
        Ok(())
    }
    pub fn write_u16(&mut self, addr: u32, v: u16) -> Result<(), MemError> {
        let o = self.offset(addr, 2)?;
        self.bytes[o..o + 2].copy_from_slice(&v.to_be_bytes());
        Ok(())
    }
    pub fn write_u32(&mut self, addr: u32, v: u32) -> Result<(), MemError> {
        let o = self.offset(addr, 4)?;
        self.bytes[o..o + 4].copy_from_slice(&v.to_be_bytes());
        Ok(())
    }
    pub fn write_u64(&mut self, addr: u32, v: u64) -> Result<(), MemError> {
        let o = self.offset(addr, 8)?;
        self.bytes[o..o + 8].copy_from_slice(&v.to_be_bytes());
        Ok(())
    }

    /// Load a contiguous slice at `addr`. Panics on overflow; returns Err on OOB.
    pub fn write_bytes(&mut self, addr: u32, src: &[u8]) -> Result<(), MemError> {
        let o = self.offset(addr, src.len() as u32)?;
        self.bytes[o..o + src.len()].copy_from_slice(src);
        Ok(())
    }

    pub fn read_bytes(&self, addr: u32, len: u32) -> Result<&[u8], MemError> {
        let o = self.offset(addr, len)?;
        Ok(&self.bytes[o..o + len as usize])
    }

    pub fn clear(&mut self) {
        self.bytes.iter_mut().for_each(|b| *b = 0);
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}
