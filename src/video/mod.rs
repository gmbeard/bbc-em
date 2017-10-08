pub mod framebuffer;
pub use self::framebuffer::FrameBuffer;

use memory::{MemoryMap, AsMemoryRegion};

pub struct Crtc6845 {
    registers: [u8; 18],
    selected_reg: Option<u8>,
    horizontal_count: usize,
    vertical_count: usize,
    scanline_count: usize,
    fb_offset: usize,
}

impl Crtc6845 {
    pub fn new() -> Crtc6845 {
        Crtc6845 {
            registers: [0x00; 18],
            selected_reg: None,
            horizontal_count: 0,
            vertical_count: 0,
            scanline_count: 0,
            fb_offset: 0,
        }
    }

    fn write_char_to_fb(&mut self, val: u8, fb: &mut [u32]) {
        for i in 0..8 {
            let v = val.wrapping_shr(7-i);
            if 0x01 == (v & 0x01) {
                fb[self.fb_offset + i as usize] = 0xffffffff;
            }
            else {
                fb[self.fb_offset + i as usize] = 0x00000000;
            }
        }
    }

    pub fn step<M>(&mut self, cycles: usize, mut mem: M, fb: &mut FrameBuffer) 
        where M: MemoryMap + AsMemoryRegion
    {
        match mem.last_hw_write() {
            Some((addr, val)) if addr == 0xfe00 => {
                self.selected_reg = Some(val);
            },
            Some((addr, val)) if addr == 0xfe01 => {
                self.registers[self.selected_reg.unwrap() as usize] = val;
                log_video!("6845 register {:02x} set to {:02x}", self.selected_reg.unwrap(), val);
            },
            _ => {}
        }

        let screen_start = ((self.registers[12] << 3) as u16) << 8 | self.registers[13] as u16;
        let video_mem = mem.region(screen_start as _..0x8000)
                           .unwrap_or_else(|e| e.0);

        //  Horiz. and vert. count = 0
        for _ in 0..cycles {
            let screen_char = video_mem.as_ref()[self.horizontal_count * (self.vertical_count + self.scanline_count)]; 
            self.write_char_to_fb(
                screen_char, 
                fb
            );

            self.horizontal_count += 1;
            self.fb_offset += 8;
            
            if self.horizontal_count >= self.registers[1] as _ {
                self.horizontal_count = 0;
                self.scanline_count += 1;
                self.fb_offset = fb.width * (self.scanline_count + self.vertical_count);
                if self.scanline_count >= self.registers[5] as _ {
                    self.scanline_count = 0;
                    self.vertical_count += 1;

                    if self.vertical_count >= self.registers[4] as _ {
                        self.vertical_count = 0;
                        self.fb_offset = 0;
                    }
                }
            }

            //  if horiz. count == horiz. sync pos. (R2); H-Sync
            //  if horiz. count outside the displayed char range; disable horiz. display
            //  if horiz. and vert. display enabled;
            //      Read a byte (char's current line) from video mem line 
            //      Write to framebuffer
            //  otherwise;
            //      Render blank
            //  if horiz. count == total horiz. count (R0);
            //      if scanline count == total scanlines per char (R5);
            //          new screen row
            //          scanline count = 0
            //  Bump horizontal count

        }
    }
}
