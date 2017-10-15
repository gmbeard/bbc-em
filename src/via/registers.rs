use via::interrupts::*;
use via::peripheral_port::*;

#[derive(Default)]
struct KeyboardBuffer {
    buffer: [u32; 16],
    next: usize,
    write_enabled: bool,
}
//Q: [0, 1],
//W: [1, 2],
//E: [2, 2],
//R: [3, 3],
//T: [3, 2],
//Y: [4, 4],
//U: [5, 3],
//I: [5, 2],
//O: [6, 3],
//P: [7, 3],
//
//A: [1, 4],
//S: [1, 5],
//D: [2, 3],
//F: [3, 4],
//G: [3, 5],
//H: [4, 5],
//J: [5, 4],
//K: [6, 4],
//L: [6, 5],
//
//Z: [1, 6],
//X: [2, 4],
//C: [2, 5],
//V: [3, 6],
//B: [4, 6],
//N: [5, 5],
//M: [5, 6],

impl KeyboardBuffer {
    fn key_down(&mut self, keynum: u32) {
        self.buffer[self.next] = keynum;
        self.next = (self.next + 1) & 0x0f;
    }

    fn clear(&mut self) {
        self.buffer = [0; 16];
        self.next = 0;
    }

    fn is_emulated_key_down(&self, rowcol: u8) -> bool {
        //  TODO:
        let col = (rowcol >> 4) & 0x07;
        let row = rowcol & 0x0f;
        log_via!("Scanning keyboard row {:02x}, col {:02x}", 
            (rowcol >> 4) & 0x07,
            rowcol & 0x0f
        );
        row == 3 && col == 3
//        rowcol == 0x43
    }

    fn write_enable(&mut self, f: bool) {
        self.write_enabled = f;
    }

    fn is_write_enabled(&self) -> bool {
        self.write_enabled
    }
}

#[derive(Default)]
pub struct Registers {
    pa1: PeripheralPort,
    pb: PeripheralPort,
    pub interrupts: Interrupts,
    pa2: PeripheralPort,
    ic: SelectedIc,
    keyboard_buffer: KeyboardBuffer,
}

#[derive(Copy, Clone)]
pub enum SelectedIc {
    Keyboard(bool),
    SpeechWrite,
    SpeechRead,
    Sound(bool),
}

impl Default for SelectedIc {
    fn default() -> SelectedIc {
        SelectedIc::Keyboard(true)
    }
}

fn check_len(mem: &[u8]) {
    if mem.len() < 16 {
        panic!("Memory region is smaller that 16 bytes");
    }
}

impl Registers {
    pub fn new() -> Registers {
        Registers::default()
    }

    pub fn write_port_a1_io(&mut self, val: u8) {
        self.pa1.set_io(Io(val)); 
        if !self.keyboard_buffer.is_write_enabled() &&
            self.keyboard_buffer.is_emulated_key_down(val & 0x7f)
        {
            self.pa1.set_io(Io(0x80));
        }

//        match self.ic {
//            SelectedIc::Keyboard(false) => {
//                log_via!("Checking for key {:02x}", val);
//                if self.keyboard_buffer.is_emulated_key_down(val) {
//                    self.pa1.set_io(Io(0x80));
//                }
//            },
//            SelectedIc::Sound(true) => {
//                log_via!("Write {:02x} ({:08b}) to sound hw", val, val);
//            }
//            _ => {}
//        }
    }

    pub fn write_port_a2_io(&mut self, val: u8) {
        self.pa2.set_io(Io(val)); 
        if !self.keyboard_buffer.is_write_enabled() &&
            self.keyboard_buffer.is_emulated_key_down(val & 0x7f)
        {
            self.pa2.set_io(Io(0x80));
        }
    }

    pub fn write_port_b_io(&mut self, val: u8) {
        self.pb.write(val);
        match val & 0x03 {
            0 => {
                log_via!("Sound write enable latch set to {}", bit_is_set!(val, 3));
                self.ic = SelectedIc::Sound(bit_is_set!(val, 3));
            },
            1 => {
                log_via!("Speech read enable latch");
                self.ic = SelectedIc::SpeechRead;
            },
            2 => {
                log_via!("Speech write enable latch");
                self.ic = SelectedIc::SpeechWrite;
            },
            3 => {
                log_via!("Keyboard write enable latch set to {}", bit_is_set!(val, 3));
                let write_enabled = bit_is_set!(val, 3);
                self.ic = SelectedIc::Keyboard(write_enabled);
                self.keyboard_buffer.write_enable(write_enabled);
                //  TODO:
                //  Do we have to rescan the keyboard each time we latch the 
                //  KB IC? Do we have to rescan when we latch *any* IC?
            },
            _ => panic!("Invalid IC latched!")
        }
    }

    pub fn set_port_a_ddr(&mut self, val: u8) {
        self.pa1.set_data_direction(val);
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
