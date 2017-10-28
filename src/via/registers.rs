use via::interrupts::*;
use via::peripheral_port::*;

macro_rules! create_key_map {
    ( $( $platform_num:expr => [$row:expr, $col:expr] ),+, ) => {
        const KEY_MAP: &'static [(usize, [usize; 2])] = &[
            $(
                ($platform_num, [$row, $col])
            ),*
        ];
    };
}

create_key_map! {
    10 => [4, 7], // A
    11 => [4, 1], // B
    12 => [6, 4], // C
    13 => [3, 2], // D
    14 => [2, 2], // E
    15 => [4, 3], // F
    16 => [5, 3], // G
    17 => [5, 4], // H
    18 => [2, 6], // I
    19 => [4, 5], // J
    20 => [4, 6], // K
    21 => [5, 6], // L
    22 => [6, 5], // M
    23 => [5, 5], // N
    24 => [3, 6], // O
    25 => [3, 7], // P
    26 => [1, 0], // Q
    27 => [3, 3], // R
    28 => [5, 1], // S
    29 => [2, 3], // T
    30 => [3, 5], // U
    31 => [6, 3], // V
    32 => [2, 1], // W
    33 => [4, 2], // X
    34 => [4, 4], // Y
    35 => [6, 1], // Z
}

#[derive(Default)]
struct KeyboardBuffer {
    buffer: [u32; 16],
    next: usize,
    write_enabled: bool,
}

impl KeyboardBuffer {
    fn key_down(&mut self, keynum: u32) {
        self.buffer[self.next] = keynum;
        self.next = (self.next + 1) & 0x0f;
    }

    fn clear(&mut self) {
        self.buffer = [0; 16];
        self.next = 0;
    }

    fn len(&self) -> usize {
        self.next
    }

    fn is_emulated_key_down(&self, rowcol: u8) -> bool {
        let int_keynum: [usize; 2] = [
            ((rowcol >> 4) & 0x07) as usize,
            (rowcol & 0x0f) as usize
        ];
        
        KEY_MAP.iter()
               .filter(|k| self.buffer[..self.next]
                               .iter()
                               .find(|&b| k.0 == *b as usize)
                               .is_some())
               .find(|&k| int_keynum == k.1)
               .is_some()
    }
}

const SOUND_IC_LATCH: usize = 0;
const SPEECH_READ_IC_LATCH: usize = 1;
const SPEECH_WRITE_IC_LATCH: usize = 2;
const KEYBOARD_IC_LATCH: usize = 3;
const HW_SCROLL_LOW_LATCH: usize = 4;
const HW_SCROLL_HIGH_LATCH: usize = 5;
const CAPS_LATCH: usize = 6;
const SHIFT_LATCH: usize =7;

#[derive(Default)]
pub struct Registers {
    pa1: PeripheralPort,
    pb: PeripheralPort,
    pub interrupts: Interrupts,
    pa2: PeripheralPort,
    keyboard_buffer: KeyboardBuffer,
    latches: [bool; 8],
}

fn check_len(mem: &[u8]) {
    if mem.len() < 16 {
        panic!("Memory region is smaller that 16 bytes");
    }
}

impl Registers {
    fn is_keyboard_write_enabled(&self) -> bool {
        self.latches[KEYBOARD_IC_LATCH]
    }
}

impl Registers {
    pub fn new() -> Registers {
        Registers::default()
    }

    pub fn write_port_a1_io(&mut self, val: u8) {
        log_via!("Wrote {:02x} to peripheral port a /w handshake", val);
    }

    pub fn write_port_a2_io(&mut self, val: u8) {
        self.pa2.write(val);
        if self.keyboard_buffer.len() > 0 {
            self.interrupts.signal_one(InterruptType::Keyboard);
        }

        if !self.is_keyboard_write_enabled() && 
            !self.keyboard_buffer.is_emulated_key_down(u8::from(self.pa2.io()))
        {
            self.pa2.set_io(Io(val & 0x7f));
        }
    }

    pub fn write_port_b_io(&mut self, val: u8) {
        self.pb.write(val);
        match (val & 0x03, bit_is_set!(val, 3)) {
            (0, f) => {
                log_via!("Sound write enable latch set to {}", f);
                self.latches[SOUND_IC_LATCH] = f;
            },
            (1, f) => {
                log_via!("Speech read enable latch");
                self.latches[SPEECH_READ_IC_LATCH] = f;
            },
            (2, f) => {
                log_via!("Speech write enable latch");
                self.latches[SPEECH_WRITE_IC_LATCH] = f;
            },
            (3, f) => {
                log_via!("Keyboard write enable latch set to {}", f);
                self.latches[KEYBOARD_IC_LATCH] = f;
//                self.keyboard_buffer.write_enable(f);
                //  TODO:
                //  Do we have to rescan the keyboard each time we latch the 
                //  KB IC? Do we have to rescan when we latch *any* IC?
            },
            _ => panic!("Invalid IC ({}) latched!", val)
        }
    }

    pub fn set_port_a_ddr(&mut self, val: u8) {
        self.pa1.set_data_direction(val);
        self.pa2.set_data_direction(val);
    }

    pub fn set_port_b_ddr(&mut self, val: u8) {
        self.pb.set_data_direction(val);
    }

    pub fn write_to(&self, mem: &mut [u8]) {
        check_len(mem);
        mem[0] = self.pb.read();
        mem[2] = self.pb.data_direction().into();
        mem[1] = self.pa1.read();
        mem[3] = self.pa1.data_direction().into();
        // ...
        mem[13] = self.interrupts.flags().into();
        mem[14] = self.interrupts.enabled().into();
        mem[15] = self.pa2.read();
    }

    pub fn key_down(&mut self, keynum: u32) {
        log_via!("Keyboard interrupt signalled: {}", keynum);
        self.keyboard_buffer.key_down(keynum);
        self.interrupts.signal_one(InterruptType::Keyboard);
    }

    pub fn clear_keyboard_buffer(&mut self) {
        self.keyboard_buffer.clear();
    }
}

#[cfg(test)]
mod keyboard_should {
    use super::*;

    #[test]
    fn report_correct_key_is_down() {
        let mut kb = KeyboardBuffer::default();
        kb.key_down(16);

        assert!(kb.is_emulated_key_down(0x23));
    }
}
