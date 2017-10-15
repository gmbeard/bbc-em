use cpu::Cpu;
use memory::{MemoryMap, AsMemoryRegionMut};
use via::registers::{Registers};
use via::interrupts::{Flags, Enabled, InterruptType};
use std::ops::Range;

const MHZ: usize = 2_000_000;
const CYCLES_PER_MS: usize = MHZ / 1_000;
const TIMER_FREQ: u64 = CYCLES_PER_MS as u64 * 200;

pub struct System {
    cycles_elapsed: u64,
    timer_count: u64,
    kb_write: bool,
    registers: Registers
}

const SYSTEM_VIA_REG_RANGE: Range<usize> = 0xfe40..0xfe50;
const SYSTEM_VIA_REG_START: u16 = 0xfe40;
const IFR_REGISTER: u16 = SYSTEM_VIA_REG_START | 0x0d;
const IER_REGISTER: u16 = SYSTEM_VIA_REG_START | 0x0e;
const PA_DDR_REG: u16 = SYSTEM_VIA_REG_START | 0x03;
const PB_DDR_REG: u16 = SYSTEM_VIA_REG_START | 0x02;
const PA1_IO_REG: u16 = SYSTEM_VIA_REG_START | 0x01;
const PA2_IO_REG: u16 = SYSTEM_VIA_REG_START | 0x0f;
const PB_IO_REG: u16 = SYSTEM_VIA_REG_START | 0x00;

impl System {
    pub fn new() -> System {
        System {
            cycles_elapsed: 0,
            timer_count: 0,
            kb_write: true,
            registers: Registers::new(),
        }
    }

    fn process_reads_and_writes<K>(&mut self, 
                                   read: Option<u16>, 
                                   write: Option<(u16, u8)>,
                                   key_eval: K)
        where K: Fn(u8) -> bool
    {

        // Store any applicable register writes / reads
//        match read {
//            Some(IFR_REGISTER) => {
//                self.registers.interrupts.clear_flags(Flags(0x7f));
//                log_via!("IFR read. Now {:08b}", u8::from(self.registers.interrupts.flags()));
//            }
//            _ => {}
//        }

        match write {
            Some((PB_IO_REG, val)) => {
                match val & 0x07 {
                    0 => log_via!("Set sound write enable to {:02x}", val & 0x08),
                    1 => log_via!("Set speech read select to {:02x}", val & 0x08),
                    2 => log_via!("Set speech write select to {:02x}", val & 0x08),
                    3 => {
                        log_via!("Set keyboard write enable to {:02x}", val & 0x08);
                        self.kb_write = (0x08 == (val & 0x08));
                    }
                    4 => log_via!("Set HW scrolling Low bit to {:02x}", val & 0x08),
                    5 => log_via!("Set HW scrolling High bit to {:02x}", val & 0x08),
                    6 => log_via!("Set CAPS lock LED to {:02x}", val & 0x08),
                    7 => log_via!("Set SHIFT lock LED to {:02x}", val & 0x08),
                    _ => {},
                }

                self.registers.write_port_b_io(val);
                self.registers.interrupts.clear_flags(Flags(0x18));
            },
            Some((PA1_IO_REG, val)) => {
                self.registers.write_port_a1_io(val);
                self.registers.interrupts.clear(&[
                    InterruptType::Keyboard, 
                    InterruptType::VerticalSync]);
            },
            Some((PA2_IO_REG, val)) => {
                self.registers.write_port_a2_io(val);
                self.registers.interrupts.clear(&[
                    InterruptType::Keyboard, 
                    InterruptType::VerticalSync]);
            },
            Some((PB_DDR_REG, val)) => {
                self.registers.set_port_b_ddr(val);
                log_via!("Port B Data direction register set to {:02x}", val);
            },
            Some((PA_DDR_REG, val)) => {
                self.registers.set_port_a_ddr(val);
                log_via!("Port A Data direction register set to {:02x}", val);
            },
            Some((IFR_REGISTER, val)) => {
                self.registers.interrupts.clear_flags(Flags(val));
                log_via!(
                    "Written {:08b} to IFR. Now {:08b}", 
                    val,
                    u8::from(self.registers.interrupts.flags()));
            },
            Some((IER_REGISTER, val)) => {
                self.registers.interrupts.set_enabled(Enabled(val));
                log_via!(
                    "Written {:08b} to IER. Now {:08b}", 
                    val, 
                    u8::from(self.registers.interrupts.enabled()));
            },
            _ => {}
        }
    }

    pub fn step<M, F, K>(&mut self, 
                         cycles: usize, 
                         mut mem: M, 
                         mut interrup_request: F,
                         key_eval: K)
        where M: MemoryMap + AsMemoryRegionMut,
              F: FnMut(),
              K: Fn(u8) -> bool
    {
        self.cycles_elapsed = self.cycles_elapsed.wrapping_add(cycles as _);
        self.timer_count += cycles as _;

        self.process_reads_and_writes(
            mem.last_hw_read(), 
            mem.last_hw_write(),
            key_eval);

        let mut irq = false;

        if self.timer_count >= TIMER_FREQ {
            self.registers.interrupts.signal_one(InterruptType::Timer1);
            self.registers.interrupts.signal_one(InterruptType::VerticalSync);
            self.timer_count -= TIMER_FREQ;
        }

        let signalled = 
            self.registers.interrupts.drain_signalled();


        if signalled.iter().count() > 0 {
            log_via!(
                "{} Active interrupt(s): {}", 
                signalled.iter()
                         .count(),
                signalled.iter()
                         .map(|i| format!("{}", i))
                         .collect::<Vec<_>>()
                         .as_slice()
                         .join(", ")
            );
            interrup_request();
        }

        self.registers.write_to(
            &mut mem.region_mut(SYSTEM_VIA_REG_RANGE)
                    .unwrap_or_else(|e| e.0));

    }

    pub fn keydown(&mut self, keynum: u32) {
        self.registers.key_down(keynum);
    }

    pub fn keyup(&mut self, keynum: u32) {
        //  TODO?
    }

//    fn write_ifr(&mut self, val: u8) {
//        self.interrupt_flags &= !(val & 0x7f);
//        if (self.interrupt_flags & 0x7f) != 0 {
//            self.interrupt_flags |= 0x80;
//        }
//    }
//
//    fn read_ifr(&mut self) -> u8 {
//        let val = self.interrupt_flags;
//        self.interrupt_flags = 0;
//        val
//    }
//
//    fn write_ier(&mut self, val: u8) {
//        match (val & 0x80) {
//            0x80 => self.interrupt_enable |= (val & 0x7f) | 0x80,
//            0x00 => self.interrupt_enable &= !(val & 0x7f) & 0x7f,
//            _ => {}
//        }
//    }
//
//    fn read_ier(&self) -> u8 {
//        0x80 | (self.interrupt_enable & 0x7f)
//    }
}

//#[cfg(test)]
//mod system_via_should {
//    use super::*;
//
//    #[test]
//    fn clear_the_appropriate_ifr_bits() {
//        
//        let mut via = System::new();
//        via.interrupt_flags = 0x23;
//
//        via.write_ifr(0xa1);
//        assert_eq!(0x82, via.interrupt_flags);
//    }
//
//    #[test]
//    fn clear_the_ifr_on_read() {
//        let mut via = System::new();
//        via.interrupt_flags = 0xff;
//        let _ = via.read_ifr();
//        assert_eq!(0, via.interrupt_flags);
//    }
//
//    #[test]
//    fn clear_the_correct_ier_bits() {
//        let mut via = System::new();
//        via.interrupt_enable = 0x83;
//        via.write_ier(0x02);
//        assert_eq!(0x01, via.interrupt_enable);
//
//        via.write_ier(0xa2);
//        assert_eq!(0xa3, via.interrupt_enable);
//
//        via.interrupt_enable = 0x01;
//        via.write_ier(0x7f);
//        assert_eq!(0, via.interrupt_enable);
//    }
//}

