extern crate bbc_em;

use std::env;
use std::io::{Read, Write};
use std::fs;
use std::path::Path;
use std::str::{self, FromStr};
use std::slice;
use std::io;
use std::ops::{Deref, DerefMut};
use std::convert::AsRef;
use std::time::{Duration, Instant};
use std::thread;
use std::cmp;

use bbc_em::cpu::*;
use bbc_em::timer::*;

const MEM_SIZE: usize = 1 << 16; //std::u16::MAX as usize;
const ROM_SIZE: usize = MEM_SIZE / 4;

#[derive(Debug)]
struct MemoryLocation(u16);

impl Deref for MemoryLocation {
    type Target = u16;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MemoryLocation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
struct MemoryLocationParseError;

impl FromStr for MemoryLocation {
    type Err = MemoryLocationParseError;
    fn from_str(s: &str) -> Result<MemoryLocation, Self::Err> {
        let mut addr: u16 = 0;
        for b in s.as_bytes() {

            let val = match *b {
                b'0'...b'9' => (*b as u16 - b'0' as u16),
                b'a'...b'f' => 10 + (*b as u16 - b'a' as u16),
                b'A'...b'F' => 10 + (*b as u16 - b'A' as u16),
                _ => return Err(MemoryLocationParseError)
            };

            addr = addr.checked_mul(16).ok_or_else(|| MemoryLocationParseError)?;
            addr = addr.checked_add(val).ok_or_else(|| MemoryLocationParseError)?;
        }

        Ok(MemoryLocation(addr))
    }
}

fn load_rom_at<P: AsRef<Path>>(p: P, loc: u16, mem: &mut [u8]) -> io::Result<u64> {
    fs::File::open(p)
        .and_then(|mut f| {
            let mut mem = io::Cursor::new(&mut mem[loc as _..]);
            io::copy(&mut f, &mut mem)
        })
}

const CYCLES_PER_SECOND: usize = 2_000_000;
const SPEED_DIVISOR: usize = 1;
const FRAMES_PER_SECOND: usize = 50;
const CYCLES_PER_FRAME: usize = CYCLES_PER_SECOND / SPEED_DIVISOR / FRAMES_PER_SECOND;
const MS_PER_FRAME: usize = 1000 / FRAMES_PER_SECOND;

fn emulator() -> Result<(), io::Error> {
    let mut frame_cycles = 0;

    let os_rom_file = env::args()
        .nth(1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "No OS ROM file specified!"))?;

    let lang_rom_file = env::args()
        .nth(2)
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "No Language ROM file specified!"))?;

    let mut mem = vec![0x00; MEM_SIZE];

    load_rom_at(&lang_rom_file, 0x8000, &mut mem)?;
    load_rom_at(&os_rom_file, 0xc000, &mut mem)?;

    assert_eq!(MEM_SIZE, mem.len());

    let mut cpu = Cpu::new();
    let mut timer = Timer::new();

    cpu.initialize(&mut mem).unwrap();

    loop {
        let start = Instant::now();

        while frame_cycles < CYCLES_PER_FRAME {
            match cpu.step(&mut mem) {
                Ok(cycles) => {
                    frame_cycles += cycles;
                    if timer.step(cycles) {
                        if cpu.interrupt_request(&mut mem).unwrap() {
                            println!("** Interrupt requested **");
                        }
                    }
                },
                Err(CpuError::Paused) => break,
                Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e))
            }
        }

        frame_cycles -= cmp::min(CYCLES_PER_FRAME, frame_cycles);

        let delta = cmp::min(Duration::from_millis(MS_PER_FRAME as _), start.elapsed());
        thread::sleep(Duration::from_millis(MS_PER_FRAME as _) - delta);
    }

    Ok(())
}

fn main() {
    emulator().unwrap();
}

