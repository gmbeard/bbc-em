#[macro_use] extern crate log;

macro_rules! bit_is_set {
    ($field:expr, $bit:expr) => {{
        use std::mem;
        if ($bit as usize) >= (mem::size_of_val(&$field) * 8) {
            panic!("Attempting to check bit {} in a {} bit type. Only bits 0-{} are valid",
                $bit, 
                mem::size_of_val(&$field) * 8,
                mem::size_of_val(&$field) * 8 -1);
        }
        0x01 == ($field.rotate_right($bit as u32) & 0x01)
    }}
}

macro_rules! log_cpu {
    ($fmt:expr, $($params:expr),+) => {{
        #[cfg(feature="cpu-logging")]
        debug!($fmt, $($params),*);
    }};
    ($fmt:expr) => {{
        #[cfg(feature="cpu-logging")]
        debug!($fmt);
    }};
}

macro_rules! log_mem {
    ($fmt:expr, $($params:expr),+) => {{
        #[cfg(feature="memory-logging")]
        debug!($fmt, $($params),*);
    }};
    ($fmt:expr) => {{
        #[cfg(feature="memory-logging")]
        debug!($fmt);
    }};
}

macro_rules! log_video {
    ($fmt:expr, $($params:tt),+) => {{
        #[cfg(feature="video-logging")]
        debug!("Video: {}", format!($fmt, $($params),*));
    }};
    ($fmt:expr) => {{
        #[cfg(feature="video-logging")]
        debug!("Video: {}", format!($fmt));
    }};
}

macro_rules! log_via {
    ($fmt:expr, $($params:expr),+) => {{
        #[cfg(feature="via-logging")]
        debug!("System VIA: {}", format!($fmt, $($params),*));
    }};
    ($fmt:expr) => {{
        #[cfg(feature="via-logging")]
        debug!("System VIA: {}", format!($fmt));
    }};
}

pub mod cpu;
pub mod timer;
pub mod emulator;
pub mod debugger;
pub mod memory;
pub mod video;
pub mod via;

