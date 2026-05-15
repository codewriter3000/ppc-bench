//! Minimal GameCube DOL writer for emulator launch.
//!
//! PPC-Bench only needs a single text section containing the currently loaded
//! program bytes. The remaining header fields stay zeroed until the bench grows
//! a richer homebrew bootstrap.

const DOL_HEADER_LEN: usize = 0x100;
const TEXT_OFFSET_TABLE: usize = 0x00;
const TEXT_ADDRESS_TABLE: usize = 0x48;
const TEXT_SIZE_TABLE: usize = 0x90;
const ENTRY_POINT_OFFSET: usize = 0xE0;
const FIRST_SECTION_FILE_OFFSET: u32 = DOL_HEADER_LEN as u32;

#[inline]
fn write_be32(buf: &mut [u8], offset: usize, value: u32) {
    buf[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

pub fn generate_dol(program: &[u8], load_addr: u32, entry_point: u32) -> Vec<u8> {
    let mut dol = vec![0u8; DOL_HEADER_LEN + program.len()];

    write_be32(&mut dol, TEXT_OFFSET_TABLE, FIRST_SECTION_FILE_OFFSET);
    write_be32(&mut dol, TEXT_ADDRESS_TABLE, load_addr);
    write_be32(&mut dol, TEXT_SIZE_TABLE, program.len() as u32);
    write_be32(&mut dol, ENTRY_POINT_OFFSET, entry_point);

    dol[DOL_HEADER_LEN..].copy_from_slice(program);
    dol
}

#[cfg(test)]
mod tests {
    use super::{generate_dol, DOL_HEADER_LEN};

    #[test]
    fn writes_single_text_section_header() {
        let program = [0x60, 0x00, 0x00, 0x00, 0x4E, 0x80, 0x00, 0x20];
        let dol = generate_dol(&program, 0x8000_0000, 0x8000_0000);

        assert_eq!(dol.len(), DOL_HEADER_LEN + program.len());
        assert_eq!(&dol[0x00..0x04], &0x100_u32.to_be_bytes());
        assert_eq!(&dol[0x48..0x4C], &0x8000_0000_u32.to_be_bytes());
        assert_eq!(&dol[0x90..0x94], &(program.len() as u32).to_be_bytes());
        assert_eq!(&dol[0xE0..0xE4], &0x8000_0000_u32.to_be_bytes());
        assert_eq!(&dol[DOL_HEADER_LEN..], &program);
    }
}
