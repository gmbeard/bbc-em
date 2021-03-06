use std::sync::mpsc::{Receiver, Sender, channel, TryRecvError};
use std::thread::{JoinHandle, spawn};
use std::io::{self, Write};

use super::*;
use self::protocol::{DebuggerCmd, DebuggerResponse, IntoDebuggerMessage, FromDebuggerMessage};
use emulator::{StepResult, Emulator};
use cpu::{self, Cpu, CpuError};
use self::error::*;
use memory::AsMemoryRegion;
use video::FrameBuffer;

enum DebuggerState {
    Stop,
    Run,
    Step(u32),
//    BreakAt(u16),
}

pub struct Backend<T> {
    emulator: T,
    incoming: Receiver<DebuggerCmd>,
    outgoing: Sender<DebuggerResponse>,
    _threads: (JoinHandle<io::Result<()>>, JoinHandle<io::Result<()>>),
    state: DebuggerState,
    breakpoints: Vec<u16>,
    active_breakpoint: Option<u16>,
}

fn listener(tx: Sender<DebuggerCmd>) -> io::Result<()> {
    let stdin = io::stdin();
    let mut lock = stdin.lock();
    loop {
        let cmd = DebuggerCmd::from_debugger_message(&mut lock)?;
        if let Err(_) = tx.send(cmd) {
            break;
        }
    }

    Ok(())
}

fn sender(rx: Receiver<DebuggerResponse>) -> io::Result<()> {
    while let Ok(s) = rx.recv() {
        s.into_debugger_message(io::stdout())?;
        io::stdout().flush()?;
    }

    Ok(())
}

impl<T> Backend<T>
    where T: Emulator,
          DebuggerError: From<T::Error>
{
    pub fn new(emulator: T) -> Backend<T> {
        let (tx, incoming) = channel();
        let (outgoing, rx) = channel();

        Backend {
            emulator: emulator,
            incoming: incoming,
            outgoing: outgoing,
            _threads: (spawn(move || listener(tx)), spawn(move || sender(rx))),
            state: DebuggerState::Stop,
            breakpoints: vec![],
            active_breakpoint: None,
        }
    }

    fn process_debugger_queue(&mut self) -> Option<()> {
        match self.incoming.try_recv() {
            Ok(s) => {
                match s {
                    DebuggerCmd::Step(num) => { 
                        self.state = DebuggerState::Step(num);
                        self.active_breakpoint.take();
                        self.outgoing.send(DebuggerResponse::StreamStart).ok();
                    },
                    DebuggerCmd::Continue => { 
                        self.state = DebuggerState::Run;
                        self.active_breakpoint.take();
                    },
                    DebuggerCmd::RequestPage(page) => {
                        let (start, end) = (
                            ((page as u32) << 8) as usize,
                            (((page as u32) << 8) + 0x0100) as usize
                        );
                        let mem = self.mem()
                                      .region(start..end)
                                      .unwrap_or_else(|e| e.0)
                                      .iter()
                                      .map(|b| *b)
                                      .collect::<Vec<_>>();
                        self.outgoing.send(DebuggerResponse::Page(start as u16, mem)).ok();
                    },
                    DebuggerCmd::BreakPoint(loc) => { 
                        self.breakpoints.push(loc);
                        let msg = format!("Breakpoint set to {:04x}", loc);
                        self.outgoing.send(DebuggerResponse::Message(msg)).ok();
                    },
                    DebuggerCmd::RequestCpuState => {
                        let reg = self.emulator.cpu().registers();
                        self.outgoing.send(DebuggerResponse::CpuState(*reg)).ok();
                    },
                    DebuggerCmd::Print(mut num) => {
                        let pc = self.emulator.cpu().program_counter() as usize;
                        let mut offset = 0;
                        self.outgoing.send(DebuggerResponse::StreamStart).ok();
                        while num > 0 {
                            let mem = self.mem().region_from(pc + offset..)
                                                .unwrap_or_else(|e| e.0);
                            offset += match cpu::decode_instruction(&mem) {
                                Ok((bytes, ins)) => {
                                    let msg = format!("{:04x}: {}", pc + offset, ins);
                                    self.outgoing.send(DebuggerResponse::Message(msg)).ok();
                                    bytes
                                },
                                Err(_) => {
                                    let msg = format!("{:04x}: ...", pc + offset);
                                    self.outgoing.send(DebuggerResponse::Message(msg)).ok();
                                    1
                                }
                            };

                            num -=1;
                        }
                        self.outgoing.send(DebuggerResponse::StreamEnd).ok();
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

        let instruction_region = 
            self.mem()
                .region(self.cpu().program_counter() as _..self.cpu().program_counter() as usize + 4)
                .unwrap_or_else(|e| e.0);
        let (_, ins) = cpu::decode_instruction(&instruction_region)?;
        self.outgoing.send(DebuggerResponse::Instruction(self.cpu().program_counter(), ins)).ok();
        Ok(())
    }
}

impl<T> Emulator for Backend<T> 
    where T: Emulator,
          DebuggerError: From<T::Error>
{
    type Error = DebuggerError;
    type Memory = T::Memory;

    fn place_rom_at(&mut self, location: u16, rom: &[u8]) {
        self.emulator.place_rom_at(location, rom)
    }

    fn initialize(&mut self) -> Result<(), Self::Error> {
        self.emulator.initialize()?;
        Ok(self.send_current_instruction()?)
    }

    fn step<K: Fn(u8) -> bool>(&mut self, fb: &mut FrameBuffer, key_eval: K) -> Result<StepResult, Self::Error> {

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
                    self.outgoing.send(DebuggerResponse::StreamEnd).unwrap();
                    return Ok(StepResult::Paused);
                }
                else {
                    let result = self.emulator.step(fb, key_eval)?;
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

        let result = self.emulator.step(fb, key_eval)?;
        if let Some(bp) = self.breakpoints.iter().find(|i| **i == self.cpu().program_counter()) {
            self.active_breakpoint = Some(*bp);
            self.state = DebuggerState::Stop;
            self.outgoing.send(DebuggerResponse::Message("Breakpoint hit".to_string())).unwrap();
        }

        Ok(result)
    }

    fn cpu(&self) -> &Cpu {
        self.emulator.cpu()
    }

    fn mem(&self) -> &Self::Memory {
        self.emulator.mem()
    }

    fn keydown(&mut self, keynum: u32) {
        self.emulator.keydown(keynum);
    }

    fn clear_keyboard_buffer(&mut self) {
        self.emulator.clear_keyboard_buffer();
    }
}

