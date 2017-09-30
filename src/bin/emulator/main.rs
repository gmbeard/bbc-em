extern crate bbc_em;

use std::env;
use std::io::Read;
use std::fs;
use std::path::Path;
use std::str::{self, FromStr};
use std::mem;
use std::slice;
use std::io;
use std::ops::{Deref, DerefMut};
use std::convert::AsRef;

use bbc_em::cpu::*;

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

fn load_rom<P: AsRef<Path>>(p: P) -> io::Result<Vec<u8>> {

    let mut rom = vec![0xea; MEM_SIZE];

    fs::File::open(p)
        .and_then(|mut f| {
            let mut v = Vec::with_capacity(ROM_SIZE);
            let read = f.read_to_end(&mut v)?;
            assert_eq!(read, ROM_SIZE);

            let rom_location = {
                unsafe {
                    let p = rom.as_mut_ptr()
                        .offset(MEM_SIZE as isize)
                        .offset(-(ROM_SIZE as isize));
                    slice::from_raw_parts_mut(p, ROM_SIZE)
                }
            };

            rom_location.copy_from_slice(&v);

            Ok(())
        })?;

    Ok(rom)
}

fn main() {
    let mut start = 0;

    let mut rom = load_rom(&env::args().nth(1).unwrap()).unwrap();

    assert_eq!(MEM_SIZE, rom.len());

    let mut cpu = Cpu::new();
    cpu.initialize(&mut rom).unwrap();

//    loop {
    for _ in 1..100000 {
        cpu.step(&mut rom);
    }
}

