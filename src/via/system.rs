use cpu::Cpu;
use memory::{MemoryMap, AsMemoryRegionMut};

const MHZ: usize = 2_000_000;
const CYCLES_PER_MS: usize = MHZ / 1_000;
const TIMER_FREQ: usize = CYCLES_PER_MS * 20;

pub struct System {
    cycles_elapsed: usize,
    timer_count: usize,
    keys_down: Vec<u32>,
    kb_write: bool,
    port_a_ddr: u8,
    port_a_io: u8,
    port_b_ddr: u8,
    port_b_io: u8,
    interrupt_flags: u8,
    interrupt_enable: u8,
}

impl System {
    pub fn new() -> System {
        System {
            cycles_elapsed: 0,
            timer_count: 0,
            keys_down: vec![],
            kb_write: true,
            port_a_ddr: 0,
            port_a_io: 0,
            port_b_ddr: 0,
            port_b_io: 0,
            interrupt_flags: 0,
            interrupt_enable: 0,
        }
    }

    fn is_key_down(&self, rowcol: u8) -> bool {
        rowcol == 0x37
    }

    pub fn step<M, F>(&mut self, cycles: usize, mut mem: M, mut interrup_request: F)
        where M: MemoryMap + AsMemoryRegionMut,
              F: FnMut()
    {
        self.cycles_elapsed = self.cycles_elapsed.wrapping_add(cycles);
        self.timer_count += cycles;

        // Store any applicable register writes / reads
        match mem.last_hw_read() {
            Some(0xfe4d) => self.interrupt_flags = 0,
            _ => {}
        }

        match mem.last_hw_write() {
            Some((0xfe40, val)) => {
                self.port_b_io = val;
                match (val & 0x03) {
                    3 => self.kb_write = 0x40 == (val & 0x40),
                    _ => {}
                }
                self.interrupt_flags &= !0x18;
            },
            Some((0xfe41, val)) | Some((0xfe4f, val)) => {
                if !self.kb_write && self.is_key_down(val & self.port_a_ddr) {
                    self.port_a_io = 0x80;
                }
                else {
                    self.port_a_io = val;
                }
                self.interrupt_flags &= !0x03;
            },
            Some((0xfe42, val)) => self.port_b_ddr = val,
            Some((0xfe43, val)) => self.port_a_ddr = val,
            Some((0xfe4d, val)) => self.write_ifr(val),
            Some((0xfe4e, val)) => self.write_ier(val),
            _ => {}
        }

//        if 1 == (self.cycles_elapsed & 0x01) {
//            // VIA is on a 1MHz bus, so only step every other cpu cycle
//            return;
//        }

        let mut irq = false;
        if self.keys_down.len() > 0 && 0x01 == (self.read_ier() & 0x01) {
            self.interrupt_flags |= 0x81;
            self.interrupt_enable |= 0x01;
            irq = true;
        }

        if self.timer_count >= TIMER_FREQ && 0x20 == (self.read_ier() & 0x20) {
            self.interrupt_flags |= 0xa0;
            self.timer_count -= TIMER_FREQ;
            irq = true;
        }

        if irq {
            interrup_request();
        }

        self.interrupt_enable |= 0x80;

        let r = &mut *mem.region_mut(0xfe40..0xfe50)
                         .unwrap_or_else(|e| e.0);

        r[0] = self.port_b_io & !self.port_b_ddr;
        r[1] = self.port_a_io & !self.port_a_ddr;
        r[2] = self.port_b_ddr;
        r[3] = self.port_a_ddr;
        r[13] = self.interrupt_flags;
        r[14] = self.interrupt_enable;
        r[15] = self.port_a_io & !self.port_a_ddr;

    }

    pub fn keydown(&mut self, keynum: u32) {
        if let None = self.keys_down.iter()
                                    .find(|&k| *k == keynum)
        {
            self.keys_down.push(keynum);
//            self.interrupt_flags |= 0x01;
//            self.interrupt_enable |= 0x01;
        }
    }

    pub fn keyup(&mut self, keynum: u32) {
        self.keys_down.iter()
                      .position(|&k| k == keynum)
                      .map(|i| self.keys_down.remove(i));
    }

    fn write_ifr(&mut self, val: u8) {
        self.interrupt_flags &= !(val & 0x7f);
        if (self.interrupt_flags & 0x7f) != 0 {
            self.interrupt_flags |= 0x80;
        }
    }

    fn read_ifr(&mut self) -> u8 {
        let val = self.interrupt_flags;
        self.interrupt_flags = 0;
        val
    }

    fn write_ier(&mut self, val: u8) {
        match (val & 0x80) {
            0x80 => self.interrupt_enable |= (val & 0x7f) | 0x80,
            0x00 => self.interrupt_enable &= !(val & 0x7f) & 0x7f,
            _ => {}
        }
    }

    fn read_ier(&self) -> u8 {
        0x80 | (self.interrupt_enable & 0x7f)
    }
}

#[cfg(test)]
mod system_via_should {
    use super::*;

    #[test]
    fn clear_the_appropriate_ifr_bits() {
        
        let mut via = System::new();
        via.interrupt_flags = 0x23;

        via.write_ifr(0xa1);
        assert_eq!(0x82, via.interrupt_flags);
    }

    #[test]
    fn clear_the_ifr_on_read() {
        let mut via = System::new();
        via.interrupt_flags = 0xff;
        let _ = via.read_ifr();
        assert_eq!(0, via.interrupt_flags);
    }

    #[test]
    fn clear_the_correct_ier_bits() {
        let mut via = System::new();
        via.interrupt_enable = 0x83;
        via.write_ier(0x02);
        assert_eq!(0x01, via.interrupt_enable);

        via.write_ier(0xa2);
        assert_eq!(0xa3, via.interrupt_enable);

        via.interrupt_enable = 0x01;
        via.write_ier(0x7f);
        assert_eq!(0, via.interrupt_enable);
    }
}

