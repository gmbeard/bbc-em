use std::io;
use cpu::{CpuError};

#[derive(Debug)]
pub enum DebuggerError {
    Cpu(CpuError),
    Protocol,
    Io(io::Error)
}

impl From<CpuError> for DebuggerError {
    fn from(e: CpuError) -> DebuggerError {
        DebuggerError::Cpu(e)
    }
}

impl From<io::Error> for DebuggerError {
    fn from(e: io::Error) -> DebuggerError {
        DebuggerError::Io(e)
    }
}

