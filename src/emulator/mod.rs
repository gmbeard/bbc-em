use cpu::*;
use timer::*;
use memory::*;
use video::*;
use via;

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
    mem: M,
    video: Crtc6845,
    system_via: via::System,
}

impl<M> BbcEmulator<M> {
    pub fn with_memory(mem: M) -> BbcEmulator<M> {
        use std::u16;

        BbcEmulator {
            cpu: Cpu::new(),
            mem: mem,
            video: Crtc6845::new(),
            system_via: via::System::new(),
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
        let mut irq = false;

        let cycles = self.cpu.step(&mut self.mem)?;
        self.system_via.step(cycles, &mut self.mem, || { irq = true });
        self.video.step(cycles, &mut self.mem, fb);

        if irq {
            self.cpu.interrupt_request(&mut self.mem);
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

