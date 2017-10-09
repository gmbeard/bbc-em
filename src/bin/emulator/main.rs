extern crate bbc_em;
extern crate minifb;

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

const CYCLES_PER_SECOND: usize = 2_000_000;
const SPEED_DIVISOR: usize = 1;
const FRAMES_PER_SECOND: usize = 50;
const CYCLES_PER_FRAME: usize = CYCLES_PER_SECOND / SPEED_DIVISOR / FRAMES_PER_SECOND;
const MS_PER_FRAME: usize = 1000 / FRAMES_PER_SECOND;

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

fn update_buffer(buf: &mut [u32], cols: usize, rows: usize, cycles: u64) {
    use std::cmp;

    const SCALE: usize = 30;
//    let cols = buf.len() / rows;
    let cycles = cycles as usize / 2500;
    let cycle_row = cycles / cols * SCALE % rows;
    let cycle_col = cycles % cols;

    for (i, b) in buf.iter_mut().enumerate() {
        let row = i / cols % rows;
        let col = i % cols;
        
        *b = if (row >= cycle_row && row < cycle_row + SCALE) 
            && (col > cycle_col && col < cycle_col + SCALE) {
            0xffffffff
        }
        else {
            let mut alpha = (*b & 0xff000000) >> 24;
            let mut red = (*b & 0x00ff0000) >> 16;
            let mut green = (*b & 0x0000ff00) >> 8;
            let mut blue = (*b & 0x000000ff);
            
            alpha = cmp::max(0xff, alpha + 0xff - 8) - 0xff;
            red = cmp::max(0xff, red + 0xff - 8) - 0xff;
            green = cmp::max(0xff, green + 0xff - 8) - 0xff;
            blue = cmp::max(0xff, blue + 0xff - 8) - 0xff;

            0xff << 24 | red << 16 | green << 8 | blue
        };

    }
//    for y in 0..rows {
//        for x in 0..cols {
// 
//            buf[x * y] = 0xffffffff;
//            let current = buf[x * y];
//            buf[x * y] = {
//                if (x / SCALE) * (y / SCALE) == (cycle_row / SCALE) * (cycle_col / SCALE) {
//                    0
//                }
//                else {
//                    0xffffffff
//                }
//            };
//        }
//    }
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
    let mut frame_cycles = 0;

    let os_rom_file = args.iter().nth(1)
        .ok_or_else(|| ApplicationError::MissingRom("No OS ROM file specified!"))?;

//    let lang_rom_file = args.iter().nth(2)
//        .ok_or_else(|| ApplicationError::MissingRom("No Language ROM file specified!"))?;

    let os_rom = fs::File::open(os_rom_file)?.bytes().collect::<io::Result<Vec<_>>>()?;
//    let lang_rom = fs::File::open(lang_rom_file)?.bytes().collect::<io::Result<Vec<_>>>()?;

//    emu.place_rom_at(0x8000, lang_rom.as_slice());
    emu.place_rom_at(0xc000, os_rom.as_slice());
    emu.initialize()?;

    let mut buf: Vec<u32> = vec![0; 640 * 480];
    let mut window = Window::new("Bbc-Em",
                                 640,
                                 480,
                                 WindowOptions::default()).unwrap();

    let mut fb = FrameBuffer::new(640, 480);
    for b in fb.as_mut() {
        *b = 0xff606060;
    }
    let mut total_cycles: u64 = 0;
    while window.is_open() && !window.is_key_down(Key::Escape) {
//    loop {
        let start = Instant::now();

        while frame_cycles < CYCLES_PER_FRAME {
            match emu.step(&mut fb)? {
                StepResult::Progressed(cycles) => {
                    frame_cycles += cycles;
                },
                StepResult::Paused => break,
                StepResult::Exit => return Ok(()),
            }
        }

        total_cycles += frame_cycles as u64;

//        update_buffer(&mut buf, 640, 480, total_cycles);
        window.update_with_buffer(&fb).unwrap();

        frame_cycles -= cmp::min(CYCLES_PER_FRAME, frame_cycles);

        let delta = cmp::min(Duration::from_millis(MS_PER_FRAME as _), start.elapsed());
        thread::sleep(Duration::from_millis(MS_PER_FRAME as _) - delta);
    }

    Ok(())
}

fn main() {
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

