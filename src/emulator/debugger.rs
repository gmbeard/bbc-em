use super::*;
use std::sync::mpsc::{Receiver, Sender, channel, TryRecvError};
use std::thread::{JoinHandle, spawn};
use cpu;
use std::io::{self, Write, Read};
use std::str;

pub enum DebuggerInput {
    Step(u32),
    Continue,
    Restart,
    RequestPage(u8),
    BreakPoint(u16),
    Unknown(u8),
}

#[derive(Debug)]
pub enum DebuggerOutput {
    Instruction(u16, cpu::Instruction),
    Page(u16, Vec<u8>),
    Message(String),
    Unknown(u8),
    StreamStart,
    StreamEnd,
}

unsafe impl Send for DebuggerInput { }
unsafe impl Send for DebuggerOutput { }

pub trait IntoDebuggerMessage {
    fn into_debugger_message<W: Write>(&self, writer: W) -> io::Result<usize>;
}

pub trait FromDebuggerMessage : Sized {
    fn from_debugger_message<R: Read>(reader: R) -> io::Result<Self>;
}

impl FromDebuggerMessage for DebuggerInput {
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

                DebuggerInput::Step(n)
            }
            0x02 => DebuggerInput::Continue,
            0x03 => DebuggerInput::Restart,
            0x04 => {
                DebuggerInput::RequestPage(vec[0])
            },
            0x05 => {
                let loc = vec[0] as u16 | (vec[1] as u16) << 8;
                DebuggerInput::BreakPoint(loc)
            }
            _ => DebuggerInput::Unknown(id),
        };

        Ok(cmd)
    }
}

impl FromDebuggerMessage for DebuggerOutput {
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
                DebuggerOutput::Instruction(loc, ins)
            },
            0x02 => {
                let page = buf[0] as u16 | (buf[1] as u16) << 8;
                DebuggerOutput::Page(page, buf.split_off(2))
            },
            0x03 => {
                DebuggerOutput::Message(
                    str::from_utf8(&buf)
                        .map_err(|e|io::Error::new(io::ErrorKind::Other, e))?
                        .to_string()
                )
            },
            0xfd => DebuggerOutput::StreamStart,
            0xfe => DebuggerOutput::StreamEnd,
            _ => DebuggerOutput::Unknown(id)
        };

        Ok(cmd)
    }
}

impl IntoDebuggerMessage for DebuggerInput {
    fn into_debugger_message<W: Write>(&self, mut writer: W) -> io::Result<usize> {
        let written = match *self {
            DebuggerInput::Step(ref num) => {
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
            DebuggerInput::Continue => {
                writer.write_all(&[0x02, 0x00, 0x00])?;
                3
            },
            DebuggerInput::Restart => {
                writer.write_all(&[0x03, 0x00, 0x00])?;
                3
            },
            DebuggerInput::RequestPage(ref page) => {
                writer.write_all(&[0x04, 0x01, 0x00, *page])?;
                4
            },
            DebuggerInput::BreakPoint(ref loc) => {
                writer.write_all(&[0x05, 0x02, 0x00, (*loc as u8) & 0xff, (*loc >> 8) as u8])?;
                5
            }
            DebuggerInput::Unknown(_) => {
                writer.write_all(&[0xff, 0x00, 0x00])?;
                3
            }
        };

        Ok(written)
    }
}

impl IntoDebuggerMessage for DebuggerOutput {
    fn into_debugger_message<W: Write>(&self, mut writer: W) -> io::Result<usize> {
        let written = match *self {
            DebuggerOutput::Instruction(ref loc, ref ins) => {
                let str = format!("{:04x} {}", loc, ins);
                let data_len = str.len();
                let low = data_len as u8;
                let hi = ((data_len as u16 & 0xff00) >> 8) as u8;
                writer.write_all(&[0x03, low, hi])?;
                writer.write_all(str.as_ref())?;

                data_len + 1
            },
            DebuggerOutput::Message(ref s) => {
                let data_len = s.len();
                let low = data_len as u8;
                let hi = ((data_len as u16 & 0xff00) >> 8) as u8;
                writer.write_all(&[0x03, low, hi])?;
                writer.write_all(s.as_ref())?;

                data_len + 1
            },

            DebuggerOutput::Page(ref loc, ref mem) => {
                let data_len = mem.len() + 2;
                let low = data_len as u8;
                let hi = ((data_len as u16 & 0xff00) >> 8) as u8;
                let page_low = (*loc as u8);
                let page_high = (*loc >> 8) as u8;
                writer.write_all(&[0x02, low, hi, page_low, page_high])?;
                writer.write_all(mem)?;

                mem.len() + 1
            },

            DebuggerOutput::Unknown(_) => {
                writer.write_all(&[0xff, 0x00, 0x00])?;
                3
            },

            DebuggerOutput::StreamStart => {
                writer.write_all(&[0xfd, 0x00, 0x00])?;
                3
            },

            DebuggerOutput::StreamEnd => {
                writer.write_all(&[0xfe, 0x00, 0x00])?;
                3
            },
        };

        Ok(written)

    }
}

enum DebuggerState {
    Stop,
    Run,
    Step(u32),
//    BreakAt(u16),
}

pub struct Debugger<T> {
    emulator: T,
    incoming: Receiver<DebuggerInput>,
    outgoing: Sender<DebuggerOutput>,
    threads: (JoinHandle<io::Result<()>>, JoinHandle<io::Result<()>>),
    state: DebuggerState,
    breakpoints: Vec<u16>,
    active_breakpoint: Option<u16>,
}

fn listener(tx: Sender<DebuggerInput>) -> io::Result<()> {
    let stdin = io::stdin();
    let mut lock = stdin.lock();
    while let Ok(cmd) = DebuggerInput::from_debugger_message(&mut lock) {
        if let Err(_) = tx.send(cmd) {
            break;
        }
    }
    Ok(())
}

fn sender(rx: Receiver<DebuggerOutput>) -> io::Result<()> {
    while let Ok(s) = rx.recv() {
        s.into_debugger_message(io::stdout())?;
        io::stdout().flush()?;
    }

    Ok(())
}

impl<T: Emulator> Debugger<T> {
    pub fn new(emulator: T) -> Debugger<T> {
        let (tx, incoming) = channel();
        let (outgoing, rx) = channel();

        Debugger {
            emulator: emulator,
            incoming: incoming,
            outgoing: outgoing,
            threads: (spawn(move || listener(tx)), spawn(move || sender(rx))),
            state: DebuggerState::Stop,
            breakpoints: vec![],
            active_breakpoint: None,
        }
    }

    fn process_debugger_queue(&mut self) -> Option<()> {
        match self.incoming.try_recv() {
            Ok(s) => {
                match s {
                    DebuggerInput::Step(num) => { 
                        self.state = DebuggerState::Step(num);
                        self.active_breakpoint.take();
                        self.outgoing.send(DebuggerOutput::StreamStart).unwrap();
                    },
                    DebuggerInput::Continue => { 
                        self.state = DebuggerState::Run;
                        self.active_breakpoint.take();
                    },
//                    DebuggerInput::Restart => { self.state = DebuggerState::Restart; },
                    DebuggerInput::RequestPage(page) => {
                        let hi = (page as u32) << 8;
                        let next = hi + 0x0100;
                        self.outgoing.send(
                            DebuggerOutput::Page(hi as u16, self.mem()[hi as usize..next as usize].to_vec())
                        );
                    },
                    DebuggerInput::BreakPoint(loc) => { 
                        self.breakpoints.push(loc);
                        self.outgoing.send(
                            DebuggerOutput::Message(format!("Breakpoint set to {:4x}", loc))
                        );
                    },
                    _ => {}
                }
            },
            Err(TryRecvError::Disconnected) => return None,
            _ => {}
        }

        Some(())
    }

    fn send_current_instruction(&mut self) -> Result<(), CpuError> {
        let (_, ins) = cpu::decode_instruction(&self.mem()[self.cpu().program_counter() as usize..])?;
        self.outgoing.send(DebuggerOutput::Instruction(self.cpu().program_counter(), ins)).unwrap();
        Ok(())
    }
}

impl<T: Emulator> Emulator for Debugger<T> {
    fn place_rom_at(&mut self, location: u16, rom: &[u8]) {
        self.emulator.place_rom_at(location, rom)
    }

    fn initialize(&mut self) -> Result<(), CpuError> {
        self.emulator.initialize()?;
        self.send_current_instruction()
    }

    fn step(&mut self) -> Result<StepResult, CpuError> {

        if self.process_debugger_queue().is_none() {
            return Ok(StepResult::Exit);
        }

        match self.state {
            DebuggerState::Stop => {
                return Ok(StepResult::Paused)
            }
            DebuggerState::Step(num) => {
                if num == 0 {
                    self.state = DebuggerState::Stop;
                    self.outgoing.send(DebuggerOutput::StreamEnd).unwrap();
                    return Ok(StepResult::Paused);
                }
                else {
                    let result = self.emulator.step()?;
                    self.send_current_instruction()?;
                    self.state = DebuggerState::Step(num - 1);
                    return Ok(result);
                }
            },
            DebuggerState::Run => {
                if self.active_breakpoint.is_some() {
                    return Ok(StepResult::Paused);
                }
            },
        }

        let result = self.emulator.step()?;
        if let Some(bp) = self.breakpoints.iter().find(|i| **i == self.cpu().program_counter()) {
            self.active_breakpoint = Some(*bp);
            self.state = DebuggerState::Stop;
            self.outgoing.send(DebuggerOutput::Message("Breakpoint hit".to_string())).unwrap();
        }

        Ok(result)
    }

    fn cpu(&self) -> &Cpu {
        self.emulator.cpu()
    }

    fn mem(&self) -> &[u8] {
        self.emulator.mem()
    }
}

