macro_rules! log_cpu {
    ($fmt:expr, $($params:expr),+) => {
        #[cfg(feature="cpu-logging")]
        eprintln!($fmt, $($params),*);
    };
    ($fmt:expr) => {
        #[cfg(feature="cpu-logging")]
        eprintln!($fmt);
    };
}

macro_rules! log_mem {
    ($fmt:expr, $($params:expr),+) => {
        #[cfg(feature="memory-logging")]
        eprintln!($fmt, $($params),*);
    };
    ($fmt:expr) => {
        #[cfg(feature="memory-logging")]
        eprintln!($fmt);
    };
}

pub mod cpu;
pub mod timer;
pub mod emulator;
pub mod debugger;
pub mod memory;

