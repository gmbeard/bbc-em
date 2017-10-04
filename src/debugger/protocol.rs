use std::io::{self, Write, Read};
use std::str;

use cpu;

pub enum DebuggerCmd {
    Step(u32),
    Continue,
    Restart,
    RequestPage(u8),
    BreakPoint(u16),
    Unknown(u8),
}

#[derive(Debug)]
pub enum DebuggerResponse {
    Instruction(u16, cpu::Instruction),
    Page(u16, Vec<u8>),
    Message(String),
    Unknown(u8),
    StreamStart,
    StreamEnd,
}

unsafe impl Send for DebuggerCmd { }
unsafe impl Send for DebuggerResponse { }

pub trait IntoDebuggerMessage {
    fn into_debugger_message<W: Write>(&self, writer: W) -> io::Result<usize>;
}

pub trait FromDebuggerMessage : Sized {
    fn from_debugger_message<R: Read>(reader: R) -> io::Result<Self>;
}

impl FromDebuggerMessage for DebuggerCmd {
    fn from_debugger_message<R: Read>(mut reader: R) -> io::Result<Self> {
        let mut header: [u8; 3] = [0x00; 3];
        reader.read_exact(&mut header)?;
        let id = header[0];
        let size = header[1] as u16 | (header[2] as u16) << 8;
        let mut vec = Vec::with_capacity(size as usize);
        vec.resize(size as usize, 0x00);
        reader.read_exact(&mut vec)?;

        let cmd = match id {
            0x01 => {
                let mut n: u32 = 0;
                for (i, b) in vec[..4].iter().enumerate() {
                    n |= (*b as u32) << (8 * i); 
                }

                DebuggerCmd::Step(n)
            }
            0x02 => DebuggerCmd::Continue,
            0x03 => DebuggerCmd::Restart,
            0x04 => {
                DebuggerCmd::RequestPage(vec[0])
            },
            0x05 => {
                let loc = vec[0] as u16 | (vec[1] as u16) << 8;
                DebuggerCmd::BreakPoint(loc)
            }
            _ => DebuggerCmd::Unknown(id),
        };

        Ok(cmd)
    }
}

impl FromDebuggerMessage for DebuggerResponse {
    fn from_debugger_message<R: Read>(mut reader: R) -> io::Result<Self> {
        let mut header = [0x00; 3];
        reader.read_exact(&mut header)?;
        let id = header[0];
        let size = header[1] as u16 | (header[2] as u16) << 8;

        let mut buf = Vec::with_capacity(size as usize);
        buf.resize(size as usize, 0x00);
        reader.read_exact(&mut buf)?;
        let cmd = match id {
            0x01 => {
                let loc = buf[0] as u16 | (buf[1] as u16) << 8;
                let (_, ins) = cpu::decode_instruction(&buf[2..])
                    .map_err(|_| io::Error::new(io::ErrorKind::Other, "Invalid instruction"))?;
                DebuggerResponse::Instruction(loc, ins)
            },
            0x02 => {
                let page = buf[0] as u16 | (buf[1] as u16) << 8;
                DebuggerResponse::Page(page, buf.split_off(2))
            },
            0x03 => {
                DebuggerResponse::Message(
                    str::from_utf8(&buf)
                        .map_err(|e|io::Error::new(io::ErrorKind::Other, e))?
                        .to_string()
                )
            },
            0xfd => DebuggerResponse::StreamStart,
            0xfe => DebuggerResponse::StreamEnd,
            _ => DebuggerResponse::Unknown(id)
        };

        Ok(cmd)
    }
}

impl IntoDebuggerMessage for DebuggerCmd {
    fn into_debugger_message<W: Write>(&self, mut writer: W) -> io::Result<usize> {
        let written = match *self {
            DebuggerCmd::Step(ref num) => {
                writer.write_all(&[
                    0x01, 
                    0x04, 
                    0x00,
                    (*num as u8),
                    (*num >> 8) as u8,
                    (*num >> 16) as u8,
                    (*num >> 24) as u8,
                ])?;
                7
            },
            DebuggerCmd::Continue => {
                writer.write_all(&[0x02, 0x00, 0x00])?;
                3
            },
            DebuggerCmd::Restart => {
                writer.write_all(&[0x03, 0x00, 0x00])?;
                3
            },
            DebuggerCmd::RequestPage(ref page) => {
                writer.write_all(&[0x04, 0x01, 0x00, *page])?;
                4
            },
            DebuggerCmd::BreakPoint(ref loc) => {
                writer.write_all(&[0x05, 0x02, 0x00, (*loc as u8) & 0xff, (*loc >> 8) as u8])?;
                5
            }
            DebuggerCmd::Unknown(_) => {
                writer.write_all(&[0xff, 0x00, 0x00])?;
                3
            }
        };

        Ok(written)
    }
}

impl IntoDebuggerMessage for DebuggerResponse {
    fn into_debugger_message<W: Write>(&self, mut writer: W) -> io::Result<usize> {
        let written = match *self {
            DebuggerResponse::Instruction(ref loc, ref ins) => {
                let str = format!("{:04x} {}", loc, ins);
                let data_len = str.len();
                let low = data_len as u8;
                let hi = ((data_len as u16 & 0xff00) >> 8) as u8;
                writer.write_all(&[0x03, low, hi])?;
                writer.write_all(str.as_ref())?;

                data_len + 1
            },
            DebuggerResponse::Message(ref s) => {
                let data_len = s.len();
                let low = data_len as u8;
                let hi = ((data_len as u16 & 0xff00) >> 8) as u8;
                writer.write_all(&[0x03, low, hi])?;
                writer.write_all(s.as_ref())?;

                data_len + 1
            },

            DebuggerResponse::Page(ref loc, ref mem) => {
                let data_len = mem.len() + 2;
                let low = data_len as u8;
                let hi = ((data_len as u16 & 0xff00) >> 8) as u8;
                let page_low = *loc as u8;
                let page_high = (*loc >> 8) as u8;
                writer.write_all(&[0x02, low, hi, page_low, page_high])?;
                writer.write_all(mem)?;

                mem.len() + 1
            },

            DebuggerResponse::Unknown(_) => {
                writer.write_all(&[0xff, 0x00, 0x00])?;
                3
            },

            DebuggerResponse::StreamStart => {
                writer.write_all(&[0xfd, 0x00, 0x00])?;
                3
            },

            DebuggerResponse::StreamEnd => {
                writer.write_all(&[0xfe, 0x00, 0x00])?;
                3
            },
        };

        Ok(written)

    }
}

