use cpu::*;
use timer::*;

#[derive(Debug)]
pub enum StepResult {
    Progressed(usize),
    Paused,
    Exit
}

pub trait Emulator {
    type Error;

    fn place_rom_at(&mut self, location: u16, rom: &[u8]);
    fn initialize(&mut self) -> Result<(), Self::Error>;
    fn step(&mut self) -> Result<StepResult, Self::Error>;
    fn cpu(&self) -> &Cpu;
    fn mem(&self) -> &[u8];
}

pub struct BbcEmulator {
    cpu: Cpu,
    timer: Timer,
    mem: Vec<u8>
}

impl BbcEmulator {
    pub fn new() -> BbcEmulator {
        use std::u16;

        BbcEmulator {
            cpu: Cpu::new(),
            timer: Timer::new(),
            mem: vec![0x00; u16::MAX as usize + 1]
        }
    }
}

impl Emulator for BbcEmulator {
    type Error = CpuError;

    fn place_rom_at(&mut self, location: u16, rom: &[u8]) {
        use std::io::{self, Cursor};

        io::copy(
            &mut Cursor::new(rom), 
            &mut Cursor::new(&mut self.mem[location as usize..])
        ).unwrap();
    }

    fn initialize(&mut self) -> Result<(), CpuError> {
        self.cpu.initialize(&mut self.mem)
    }

    fn step(&mut self) -> Result<StepResult, CpuError> {
        let cycles = self.cpu.step(&mut self.mem)?;
        if self.timer.step(cycles) {
            self.cpu.interrupt_request(&mut self.mem)?;
        }

        Ok(StepResult::Progressed(cycles))
    }

    fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    fn mem(&self) -> &[u8] {
        &self.mem
    }
}

