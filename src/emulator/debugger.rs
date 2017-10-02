use super::*;
use std::sync::mpsc::{Receiver, Sender, channel, TryRecvError};
use std::thread::{JoinHandle, spawn};
use cpu;
use std::io::{self, Write};

enum DebuggerState {
    Stop,
    Run,
    Step,
    Restart,
//    BreakAt(u16),
}

struct Debugger<T> {
    emulator: T,
    incoming: Receiver<String>,
    outgoing: Sender<String>,
    threads: (JoinHandle<io::Result<()>>, JoinHandle<io::Result<()>>),
    state: DebuggerState
}

fn listener(tx: Sender<String>) -> io::Result<()> {
    let mut buffer = String::with_capacity(64);
    while let Ok(_) = io::stdin().read_line(&mut buffer) {
        if let Err(e) = tx.send(buffer.clone()) {
            break;
        }
    }

    Ok(())
}

fn sender(rx: Receiver<String>) -> io::Result<()> {
    while let Ok(s) = rx.recv() {
        writeln!(io::stdout(), "{}", s)?;
    }

    Ok(())
}

impl<T: Emulator> Debugger<T> {
    fn new(emulator: T) -> Debugger<T> {
        let (tx, incoming) = channel();
        let (outgoing, rx) = channel();

        Debugger {
            emulator: emulator,
            incoming: incoming,
            outgoing: outgoing,
            threads: (spawn(move || listener(tx)), spawn(move || sender(rx))),
            state: DebuggerState::Stop
        }
    }

    fn process_debugger_queue(&mut self) -> Option<()> {
        match self.incoming.try_recv() {
            Ok(s) => {
                match s.as_ref() {
                    "next" => { self.state = DebuggerState::Stop; },
                    "continue" => { self.state = DebuggerState::Run; },
                    "restart" => { self.state = DebuggerState::Restart; },
                    _ => { self.outgoing.send(format!("Unknown command '{}'", s)); },
                }
            },
            Err(TryRecvError::Disconnected) => return None,
            _ => {}
        }

        Some(())
    }

    fn send_current_instruction(&mut self) -> Result<(), CpuError> {
        let (_, ins) = cpu::decode_instruction(&self.mem()[self.cpu().program_counter() as usize..])?;
        self.outgoing.send(format!("{:04x} {}", self.cpu().program_counter(), ins));
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
            DebuggerState::Step => {
                self.state = DebuggerState::Stop;
            },
            DebuggerState::Run => {
                return self.emulator.step();
            },
            DebuggerState::Restart => {
                self.state = DebuggerState::Stop;
                self.initialize()?;
                return Ok(StepResult::Paused); 
            }
        }

        let result = self.emulator.step()?;
        self.send_current_instruction()?;
        Ok(result)
    }

    fn cpu(&self) -> &Cpu {
        self.emulator.cpu()
    }

    fn mem(&self) -> &[u8] {
        self.emulator.mem()
    }
}

