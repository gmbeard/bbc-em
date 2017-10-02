use cpu::*;

pub mod debugger;

#[derive(Debug)]
pub enum StepResult {
    Progressed(usize),
    Paused,
    Exit
}

pub trait Emulator {
    fn place_rom_at(&mut self, location: u16, rom: &[u8]);
    fn initialize(&mut self) -> Result<(), CpuError>;
    fn step(&mut self) -> Result<StepResult, CpuError>;
    fn cpu(&self) -> &Cpu;
    fn mem(&self) -> &[u8];
}

struct BbcEmulator {
    cpu: Cpu,
    mem: Vec<u8>
}

impl BbcEmulator {
    fn new() -> BbcEmulator {
        use std::u16;

        BbcEmulator {
            cpu: Cpu::new(),
            mem: vec![0x00; u16::MAX as usize]
        }
    }
}

impl Emulator for BbcEmulator {
    fn place_rom_at(&mut self, location: u16, rom: &[u8]) {
        use std::io::{self, Cursor};

        io::copy(
            &mut Cursor::new(rom), 
            &mut Cursor::new(&mut self.mem[location as usize..])
        ).unwrap();
    }

    fn initialize(&mut self) -> Result<(), CpuError> {
        self.cpu.initialize(&self.mem)
    }

    fn step(&mut self) -> Result<StepResult, CpuError> {
        let cycles = self.cpu.step(&mut self.mem)?;

        Ok(StepResult::Progressed(cycles))
    }

    fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    fn mem(&self) -> &[u8] {
        &self.mem
    }
}

