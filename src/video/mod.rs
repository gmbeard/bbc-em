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
    video_addr: usize,
    video_line_addr: usize,
}

const OUTPUT_SCALE: usize = 2;

impl Crtc6845 {
    pub fn new() -> Crtc6845 {
        Crtc6845 {
            registers: [0x00; 18],
            selected_reg: None,
            horizontal_count: 0,
            vertical_count: 0,
            scanline_count: 0,
            fb_offset: 0,
            video_addr: 0,
            video_line_addr: 0,
        }
    }

    fn write_char_to_fb(&mut self, val: u8, fb: &mut [u32]) {
        for y in 0..OUTPUT_SCALE {
            for x in 0..OUTPUT_SCALE {
                for i in 0..8_usize {
                    let v = val.wrapping_shr(7-i as u32);
                    if 0x01 == (v & 0x01) {
                        fb[(640 * y) + self.fb_offset + i + x] = 0xffffffff;
                    }
                    else {
                        fb[(640 * y) + self.fb_offset + i + x] = 0x00000000;
                    }
                }
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
            let screen_char = video_mem.as_ref()[self.video_addr]; 
            self.write_char_to_fb(
                screen_char, 
                fb
            );

            self.horizontal_count += 1;
            self.video_addr += 8;
            self.fb_offset += 8;
            
            if self.horizontal_count >= (self.registers[1] as usize + 1) {
                self.horizontal_count = 0;
                self.scanline_count += 1;
                self.video_line_addr += 1;
                self.video_addr = self.video_line_addr;
                self.fb_offset = fb.width * OUTPUT_SCALE * (self.scanline_count + self.vertical_count);

                if self.scanline_count >= (self.registers[5] as usize + 1) {
                    self.video_line_addr = self.video_addr;
                    self.scanline_count = 0;
                    self.vertical_count += 1;

                    if self.vertical_count >= (self.registers[4] as usize + 1) {
                        self.video_line_addr = 0;
                        self.video_addr = 0;
                        self.vertical_count = 0;
                        self.fb_offset = 0;
                    }
                }
            }
        }
    }
}
