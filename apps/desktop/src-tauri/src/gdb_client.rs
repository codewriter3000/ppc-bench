//! Minimal GDB Remote Serial Protocol client for Dolphin integration.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::thread::sleep;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct GdbRegisterState {
    pub gpr: [u32; 32],
    pub fpr: Vec<[f64; 2]>,
    pub pc: u32,
    pub msr: u32,
    pub cr: u32,
    pub lr: u32,
    pub ctr: u32,
    pub xer: u32,
    pub fpscr: u32,
}

#[derive(Debug, Clone)]
pub struct StopSignal {
    pub signal: u8,
    pub pc: Option<u32>,
    pub sp: Option<u32>,
    pub watchpoint: Option<StopWatchpoint>,
    pub exception_code: Option<String>,
    pub raw: String,
}

#[derive(Debug, Clone)]
pub struct StopWatchpoint {
    pub kind: StopWatchpointKind,
    pub address: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum StopWatchpointKind {
    Write,
    Read,
    Access,
}

#[derive(Debug, Clone)]
pub enum StopPacket {
    Signal(StopSignal),
    Exit(u8),
    Reply(String),
}

pub struct GdbClient {
    stream: TcpStream,
}

impl GdbClient {
    pub fn connect(port: u16, timeout: Duration) -> Result<Self, String> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let start = Instant::now();

        loop {
            match TcpStream::connect(addr) {
                Ok(stream) => {
                    stream
                        .set_nodelay(true)
                        .map_err(|err| format!("failed to enable TCP_NODELAY: {err}"))?;
                    return Ok(Self { stream });
                }
                Err(_err) if start.elapsed() < timeout => sleep(Duration::from_millis(100)),
                Err(err) => {
                    return Err(format!(
                        "failed to connect to Dolphin GDB stub at 127.0.0.1:{port}: {err}"
                    ));
                }
            }
        }
    }

    pub fn interrupt_clone(&self) -> Result<TcpStream, String> {
        self.stream
            .try_clone()
            .map_err(|err| format!("failed to clone GDB socket: {err}"))
    }

    pub fn query_stop_reason(&mut self) -> Result<StopPacket, String> {
        Ok(parse_stop_packet(&self.request("?")?))
    }

    pub fn read_registers(&mut self) -> Result<GdbRegisterState, String> {
        parse_registers(&self.request("g")?)
    }

    pub fn read_memory(&mut self, address: u32, length: u32) -> Result<Vec<u8>, String> {
        decode_hex_bytes(&self.request(&format!("m{address:08X},{length:X}"))?)
    }

    pub fn set_breakpoint(&mut self, address: u32) -> Result<(), String> {
        self.expect_ok(&format!("Z0,{address:08X},4"))
    }

    pub fn clear_breakpoint(&mut self, address: u32) -> Result<(), String> {
        self.expect_ok(&format!("z0,{address:08X},4"))
    }

    pub fn send_continue(&mut self) -> Result<(), String> {
        self.write_packet("c")?;
        self.read_ack()
    }

    pub fn step(&mut self) -> Result<StopPacket, String> {
        self.write_packet("s")?;
        self.read_ack()?;
        self.wait_for_stop()
    }

    pub fn wait_for_stop(&mut self) -> Result<StopPacket, String> {
        Ok(parse_stop_packet(&self.read_packet()?))
    }

    fn expect_ok(&mut self, payload: &str) -> Result<(), String> {
        let reply = self.request(payload)?;
        if reply == "OK" {
            return Ok(());
        }

        Err(format!("unexpected GDB reply: {reply}"))
    }

    fn request(&mut self, payload: &str) -> Result<String, String> {
        self.write_packet(payload)?;
        let reply = self.read_reply()?;
        if reply.starts_with('E') {
            return Err(format!("GDB error reply: {reply}"));
        }
        Ok(reply)
    }

    fn write_packet(&mut self, payload: &str) -> Result<(), String> {
        let checksum = payload
            .as_bytes()
            .iter()
            .fold(0u8, |sum, byte| sum.wrapping_add(*byte));
        let packet = format!("${payload}#{checksum:02X}");
        self.stream
            .write_all(packet.as_bytes())
            .and_then(|_| self.stream.flush())
            .map_err(|err| format!("failed to write GDB packet: {err}"))
    }

    fn read_ack(&mut self) -> Result<(), String> {
        loop {
            match self.read_byte()? {
                b'+' => return Ok(()),
                b'-' => return Err("GDB rejected packet checksum".to_string()),
                _ => continue,
            }
        }
    }

    fn read_reply(&mut self) -> Result<String, String> {
        loop {
            match self.read_byte()? {
                b'+' => return self.read_packet(),
                b'$' => return self.read_packet_body(),
                b'-' => return Err("GDB rejected packet checksum".to_string()),
                _ => continue,
            }
        }
    }

    fn read_packet(&mut self) -> Result<String, String> {
        loop {
            match self.read_byte()? {
                b'$' => return self.read_packet_body(),
                b'+' => continue,
                _ => continue,
            }
        }
    }

    fn read_packet_body(&mut self) -> Result<String, String> {
        let mut payload = Vec::new();
        loop {
            let byte = self.read_byte()?;
            if byte == b'#' {
                break;
            }
            payload.push(byte);
        }

        let hi = self.read_byte()?;
        let lo = self.read_byte()?;
        let checksum = parse_hex_u8(&String::from_utf8(vec![hi, lo]).map_err(|err| err.to_string())?)?;
        let calc = payload.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte));
        if checksum != calc {
            return Err(format!(
                "invalid GDB checksum: expected {calc:02X}, received {checksum:02X}"
            ));
        }

        self.stream
            .write_all(b"+")
            .and_then(|_| self.stream.flush())
            .map_err(|err| format!("failed to ACK GDB reply: {err}"))?;

        String::from_utf8(payload).map_err(|err| format!("invalid UTF-8 in GDB reply: {err}"))
    }

    fn read_byte(&mut self) -> Result<u8, String> {
        let mut byte = [0u8; 1];
        self.stream
            .read_exact(&mut byte)
            .map_err(|err| format!("failed to read from GDB socket: {err}"))?;
        Ok(byte[0])
    }
}

pub fn send_interrupt(stream: &mut TcpStream) -> Result<(), String> {
    stream
        .write_all(&[0x03])
        .and_then(|_| stream.flush())
        .map_err(|err| format!("failed to send GDB interrupt: {err}"))
}

fn parse_registers(payload: &str) -> Result<GdbRegisterState, String> {
    let mut offset = 0usize;
    let mut gpr = [0u32; 32];
    for reg in &mut gpr {
        *reg = parse_u32(take_hex(payload, &mut offset, 8)?)?;
    }

    let mut fpr = Vec::with_capacity(32);
    for _ in 0..32 {
        let bits = parse_u64(take_hex(payload, &mut offset, 16)?)?;
        fpr.push([f64::from_bits(bits), 0.0]);
    }

    let pc = parse_u32(take_hex(payload, &mut offset, 8)?)?;
    let msr = parse_u32(take_hex(payload, &mut offset, 8)?)?;
    let cr = parse_u32(take_hex(payload, &mut offset, 8)?)?;
    let lr = parse_u32(take_hex(payload, &mut offset, 8)?)?;
    let ctr = parse_u32(take_hex(payload, &mut offset, 8)?)?;
    let xer = parse_u32(take_hex(payload, &mut offset, 8)?)?;
    let fpscr = parse_u32(take_hex(payload, &mut offset, 8)?)?;

    Ok(GdbRegisterState {
        gpr,
        fpr,
        pc,
        msr,
        cr,
        lr,
        ctr,
        xer,
        fpscr,
    })
}

fn decode_hex_bytes(payload: &str) -> Result<Vec<u8>, String> {
    if payload.len() % 2 != 0 {
        return Err("odd-length hex payload from GDB".to_string());
    }

    payload
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let hex = std::str::from_utf8(pair).map_err(|err| err.to_string())?;
            parse_hex_u8(hex)
        })
        .collect()
}

fn take_hex<'a>(payload: &'a str, offset: &mut usize, len: usize) -> Result<&'a str, String> {
    if *offset + len > payload.len() {
        return Err("truncated GDB register payload".to_string());
    }

    let slice = &payload[*offset..*offset + len];
    *offset += len;
    Ok(slice)
}

fn parse_u32(hex: &str) -> Result<u32, String> {
    u32::from_str_radix(hex, 16).map_err(|err| format!("invalid u32 hex {hex}: {err}"))
}

fn parse_u64(hex: &str) -> Result<u64, String> {
    u64::from_str_radix(hex, 16).map_err(|err| format!("invalid u64 hex {hex}: {err}"))
}

fn parse_hex_u8(hex: &str) -> Result<u8, String> {
    u8::from_str_radix(hex, 16).map_err(|err| format!("invalid u8 hex {hex}: {err}"))
}

fn parse_stop_u32(value: &str) -> Result<u32, String> {
    let trimmed = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    u32::from_str_radix(trimmed, 16)
        .or_else(|_| trimmed.parse::<u32>())
        .map_err(|err| format!("invalid stop-value u32 {value}: {err}"))
}

fn parse_stop_packet(payload: &str) -> StopPacket {
    match payload.as_bytes().first().copied() {
        Some(b'T') | Some(b'S') if payload.len() >= 3 => {
            let signal = parse_hex_u8(&payload[1..3]).unwrap_or(0);
            let mut pc = None;
            let mut sp = None;
            let mut watchpoint = None;
            let mut exception_code = None;

            if payload.as_bytes().first() == Some(&b'T') && payload.len() > 3 {
                for field in payload[3..].split(';').filter(|field| !field.is_empty()) {
                    let Some((key, value)) = field.split_once(':') else {
                        continue;
                    };
                    match key {
                        "watch" | "rwatch" | "awatch" => {
                            let Ok(address) = parse_stop_u32(value) else {
                                continue;
                            };
                            let kind = match key {
                                "watch" => StopWatchpointKind::Write,
                                "rwatch" => StopWatchpointKind::Read,
                                "awatch" => StopWatchpointKind::Access,
                                _ => unreachable!(),
                            };
                            watchpoint = Some(StopWatchpoint { kind, address });
                        }
                        "exception" | "exc" => {
                            if exception_code.is_none() {
                                exception_code = Some(value.to_string());
                            }
                        }
                        _ => {
                            let Ok(register) = parse_hex_u8(key) else {
                                continue;
                            };
                            let Ok(number) = parse_u32(value) else {
                                continue;
                            };

                            match register {
                                0x40 => pc = Some(number),
                                0x01 => sp = Some(number),
                                _ => {}
                            }
                        }
                    }
                }
            }

            StopPacket::Signal(StopSignal {
                signal,
                pc,
                sp,
                watchpoint,
                exception_code,
                raw: payload.to_string(),
            })
        }
        Some(b'W') | Some(b'X') if payload.len() >= 3 => {
            let code = parse_hex_u8(&payload[1..3]).unwrap_or(0);
            StopPacket::Exit(code)
        }
        _ => StopPacket::Reply(payload.to_string()),
    }
}
