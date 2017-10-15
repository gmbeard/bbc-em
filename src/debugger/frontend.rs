use std::io::{Read, Write};
use std::str::{self, FromStr};
use std::io;
use std::ops::{Deref, DerefMut};
use std::process;
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT};
use std::num;

#[cfg(windows)]
mod signal {
    extern crate kernel32;
    extern crate winapi;

    pub use self::kernel32::SetConsoleCtrlHandler;
    pub use self::winapi::minwindef::{BOOL, DWORD, TRUE, FALSE};
    pub use self::winapi::wincon::{PHANDLER_ROUTINE, CTRL_C_EVENT};
}

use cpu::{Registers, CpuError};

use debugger::protocol::{
    DebuggerCmd, 
    DebuggerResponse, 
    IntoDebuggerMessage,
    FromDebuggerMessage,
};

use debugger::Error;

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

#[derive(Debug)]
pub enum FrontEndError {
    MissingRom(&'static str),
    Io(io::Error),
    Emulator(CpuError),
    Debugger(Error),
    DebuggerCommand(MalformedCommandError),
}

impl From<io::Error> for FrontEndError {
    fn from(e: io::Error) -> FrontEndError {
        FrontEndError::Io(e)
    }
}

impl From<CpuError> for FrontEndError {
    fn from(e: CpuError) -> FrontEndError {
        FrontEndError::Emulator(e)
    }
}

impl From<Error> for FrontEndError {
    fn from(e: Error) -> FrontEndError {
        FrontEndError::Debugger(e)
    }
}

impl From<MalformedCommandError> for FrontEndError {
    fn from(_: MalformedCommandError) -> FrontEndError {
        FrontEndError::DebuggerCommand(MalformedCommandError)
    }
}

fn print_memory_page(num: u16, mem: Vec<u8>) {
    println!("\tMemory at page {:04x}...\n", num);
    let mut current_row: u32 = num as u32 & 0xff00;
    println!("\t      0  1  2  3  4  5  6  7  8  9  a  b  c  d  e  f");
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

fn print_cpu_state(reg: Registers) {
    println!("\tCPU state...\n");
    println!("\tPC:\t{:04x}", reg.pc);
    println!("\tSP:\t{:02x}", reg.sp);
    println!("\tA:\t{:02x}", reg.acc);
    println!("\tX:\t{:02x}", reg.x);
    println!("\tY:\t{:02x}", reg.y);
    println!("\tC  Z  I  D  B  V  N");
    println!("\t{}  {}  {}  {}  {}  {}  {}", 
        reg.status.carry as u8,
        reg.status.zero as u8,
        reg.status.interrupt as u8,
        reg.status.decimal as u8,
        reg.status.brk as u8,
        reg.status.overflow as u8,
        reg.status.negative as u8
    );
}

#[derive(Debug)]
pub struct MalformedCommandError;

impl From<num::ParseIntError> for MalformedCommandError {
    fn from(_ : num::ParseIntError) -> MalformedCommandError {
        MalformedCommandError
    }
}

impl From<MemoryLocationParseError> for MalformedCommandError {
    fn from(_: MemoryLocationParseError) -> MalformedCommandError {
        MalformedCommandError
    }
}

fn process_cmd(s: &str) -> Result<DebuggerCmd, MalformedCommandError> {
    let cmd = {
        if s.starts_with("next") || s.starts_with("n ") || s == "n" {
            let num = s.split(" ").nth(1)
                .map_or_else(|| Ok(1), |s| s.parse::<u32>())?;

            DebuggerCmd::Step(num)
        }
        else if s.starts_with("page") {
            let mut parts = s.split(" ");
            let loc = parts.nth(1)
                .ok_or_else(|| MalformedCommandError)?
                .parse::<MemoryLocation>()?;

            DebuggerCmd::RequestPage(*loc as u8)
        }
        else if s.starts_with("break") {
            let loc = s.split(" ").nth(1)
                .ok_or_else(|| MalformedCommandError)?
                .parse::<MemoryLocation>()?;

            DebuggerCmd::BreakPoint(*loc as u16)
        }
        else if s.starts_with("print") || s.starts_with("p ") {
            let num = s.split(" ").nth(1)
                .map_or_else(|| Ok(1), |s| s.parse::<u32>())?;

            DebuggerCmd::Print(num)
        }
        else if s == "cpu" {
            DebuggerCmd::RequestCpuState
        }
        else {
            return Err(MalformedCommandError);
        }
    };

    Ok(cmd)
}

static CHILD_STDIN_PTR: AtomicUsize = ATOMIC_USIZE_INIT;

fn get_child_stdin<'a>() -> &'a mut process::ChildStdin {
    use std::mem::transmute;
    use std::sync::atomic::Ordering::*;

    match CHILD_STDIN_PTR.load(SeqCst) {
        0 => panic!("CHILD_STDIN_PTR is zero"),
        n => unsafe { &mut *(transmute::<usize, *mut process::ChildStdin>(n)) }
    }
}

unsafe extern "system" fn ctrl_c_handler(ctrl_type: signal::DWORD) -> signal::BOOL {

    if ctrl_type == signal::CTRL_C_EVENT {
        let cmd = DebuggerCmd::Step(1);
        cmd.into_debugger_message(get_child_stdin()).ok();
        return signal::TRUE
    }

    signal::FALSE
}

fn handle_signal<F, T>(writer: &mut process::ChildStdin, mut f: F) -> T
    where F: FnMut() -> T
{
    use std::sync::atomic::Ordering::*;

    struct Reset(*const usize);

    impl Drop for Reset {
        fn drop(&mut self) {
            unsafe { signal::SetConsoleCtrlHandler(Some(ctrl_c_handler), signal::FALSE); };
            CHILD_STDIN_PTR.store(self.0 as usize, SeqCst);
        }
    }

    let other = CHILD_STDIN_PTR.swap(writer as *mut process::ChildStdin as usize, Relaxed);
    let _reset = Reset(other as *const usize);
    unsafe { signal::SetConsoleCtrlHandler(Some(ctrl_c_handler), signal::TRUE) };
    f()
}

fn process_debugger_messages<R: Read>(mut reader: R, writer: &mut process::ChildStdin) {

    use self::DebuggerResponse::*;

    handle_signal(writer, || {
        let mut is_stream = false;
        while let Ok(msg) = DebuggerResponse::from_debugger_message(&mut reader) {
            match msg {
                Message(msg) => writeln!(io::stdout(), "\t{}", msg).unwrap(),
                Page(num, mem) => print_memory_page(num, mem),
                CpuState(reg) => print_cpu_state(reg),
                StreamStart => is_stream = true,
                StreamEnd => is_stream = false,
                _ => {}
            }

            io::stdout().flush().unwrap();

            if !is_stream {
                break;
            }
        }
    });
}

pub struct FrontEnd<'a>(&'a [String]);

impl<'a> FrontEnd<'a> {
    pub fn with_args(args: &'a [String]) -> FrontEnd<'a> {
        FrontEnd(args)
    }

    pub fn run(self) -> Result<(), FrontEndError> {
        unsafe { signal::SetConsoleCtrlHandler(None, signal::TRUE) };

        let mut child = process::Command::new(&self.0[0])
            .args(&["--attach", &self.0[1], &self.0[2]])
            .stdout(process::Stdio::piped())
            .stdin(process::Stdio::piped())
            .stderr(process::Stdio::inherit())
            .spawn()?;

        unsafe { signal::SetConsoleCtrlHandler(None, signal::FALSE) };

        let mut input_buffer = String::with_capacity(64);

        println!("Staring debugger...");
        process_debugger_messages(
            child.stdout.as_mut().unwrap(),
            child.stdin.as_mut().unwrap()
        );

        io::stdout().flush()?;

        loop {
            write!(io::stdout(), "bbc-em> ")?;
            io::stdout().flush()?;

            input_buffer.clear();
            io::stdin().read_line(&mut input_buffer)?;
            let end_pos = input_buffer.bytes()
                                      .position(|c| c == b'\r' || c == b'\n')
                                      .unwrap_or_else(|| input_buffer.len());
            let msg = {
                let s = &input_buffer[..end_pos];

                match s {
                    "continue" | "c" => DebuggerCmd::Continue,
                    "quit" => break,
                    _ => {
                        if let Ok(cmd) = process_cmd(s) {
                            cmd
                        }
                        else {
                            println!("Unknown or invalid command: {}", s);
                            continue;
                        }
                    }
                }
            };

            msg.into_debugger_message(child.stdin.as_mut().unwrap())?;
            child.stdin.as_mut().unwrap().flush()?;
            process_debugger_messages(
                child.stdout.as_mut().unwrap(),
                child.stdin.as_mut().unwrap()
            );
        }

        child.kill()?;

        Ok(())
    }
}
