extern crate bbc_em;

use std::env;
use std::io::{Read, Write};
use std::fs;
use std::str::{self, FromStr};
use std::io;
use std::ops::{Deref, DerefMut};
use std::time::{Duration, Instant};
use std::thread;
use std::cmp;
use std::process;

use bbc_em::emulator::{StepResult, Emulator, BbcEmulator};
use bbc_em::debugger::{
    Backend, 
};

use bbc_em::debugger::protocol::{
    DebuggerCmd, 
    DebuggerResponse, 
    IntoDebuggerMessage,
    FromDebuggerMessage,
};

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

const CYCLES_PER_SECOND: usize = 2_000_000;
const SPEED_DIVISOR: usize = 1;
const FRAMES_PER_SECOND: usize = 50;
const CYCLES_PER_FRAME: usize = CYCLES_PER_SECOND / SPEED_DIVISOR / FRAMES_PER_SECOND;
const MS_PER_FRAME: usize = 1000 / FRAMES_PER_SECOND;

fn run_emulator<E: Emulator>(mut emu: E, args: &[String]) -> Result<(), io::Error> {
    let mut frame_cycles = 0;

    let os_rom_file = args.iter().nth(1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "No OS ROM file specified!"))?;

    let lang_rom_file = args.iter().nth(2)
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "No Language ROM file specified!"))?;

    let os_rom = fs::File::open(os_rom_file)?.bytes().collect::<io::Result<Vec<_>>>()?;
    let lang_rom = fs::File::open(lang_rom_file)?.bytes().collect::<io::Result<Vec<_>>>()?;

    emu.place_rom_at(0x8000, lang_rom.as_slice());
    emu.place_rom_at(0xc000, os_rom.as_slice());
    emu.initialize().unwrap();

    loop {
        let start = Instant::now();

        while frame_cycles < CYCLES_PER_FRAME {
            match emu.step() {
                Ok(StepResult::Progressed(cycles)) => {
                    frame_cycles += cycles;
                },
                Ok(StepResult::Paused) => break,
                Ok(StepResult::Exit) => return Ok(()),
                Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e))
            }
        }

        frame_cycles -= cmp::min(CYCLES_PER_FRAME, frame_cycles);

        let delta = cmp::min(Duration::from_millis(MS_PER_FRAME as _), start.elapsed());
        thread::sleep(Duration::from_millis(MS_PER_FRAME as _) - delta);
    }

    Ok(())
}

fn print_memory_page(num: u16, mem: Vec<u8>) {
    println!("\tMemory at page {:04x}...\n", num);
    let mut current_row: u32 = num as u32 & 0xff00;
    while current_row < ((num as u32 & 0xff00) + 0x0100) {
        print!("\t{:04x}:", current_row);
        let current_col = current_row & 0x000f;
        for col in current_col..0x0010 {
            print!(" {:02x}", mem[(current_row - (num as u32 & 0xff00)) as usize + col as usize]);
        }

        println!();
        current_row += 0x0010;
    }
}

fn process_cmd(s: &str) -> Option<DebuggerCmd> {
    if s.starts_with("next") || s.starts_with("n ") || s == "n" {
        let num = s.split(" ").nth(1)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or_else(|| 1);
        return Some(DebuggerCmd::Step(num));
    }

    if s.starts_with("page") {
        let mut parts = s.split(" ");
        let loc = parts.nth(1).unwrap().parse::<MemoryLocation>().unwrap();
        return Some(DebuggerCmd::RequestPage(*loc as u8));
    }

    if s.starts_with("break") {
        let loc = s.split(" ").nth(1).unwrap().parse::<MemoryLocation>().unwrap();
        return Some(DebuggerCmd::BreakPoint(*loc as u16));
    }

    println!("Unknown command: {}", s);

    None
}

fn process_debugger_messages<R: Read>(mut reader: R) {

    use DebuggerResponse::*;

    let mut is_stream = false;
    while let Ok(msg) = DebuggerResponse::from_debugger_message(&mut reader) {
        match msg {
            Message(msg) => writeln!(io::stdout(), "\t{}", msg).unwrap(),
            Page(num, mem) => print_memory_page(num, mem),
            StreamStart => is_stream = true,
            StreamEnd => is_stream = false,
            _ => {}
        }

        io::stdout().flush().unwrap();

        if !is_stream {
            break;
        }
    }
}

fn spawn_debugger_front_end(args: &[String]) {
    let mut child = process::Command::new(&args[0])
        .args(&["--attach", &args[1], &args[2]])
        .stdout(process::Stdio::piped())
        .stdin(process::Stdio::piped())
        .spawn()
        .unwrap();

    let mut input_buffer = String::with_capacity(64);

    println!("Staring debugger...");
    process_debugger_messages(child.stdout.as_mut().unwrap());

    io::stdout().flush().unwrap();

    loop {
        write!(io::stdout(), "bbc-em> ").unwrap();
        io::stdout().flush().unwrap();

        let bytes = io::stdin().read_line(&mut input_buffer).unwrap();
        let msg = match &input_buffer[..bytes-2] {
            "continue" | "c" => Some(DebuggerCmd::Continue),
            "quit" => break,
            _ => process_cmd(&input_buffer[..bytes-2]),
        };

        input_buffer.clear();

        if let Some(msg) = msg {
            msg.into_debugger_message(child.stdin.as_mut().unwrap()).unwrap();
            child.stdin.as_mut().unwrap().flush().unwrap();
        }
        else {
            continue;
        }
        
        process_debugger_messages(child.stdout.as_mut().unwrap());
    }

    child.kill().unwrap();
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
        (true, false) => spawn_debugger_front_end(&args),
        (false, true) => run_emulator(Backend::new(BbcEmulator::new()), &args).unwrap(),
        (false, false) => run_emulator(BbcEmulator::new(), &args).unwrap(),
        _ => {
            eprintln!("--debug and --attach flags cannot be used together");
        }
    }
}

