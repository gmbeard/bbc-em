use std::sync::mpsc::{Receiver, Sender, channel, TryRecvError};
use std::thread::{JoinHandle, spawn};
use std::io::{self, Write};

use super::*;
use self::protocol::{DebuggerCmd, DebuggerResponse, IntoDebuggerMessage, FromDebuggerMessage};
use emulator::*;
use cpu::{self, Cpu, CpuError};

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
    while let Ok(cmd) = DebuggerCmd::from_debugger_message(&mut lock) {
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

impl<T: Emulator> Backend<T> {
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
                        self.outgoing.send(DebuggerResponse::StreamStart).unwrap();
                    },
                    DebuggerCmd::Continue => { 
                        self.state = DebuggerState::Run;
                        self.active_breakpoint.take();
                    },
//                    DebuggerCmd::Restart => { self.state = DebuggerState::Restart; },
                    DebuggerCmd::RequestPage(page) => {
                        let hi = (page as u32) << 8;
                        let next = hi + 0x0100;
                        self.outgoing.send(
                            DebuggerResponse::Page(hi as u16, self.mem()[hi as usize..next as usize].to_vec())
                        ).unwrap();
                    },
                    DebuggerCmd::BreakPoint(loc) => { 
                        self.breakpoints.push(loc);
                        self.outgoing.send(
                            DebuggerResponse::Message(format!("Breakpoint set to {:4x}", loc))
                        ).unwrap();
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
        self.outgoing.send(DebuggerResponse::Instruction(self.cpu().program_counter(), ins)).unwrap();
        Ok(())
    }
}

impl<T: Emulator> Emulator for Backend<T> {
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
                    self.outgoing.send(DebuggerResponse::StreamEnd).unwrap();
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
            self.outgoing.send(DebuggerResponse::Message("Breakpoint hit".to_string())).unwrap();
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

