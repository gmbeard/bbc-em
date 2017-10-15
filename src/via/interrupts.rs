use std::fmt::{self, Formatter, Display};

pub struct Interrupts {
    flags: u8,
    enabled: u8,
    signalled: u8,
}

#[derive(Clone, Copy)]
pub enum InterruptType {
    Keyboard = 0,
    VerticalSync = 1,
    Timer2 = 5,
    Timer1 = 6
}

impl Display for InterruptType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let s = match *self {
            InterruptType::Keyboard => "keyboard",
            InterruptType::VerticalSync => "v-sync",
            InterruptType::Timer2 => "timer2",
            InterruptType::Timer1 => "timer1",
        };

        write!(f, "{}", s)
    }
}

pub struct Enabled(pub(crate) u8);
pub struct Flags(pub(crate) u8);

impl Flags {
    pub fn iter(&self) -> InterruptIterator {
        InterruptIterator::new(self.0)
    }
}

impl Enabled {
    pub fn iter(&self) -> InterruptIterator {
        InterruptIterator::new(self.0)
    }
}

impl From<Enabled> for u8 {
    fn from(e: Enabled) -> u8 {
        e.0
    }
}

impl From<Flags> for u8 {
    fn from(f: Flags) -> u8 {
        f.0
    }
}

impl Default for Interrupts {
    fn default() -> Interrupts {
        Interrupts {
            flags: 0x00,
            enabled: 0x00,
            signalled: 0x00,
        }
    }
}

impl Interrupts {
    pub fn new(flags: Flags, enabled: Enabled) -> Interrupts {
        Interrupts {
            flags: flags.into(),
            enabled: enabled.into(),
            signalled: 0x00,
        }
    }

    pub fn enabled(&self) -> Enabled {
        Enabled(0x80 | (0x7f & self.enabled))
    }

    pub fn flags(&self) -> Flags {
        Flags(
            if self.flags & 0x7f > 0 { 
                0x80 | (self.flags & 0x7f)
            }
            else {
                0
            }
        )
    }

    pub fn active(&self) -> Flags {
        Flags(u8::from(self.flags()) & u8::from(self.enabled()))
    }

    pub fn drain_signalled(&mut self) -> Flags {
        let s = Flags(u8::from(self.signalled) & u8::from(self.enabled()));
        self.signalled = 0;
        s
    }

    pub fn is_signalled(&self, t: InterruptType) -> bool {
        bit_is_set!(self.flags, t as u32)
    }

    pub fn is_enabled(&self, t: InterruptType) -> bool {
        bit_is_set!(self.enabled, t as u32)
    }

    pub fn clear<'a, I>(&mut self, flags: I)
        where I: IntoIterator<Item=&'a InterruptType>
    {
        self.clear_flags(
            flags.into_iter()
                 .fold(Flags(0x00), |acc, &i| {
                    Flags(u8::from(acc) | (0x01 << i as u32))
                 })
        );
    }

    pub fn signal<'a, I>(&mut self, enable: I)
        where I: IntoIterator<Item=&'a InterruptType>
    {
        enable.into_iter()
              .map(|i| self.signal_one(*i))
              .count();
    }

    pub fn signal_one(&mut self, t: InterruptType) {
        self.signalled |= (0x01 << (t as u32 & 0x07));
        self.flags |= self.signalled;
    }

    pub fn set_enabled(&mut self, e: Enabled) {
        match (e.0 & 0x80) {
            0x80 => self.enabled |= (e.0 & 0x7f) | 0x80,
            0x00 => self.enabled &= !(e.0 & 0x7f) & 0x7f,
            _ => {}
        }
    }

    pub fn clear_flags(&mut self, f: Flags) {
        self.flags &= !(f.0 & 0x7f);
        if (self.flags & 0x7f) != 0 {
            self.flags |= 0x80;
        }
    }
}

pub struct InterruptIterator(u8, usize);

impl InterruptIterator {
    fn new(f: u8) -> InterruptIterator {
        InterruptIterator(f, 0)
    }
}

impl Iterator for InterruptIterator {
    type Item = InterruptType;

    fn next(&mut self) -> Option<Self::Item> {
        while self.1 < 7 {
            let inner = self.0;
            let next = inner.rotate_right(1);
            self.0 = next;
            self.1 += 1;

            if bit_is_set!(inner, 0) {
                match (self.1 - 1) {
                    0 => return Some(InterruptType::Keyboard),
                    1 => return Some(InterruptType::VerticalSync),
                    2 => continue,        
                    3 => continue,        
                    4 => continue,        
                    5 => return Some(InterruptType::Timer2),        
                    6 => return Some(InterruptType::Timer1),        
                    _ => unreachable!(),
                }
            }
        }

        None
    }
}
