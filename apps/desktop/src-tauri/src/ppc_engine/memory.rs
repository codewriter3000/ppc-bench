//! Flat big-endian memory for the PPC engine.
//!
//! Programs are typically loaded at `BASE_ADDR` (GameCube cached-RAM
//! convention), but the same backing bytes are also visible through the low
//! physical RAM mirror and the uncached mirror. This matches the way Dolphin
//! treats MEM1 when address translation is disabled.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Default load base — matches Dolphin's MEM1 cached mirror.
pub const BASE_ADDR: u32 = 0x8000_0000;
/// Uncached MEM1 mirror.
pub const UNCACHED_BASE_ADDR: u32 = 0xC000_0000;
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
    fn aliased_offset(&self, addr: u32) -> Option<usize> {
        let ram_len = self.bytes.len() as u32;

        if addr < ram_len {
            return Some(addr as usize);
        }

        if let Some(off) = addr.checked_sub(self.base) {
            if off < ram_len {
                return Some(off as usize);
            }
        }

        if let Some(off) = addr.checked_sub(UNCACHED_BASE_ADDR) {
            if off < ram_len {
                return Some(off as usize);
            }
        }

        None
    }

    #[inline]
    fn offset(&self, addr: u32, size: u32) -> Result<usize, MemError> {
        let off = self
            .aliased_offset(addr)
            .ok_or(MemError::OutOfBounds { addr, len: size })?;
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

#[cfg(test)]
mod tests {
    use super::{Memory, BASE_ADDR, UNCACHED_BASE_ADDR};

    #[test]
    fn cached_ram_is_visible_through_low_physical_mirror() {
        let mut mem = Memory::new();
        let addr = 0x0002_C100;
        let value = 0x4C00_0064;

        mem.write_u32(BASE_ADDR + addr, value).unwrap();

        assert_eq!(mem.read_u32(addr).unwrap(), value);
    }

    #[test]
    fn low_physical_writes_update_cached_and_uncached_mirrors() {
        let mut mem = Memory::new();
        let addr = 0x0002_C100;
        let value = 0x3860_0001;

        mem.write_u32(addr, value).unwrap();

        assert_eq!(mem.read_u32(BASE_ADDR + addr).unwrap(), value);
        assert_eq!(mem.read_u32(UNCACHED_BASE_ADDR + addr).unwrap(), value);
    }
}
