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

use minifb::{Window, WindowOptions, Key};

use bbc_em::cpu::CpuError;
use bbc_em::emulator::{StepResult, Emulator, BbcEmulator};
use bbc_em::debugger::{
    Backend, 
    FrontEnd,
    FrontEndError,
};

use bbc_em::debugger::Error;

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

fn update_buffer(buf: &mut [u32], rows: usize, cycles: u64) {
    let cols = buf.len() / rows;
    let cycles = cycles / 1000;
    let cycle_row = cycles % rows as u64;
    let cycle_col = cycles % cols as u64;

    for y in 0..rows {
        for x in 0..cols {
            let y_distance = ((rows + cycle_row as usize - y) % rows);
            let x_distance = ((cols + cycle_col as usize - x) % cols);
            let distance = (((x_distance * 2) + (y_distance * 2)) as f32).sqrt().trunc() as u8;

            buf[x * y] = (distance as u32 |
                         (distance as u32) << 8 |
                         (distance as u32) << 16 |
                         (distance as u32) << 24) * 2;
        }
    }
}

fn run_emulator<E>(mut emu: E, args: &[String]) -> Result<(), ApplicationError>
    where E: Emulator,
          ApplicationError: From<E::Error>
{
    let mut frame_cycles = 0;

    let os_rom_file = args.iter().nth(1)
        .ok_or_else(|| ApplicationError::MissingRom("No OS ROM file specified!"))?;

    let lang_rom_file = args.iter().nth(2)
        .ok_or_else(|| ApplicationError::MissingRom("No Language ROM file specified!"))?;

    let os_rom = fs::File::open(os_rom_file)?.bytes().collect::<io::Result<Vec<_>>>()?;
    let lang_rom = fs::File::open(lang_rom_file)?.bytes().collect::<io::Result<Vec<_>>>()?;

    emu.place_rom_at(0x8000, lang_rom.as_slice());
    emu.place_rom_at(0xc000, os_rom.as_slice());
    emu.initialize()?;

    let mut buf: Vec<u32> = vec![0; 640 * 480];
    let mut window = Window::new("Bbc-Em",
                                 640,
                                 480,
                                 WindowOptions::default()).unwrap();

    let mut total_cycles: u64 = 0;
    while window.is_open() && !window.is_key_down(Key::Escape) {
//    loop {
        let start = Instant::now();

        while frame_cycles < CYCLES_PER_FRAME {
            match emu.step()? {
                StepResult::Progressed(cycles) => {
                    frame_cycles += cycles;
                },
                StepResult::Paused => break,
                StepResult::Exit => return Ok(()),
            }
        }

        total_cycles += frame_cycles as u64;

        update_buffer(&mut buf, 480, total_cycles);
        window.update_with_buffer(&buf).unwrap();

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
        (false, true) => run_emulator(Backend::new(BbcEmulator::new()), &args).unwrap(),
        (false, false) => run_emulator(BbcEmulator::new(), &args).unwrap(),
        _ => {
            eprintln!("--debug and --attach flags cannot be used together");
        }
    }
}

