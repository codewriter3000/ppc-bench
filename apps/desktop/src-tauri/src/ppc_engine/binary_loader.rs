//! Parsers for file-backed PPC binaries.

use super::memory::{BASE_ADDR, RAM_SIZE};

const DOL_HEADER_LEN: usize = 0x100;
const DOL_TEXT_COUNT: usize = 7;
const DOL_DATA_COUNT: usize = 11;

const ELF_HEADER_LEN: usize = 0x34;
const ELF_PROGRAM_HEADER_LEN: usize = 0x20;
const ELF_CLASS_32: u8 = 1;
const ELF_DATA_BIG_ENDIAN: u8 = 2;
const ELF_MACHINE_PPC: u16 = 0x14;
const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
const ELF_PT_LOAD: u32 = 1;
const ELF_PF_X: u32 = 0x1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedSection {
    pub name: String,
    pub load_addr: u32,
    pub bytes: Vec<u8>,
    pub is_executable: bool,
    pub disasm_len: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedBinary {
    pub sections: Vec<LoadedSection>,
    pub entry_point: u32,
    pub format: String,
    pub program_end: u32,
}

pub fn load_binary(bytes: &[u8]) -> Result<LoadedBinary, String> {
    if bytes.starts_with(&ELF_MAGIC) {
        return parse_elf(bytes);
    }

    parse_dol(bytes).map_err(|err| format!("unsupported or invalid binary format: {err}"))
}

fn parse_dol(bytes: &[u8]) -> Result<LoadedBinary, String> {
    if bytes.len() < DOL_HEADER_LEN {
        return Err("DOL header is smaller than 0x100 bytes".to_string());
    }

    let mut sections = Vec::new();

    for index in 0..DOL_TEXT_COUNT {
        let offset = read_be_u32(bytes, 0x00 + index * 4)?;
        let load_addr = read_be_u32(bytes, 0x48 + index * 4)?;
        let size = read_be_u32(bytes, 0x90 + index * 4)?;
        if offset == 0 || size == 0 {
            continue;
        }

        validate_memory_range(load_addr, size, &format!("DOL text section {index}"))?;
        let data = checked_slice(bytes, offset as usize, size as usize, &format!("DOL text section {index}"))?;
        sections.push(LoadedSection {
            name: format!(".text{index}"),
            load_addr,
            bytes: data.to_vec(),
            is_executable: true,
            disasm_len: size,
        });
    }

    for index in 0..DOL_DATA_COUNT {
        let offset = read_be_u32(bytes, 0x1C + index * 4)?;
        let load_addr = read_be_u32(bytes, 0x64 + index * 4)?;
        let size = read_be_u32(bytes, 0xAC + index * 4)?;
        if offset == 0 || size == 0 {
            continue;
        }

        validate_memory_range(load_addr, size, &format!("DOL data section {index}"))?;
        let data = checked_slice(bytes, offset as usize, size as usize, &format!("DOL data section {index}"))?;
        sections.push(LoadedSection {
            name: format!(".data{index}"),
            load_addr,
            bytes: data.to_vec(),
            is_executable: false,
            disasm_len: 0,
        });
    }

    let bss_addr = read_be_u32(bytes, 0xD8)?;
    let bss_size = read_be_u32(bytes, 0xDC)?;
    if bss_addr != 0 && bss_size != 0 {
        validate_memory_range(bss_addr, bss_size, "DOL BSS")?;
        sections.push(LoadedSection {
            name: ".bss".to_string(),
            load_addr: bss_addr,
            bytes: vec![0; bss_size as usize],
            is_executable: false,
            disasm_len: 0,
        });
    }

    if sections.is_empty() {
        return Err("DOL file does not contain any loadable sections".to_string());
    }

    let entry_point = read_be_u32(bytes, 0xE0)?;
    validate_memory_range(entry_point, 1, "DOL entry point")?;
    let program_end = compute_program_end(&sections, entry_point)?;

    Ok(LoadedBinary {
        sections,
        entry_point,
        format: "DOL".to_string(),
        program_end,
    })
}

fn parse_elf(bytes: &[u8]) -> Result<LoadedBinary, String> {
    if bytes.len() < ELF_HEADER_LEN {
        return Err("ELF header is truncated".to_string());
    }
    if bytes[4] != ELF_CLASS_32 {
        return Err("only ELF32 binaries are supported".to_string());
    }
    if bytes[5] != ELF_DATA_BIG_ENDIAN {
        return Err("only big-endian ELF binaries are supported".to_string());
    }

    let machine = read_be_u16(bytes, 0x12)?;
    if machine != ELF_MACHINE_PPC {
        return Err(format!("unsupported ELF machine 0x{machine:04X}; expected PowerPC"));
    }

    let entry_point = read_be_u32(bytes, 0x18)?;
    let program_header_offset = read_be_u32(bytes, 0x1C)? as usize;
    let program_header_size = read_be_u16(bytes, 0x2A)? as usize;
    let program_header_count = read_be_u16(bytes, 0x2C)? as usize;

    if program_header_count == 0 {
        return Err("ELF file does not contain any program headers".to_string());
    }
    if program_header_size < ELF_PROGRAM_HEADER_LEN {
        return Err(format!(
            "ELF program headers are {program_header_size} bytes; expected at least {ELF_PROGRAM_HEADER_LEN}"
        ));
    }

    validate_memory_range(entry_point, 1, "ELF entry point")?;

    let mut sections = Vec::new();
    for index in 0..program_header_count {
        let header_offset = program_header_offset
            .checked_add(index.saturating_mul(program_header_size))
            .ok_or_else(|| "ELF program header offset overflowed".to_string())?;
        checked_slice(bytes, header_offset, program_header_size, &format!("ELF program header {index}"))?;

        let segment_type = read_be_u32(bytes, header_offset)?;
        if segment_type != ELF_PT_LOAD {
            continue;
        }

        let file_offset = read_be_u32(bytes, header_offset + 4)? as usize;
        let load_addr = read_be_u32(bytes, header_offset + 8)?;
        let file_size = read_be_u32(bytes, header_offset + 16)?;
        let memory_size = read_be_u32(bytes, header_offset + 20)?;
        let flags = read_be_u32(bytes, header_offset + 24)?;

        if memory_size == 0 {
            continue;
        }
        if file_size > memory_size {
            return Err(format!(
                "ELF PT_LOAD segment {index} has file size {file_size} larger than memory size {memory_size}"
            ));
        }

        validate_memory_range(load_addr, memory_size, &format!("ELF PT_LOAD segment {index}"))?;
        let file_bytes = checked_slice(bytes, file_offset, file_size as usize, &format!("ELF PT_LOAD segment {index}"))?;

        let mut segment_bytes = vec![0; memory_size as usize];
        segment_bytes[..file_size as usize].copy_from_slice(file_bytes);

        sections.push(LoadedSection {
            name: if flags & ELF_PF_X != 0 {
                format!(".text{index}")
            } else {
                format!(".data{index}")
            },
            load_addr,
            bytes: segment_bytes,
            is_executable: flags & ELF_PF_X != 0,
            disasm_len: file_size,
        });
    }

    if sections.is_empty() {
        return Err("ELF file does not contain any PT_LOAD segments".to_string());
    }

    let program_end = compute_program_end(&sections, entry_point)?;

    Ok(LoadedBinary {
        sections,
        entry_point,
        format: "ELF".to_string(),
        program_end,
    })
}

fn compute_program_end(sections: &[LoadedSection], entry_point: u32) -> Result<u32, String> {
    let mut highest_executable = entry_point;
    let mut highest_any = entry_point;
    let mut saw_executable = false;

    for section in sections {
        let end = section
            .load_addr
            .checked_add(section.bytes.len() as u32)
            .ok_or_else(|| format!("section {} end address overflowed", section.name))?;
        highest_any = highest_any.max(end);
        if section.is_executable {
            highest_executable = highest_executable.max(end);
            saw_executable = true;
        }
    }

    Ok(if saw_executable { highest_executable } else { highest_any })
}

fn validate_memory_range(addr: u32, len: u32, label: &str) -> Result<(), String> {
    if len == 0 {
        return Ok(());
    }

    let ram_end = BASE_ADDR
        .checked_add(RAM_SIZE as u32)
        .ok_or_else(|| "RAM size overflowed".to_string())?;
    let end = addr
        .checked_add(len)
        .ok_or_else(|| format!("{label} end address overflowed"))?;

    if addr < BASE_ADDR || end > ram_end {
        return Err(format!(
            "{label} at 0x{addr:08X} (len {len}) falls outside emulator RAM"
        ));
    }

    Ok(())
}

fn read_be_u16(bytes: &[u8], offset: usize) -> Result<u16, String> {
    let slice = checked_slice(bytes, offset, 2, &format!("u16 @ 0x{offset:X}"))?;
    Ok(u16::from_be_bytes([slice[0], slice[1]]))
}

fn read_be_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let slice = checked_slice(bytes, offset, 4, &format!("u32 @ 0x{offset:X}"))?;
    Ok(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn checked_slice<'a>(bytes: &'a [u8], offset: usize, len: usize, label: &str) -> Result<&'a [u8], String> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| format!("{label} range overflowed"))?;
    bytes
        .get(offset..end)
        .ok_or_else(|| format!("{label} extends past end of file"))
}

#[cfg(test)]
mod tests {
    use super::{load_binary, ELF_MAGIC};
    use crate::ppc_engine::dol::generate_dol;
    use crate::ppc_engine::memory::BASE_ADDR;

    fn write_be16(buf: &mut [u8], offset: usize, value: u16) {
        buf[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }

    fn write_be32(buf: &mut [u8], offset: usize, value: u32) {
        buf[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    #[test]
    fn parses_generated_dol() {
        let program = [0x60, 0x00, 0x00, 0x00, 0x4E, 0x80, 0x00, 0x20];
        let dol = generate_dol(&program, BASE_ADDR, BASE_ADDR);

        let loaded = load_binary(&dol).expect("DOL should parse");

        assert_eq!(loaded.format, "DOL");
        assert_eq!(loaded.entry_point, BASE_ADDR);
        assert_eq!(loaded.program_end, BASE_ADDR + program.len() as u32);
        assert_eq!(loaded.sections.len(), 1);
        assert_eq!(loaded.sections[0].name, ".text0");
        assert_eq!(loaded.sections[0].load_addr, BASE_ADDR);
        assert!(loaded.sections[0].is_executable);
        assert_eq!(loaded.sections[0].disasm_len, program.len() as u32);
        assert_eq!(loaded.sections[0].bytes, program);
    }

    #[test]
    fn parses_elf32_big_endian_load_segment() {
        let entry = BASE_ADDR + 0x3100;
        let file_offset = 0x80usize;
        let file_size = 8u32;
        let memory_size = 16u32;
        let program = [0x60, 0x00, 0x00, 0x00, 0x4E, 0x80, 0x00, 0x20];

        let mut elf = vec![0u8; file_offset + file_size as usize];
        elf[0..4].copy_from_slice(&ELF_MAGIC);
        elf[4] = 1;
        elf[5] = 2;
        elf[6] = 1;

        write_be16(&mut elf, 0x10, 2);
        write_be16(&mut elf, 0x12, 0x14);
        write_be32(&mut elf, 0x14, 1);
        write_be32(&mut elf, 0x18, entry);
        write_be32(&mut elf, 0x1C, 0x34);
        write_be32(&mut elf, 0x20, 0);
        write_be32(&mut elf, 0x24, 0);
        write_be16(&mut elf, 0x28, 0x34);
        write_be16(&mut elf, 0x2A, 0x20);
        write_be16(&mut elf, 0x2C, 1);
        write_be16(&mut elf, 0x2E, 0);
        write_be16(&mut elf, 0x30, 0);
        write_be16(&mut elf, 0x32, 0);

        write_be32(&mut elf, 0x34, 1);
        write_be32(&mut elf, 0x38, file_offset as u32);
        write_be32(&mut elf, 0x3C, entry);
        write_be32(&mut elf, 0x40, entry);
        write_be32(&mut elf, 0x44, file_size);
        write_be32(&mut elf, 0x48, memory_size);
        write_be32(&mut elf, 0x4C, 0x5);
        write_be32(&mut elf, 0x50, 0x20);

        elf[file_offset..file_offset + program.len()].copy_from_slice(&program);

        let loaded = load_binary(&elf).expect("ELF should parse");

        assert_eq!(loaded.format, "ELF");
        assert_eq!(loaded.entry_point, entry);
        assert_eq!(loaded.program_end, entry + memory_size);
        assert_eq!(loaded.sections.len(), 1);
        assert_eq!(loaded.sections[0].name, ".text0");
        assert_eq!(loaded.sections[0].load_addr, entry);
        assert!(loaded.sections[0].is_executable);
        assert_eq!(loaded.sections[0].disasm_len, file_size);
        assert_eq!(&loaded.sections[0].bytes[..program.len()], &program);
        assert_eq!(&loaded.sections[0].bytes[program.len()..], &[0; 8]);
    }

    #[test]
    fn rejects_unknown_bytes() {
        let err = load_binary(b"not a dol or elf").expect_err("random bytes should fail");
        assert!(err.contains("unsupported or invalid binary format"));
    }
}