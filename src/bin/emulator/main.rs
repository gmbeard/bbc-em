extern crate bbc_em;
extern crate minifb;
extern crate env_logger;

use std::env;
use std::io::Read;
use std::fs;
use std::str;
use std::io;
use std::time::{Duration, Instant};
use std::thread;
use std::cmp;
use std::path::Path;

use minifb::{Window, WindowOptions, Key};

use bbc_em::cpu::CpuError;
use bbc_em::emulator::{StepResult, Emulator, BbcEmulator};
use bbc_em::debugger::{
    Backend, 
    FrontEnd,
    FrontEndError,
};
use bbc_em::memory::Map;
use bbc_em::debugger::Error;
use bbc_em::video::FrameBuffer;

const NS_PER_CYCLE: u64 = 500;
const MAX_FRAME_NS: u64 = 2_000_000;

struct Timer(Instant);

impl Timer {
    fn new() -> Timer {
        Timer(Instant::now())
    }

    fn elapsed_nanos(&self) -> u64 {
        let point = self.0.elapsed();
        (point.as_secs() * 1_000_000_000) + point.subsec_nanos() as u64
    }
}

#[derive(Debug)]
enum ApplicationError {
    MissingRom(&'static str),
    Io(io::Error),
    Emulator(CpuError),
    DebuggerFrontEnd(FrontEndError),
    DebuggerBackEnd(Error),
}

impl From<io::Error> for ApplicationError {
    fn from(e: io::Error) -> ApplicationError {
        ApplicationError::Io(e)
    }
}

impl From<CpuError> for ApplicationError {
    fn from(e: CpuError) -> ApplicationError {
        ApplicationError::Emulator(e)
    }
}

impl From<FrontEndError> for ApplicationError {
    fn from(e: FrontEndError) -> ApplicationError {
        ApplicationError::DebuggerFrontEnd(e)
    }
}

impl From<Error> for ApplicationError {
    fn from(e: Error) -> ApplicationError {
        ApplicationError::DebuggerBackEnd(e)
    }
}

fn load_rom_file<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    fs::File::open(path)?.bytes().collect::<io::Result<Vec<_>>>()
}

fn build_memory(args: &[String]) -> io::Result<Map> {
    let mut map = Map::new().with_hw_range(0xfe00..0xff00);
    for f in &args[2..] {
        map.add_paged_rom(load_rom_file(f)?);
    }

    Ok(map)
}

fn run_emulator<E>(mut emu: E, args: &[String]) -> Result<(), ApplicationError>
    where E: Emulator,
          ApplicationError: From<E::Error>
{
    let os_rom_file = args.iter().nth(1)
        .ok_or_else(|| ApplicationError::MissingRom("No OS ROM file specified!"))?;

    let os_rom = fs::File::open(os_rom_file)?.bytes().collect::<io::Result<Vec<_>>>()?;

    emu.place_rom_at(0xc000, os_rom.as_slice());
    emu.initialize()?;

    let mut window = Window::new("Bbc-Em",
                                 640,
                                 480,
                                 WindowOptions::default()).unwrap();

    let mut fb = FrameBuffer::new(640, 480);
    for b in fb.as_mut() {
        *b = 0xff606060;
    }

    let timer = Timer::new();
    let mut emulated_cycles = 0;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        window.get_keys().map(|keys| {
            for k in keys {
                emu.keydown(k as u32);
            }
        });

        let target_cycles = timer.elapsed_nanos() / NS_PER_CYCLE;
        let frame_timer = Timer::new();

        while emulated_cycles < target_cycles
            && frame_timer.elapsed_nanos() < MAX_FRAME_NS
        {
            match emu.step(&mut fb, |k| {
                k == 0x42     
            })? 
            {
                StepResult::Progressed(cycles) => {
                     emulated_cycles += cycles as u64;
                },
                StepResult::Paused => {
                    break;
                }
                StepResult::Exit => return Ok(()),
            }
        }

        window.update_with_buffer(&fb).unwrap();
        thread::sleep(Duration::from_millis(3));
    }

    Ok(())
}

fn main() {
    env_logger::init().ok();

    let mut args = env::args().collect::<Vec<_>>();
    let mut debug = false;
    let mut attach = false;

    args.iter()
        .position(|i| *i == "--debug")
        .map(|i| {
            args.remove(i);
            debug = true;
        });

    args.iter()
        .position(|i| *i == "--attach")
        .map(|i| {
            args.remove(i);
            attach = true
        });

    match (debug, attach) {
        (true, false) => FrontEnd::with_args(&args).run().unwrap(),
        (false, true) => {
            let emu = BbcEmulator::with_memory(build_memory(&args).unwrap());
            run_emulator(Backend::new(emu), &args).unwrap();
        }
        (false, false) => {
            let emu = BbcEmulator::with_memory(build_memory(&args).unwrap());
            run_emulator(emu, &args).unwrap();
        }
        _ => {
            eprintln!("--debug and --attach flags cannot be used together");
        }
    }
}

