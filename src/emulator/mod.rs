use cpu::*;
use timer::*;
use memory::*;
use video::*;

#[derive(Debug)]
pub enum StepResult {
    Progressed(usize),
    Paused,
    Exit
}

pub trait Emulator {
    type Error;
    type Memory: MemoryMap + AsMemoryRegionMut;

    fn place_rom_at(&mut self, location: u16, rom: &[u8]);
    fn initialize(&mut self) -> Result<(), Self::Error>;
    fn step(&mut self, fb: &mut FrameBuffer) -> Result<StepResult, Self::Error>;
    fn cpu(&self) -> &Cpu;
    fn mem(&self) -> &Self::Memory;
}

pub struct BbcEmulator<M> {
    cpu: Cpu,
    timer: Timer,
    mem: M,
    video: Crtc6845,
}

impl<M> BbcEmulator<M> {
    pub fn with_memory(mem: M) -> BbcEmulator<M> {
        use std::u16;

        BbcEmulator {
            cpu: Cpu::new(),
            timer: Timer::new(),
            mem: mem,
            video: Crtc6845::new(),
        }
    }
}

impl<M> Emulator for BbcEmulator<M> 
    where M: MemoryMap + AsMemoryRegionMut
{
    type Error = CpuError;
    type Memory = M;

    fn place_rom_at(&mut self, location: u16, rom: &[u8]) {
        use std::io::{self, Cursor};

        let mut region = self.mem.region_from_mut(location as _..)
                                 .unwrap_or_else(|e| e.0);
        io::copy(
            &mut Cursor::new(rom), 
            &mut Cursor::new(region.as_mut())
        ).unwrap();
    }

    fn initialize(&mut self) -> Result<(), CpuError> {
        self.cpu.initialize(&mut self.mem)
    }

    fn step(&mut self, fb: &mut FrameBuffer) -> Result<StepResult, CpuError> {
        let cycles = self.cpu.step(&mut self.mem)?;
        self.video.step(cycles, &mut self.mem, fb);
        if self.timer.step(cycles) {
            self.cpu.interrupt_request(&mut self.mem)?;
        }

        self.mem.clear_last_hw_access();
        Ok(StepResult::Progressed(cycles))
    }

    fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    fn mem(&self) -> &M {
        &self.mem
    }
}

