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
        for i in 0..8_usize {
            let v = val.wrapping_shr(7-i as u32);
            if 0x01 == (v & 0x01) {
                fb[self.fb_offset + i] = 0xffffffff;
            }
            else {
                fb[self.fb_offset + i] = 0x00000000;
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

        let total_line_chars = self.registers[0] as usize;
        let displayed_line_chars = self.registers[1] as usize;
        let total_vert_lines = self.registers[4] as usize;
        let displayed_vert_lines = self.registers[6] as usize;
        let scan_lines_per_char = self.registers[9] as usize;

        if displayed_line_chars == 0 || displayed_vert_lines == 0 {
            return;
        }
        
        for _ in 0..cycles {
            let pos = (self.scanline_count + (self.horizontal_count * 8)) 
                + (displayed_line_chars * self.vertical_count * (scan_lines_per_char + 1));

            if self.horizontal_count == 0 && self.scanline_count == 0 && self.vertical_count == 1 {
                assert_eq!(screen_start + 0x0140, ((pos & 0xffff) as u16) + screen_start);
            }

            let screen_char = video_mem.as_ref()[pos]; 
            self.write_char_to_fb(
                screen_char, 
                fb
            );

            self.horizontal_count += 1;
            self.fb_offset += 8;

            if self.horizontal_count >= (displayed_line_chars - 1) {
                self.horizontal_count = 0;
                self.scanline_count += 1;
                self.fb_offset = fb.width * ((self.fb_offset + fb.width) / fb.width);

                if self.scanline_count >= (scan_lines_per_char + 1) {
                    self.vertical_count += 1;
                    self.scanline_count = 0;

                    if self.vertical_count >= displayed_vert_lines {
                        self.vertical_count = 0;
                        self.fb_offset = 0;
                    }
                }
            }
        }
    }
}
