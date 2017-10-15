extern crate glyphs;

use std::cmp;

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
    state: VideoState,
    video_control_reg: u8,
}

enum VideoState {
    NotInitialized,
    NewFrame(u16),                     // Screen start address
    DisplayingLine(u16, usize, usize, usize), // Line Addr, Line, Char, Scanline
    EndOfLine(u16, usize),                    // Line addr, Lines
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
            state: VideoState::NotInitialized,
            video_control_reg: 0,
        }
    }

//    fn write_char_to_fb(&mut self, val: u8, fb: &mut [u32]) {
//        for i in 0..8_usize {
//            let v = val.wrapping_shr(7-i as u32);
//            if 0x01 == (v & 0x01) {
//                fb[self.fb_offset + i] = 0xffffffff;
//            }
//            else {
//                fb[self.fb_offset + i] = 0x00000000;
//            }
//        }
//    }

    fn render_glyph<I>(&self, mut g: I, fb: &mut FrameBuffer, scanline: usize, x: usize, y: usize)
        where I: Iterator<Item=u8>
    {
        const MODE_7_HORIZ_TOTAL: usize = 40;
        const MODE_7_SCANLINES_PER_CHAR: usize = 19;

        let bytes = glyphs::expand_byte_to_u32_array(g.nth(scanline).unwrap());
        let output_x = x * 8;
        let output_y = (y * fb.width * MODE_7_SCANLINES_PER_CHAR) + 
            (scanline * fb.width);

        for n in 0..8 {
            fb[output_y + output_x + n] = bytes[n];
        }
    }

    fn render_char(&self, byte: u8, fb: &mut FrameBuffer, scanline: usize, x: usize, y: usize) {
        const SCANLINES_PER_CHAR: usize = 9;

        let bytes = glyphs::expand_byte_to_u32_array(byte);
        let output_x = x * 8;
        let output_y = (y * fb.width * (self.registers[SCANLINES_PER_CHAR] as usize + 1)) + 
            (scanline * fb.width);

        for n in 0..8 {
            fb[output_y + output_x + n] = bytes[n];
        }
    }

    fn is_teletext(&self) -> bool {
        bit_is_set!(self.video_control_reg, 1)
    }

    pub fn step<M>(&mut self, cycles: usize, mut mem: M, fb: &mut FrameBuffer) 
        where M: MemoryMap + AsMemoryRegion
    {
        const TOTAL_HORIZ: usize = 0;
        const TOTAL_HORIZ_DISP: usize = 1;
        const TOTAL_VERT: usize = 4;
        const TOTAL_VERT_DISP: usize = 6;
        const SCANLINES_PER_CHAR: usize = 9;
        const SCREEN_START_HI: usize = 12;
        const SCREEN_START_LO: usize = 13;

        match mem.last_hw_write() {
            Some((addr, val)) if addr == 0xfe00 => {
                self.selected_reg = Some(val);
            },
            Some((addr, val)) if addr == 0xfe01 => {
                self.registers[self.selected_reg.unwrap() as usize] = val;
                #[cfg(feature="video-logging")]
                match self.selected_reg {
                    Some(0) => log_video!("Horiz. total register set to {:02x}", val),
                    Some(1) => log_video!("Horiz. display register set to {:02x}", val),
                    Some(2) => log_video!("Horiz. sync position register set to {:02x}", val),
                    Some(3) => log_video!("Sync width register set to {:02x}", val),
                    Some(4) => log_video!("Vert. total register set to {:02x}", val),
                    Some(5) => log_video!("Vert. total adjust register set to {:02x}", val),
                    Some(6) => log_video!("Vert. display register set to {:02x}", val),
                    Some(7) => log_video!("Vert. sync position set to {:02x}", val),
                    Some(8) => log_video!("Interlace and delay register set to {:02x} ({:08b})", val, val),
                    Some(9) => log_video!("Scanlines per char register set to {:02x} ({:08b})", val, val),
                    Some(10) => log_video!("Cursor start register set to {:02x} ({:08b})", val, val),
                    Some(11) => log_video!("Cursor end register set to {:02x} ({:08b})", val, val),
                    Some(12) => log_video!("Screen start address high set to {:02x}", val),
                    Some(13) => log_video!("Screen start address low set to {:02x}", val),
                    Some(14) => log_video!("Cursor position high set to {:02x}", val),
                    Some(15) => log_video!("Cursor position low set to {:02x}", val),

                    _ => {},
                }
            },
            Some((addr, val)) if addr == 0xfe20 => {
                self.video_control_reg = val;
                log_video!("ULA: Video control register set to {:02x} ({:08b})", val, val);
            },
            Some((addr, val)) if addr == 0xfe21 => log_video!("ULA: Palette register set to {:02x} ({:08b})", val, val),
            _ => {}
        }

        fn calc_start_addr(hi: u8, lo: u8, teletext: bool) -> u16 {
            if teletext {
                let hi = (hi ^ 0x20) + 0x74;
                (hi as u16) << 8 | lo as u16
            }
            else {
                ((hi as u16) << 8 | lo as u16) << 3
            }
        }
        
        for _ in 0..cycles {
            loop {
                match self.state {
                    VideoState::NotInitialized => {
                        let start_addr = calc_start_addr(
                            self.registers[SCREEN_START_HI], 
                            self.registers[SCREEN_START_LO],
                            self.is_teletext()
                        );

                        if self.registers[TOTAL_HORIZ_DISP] == 0 ||
                            self.registers[TOTAL_VERT_DISP] == 0
                            || start_addr == 0
                        {
                            break;
                        }

                        self.state = VideoState::NewFrame(start_addr);

                        log_video!("Video: Latched start address {:04x}", start_addr); 
                    },
                    VideoState::NewFrame(start_addr) => {
                        self.state = VideoState::DisplayingLine(start_addr, 0, 0, 0);
                    },
                    VideoState::DisplayingLine(line_addr, l, c, sl) => {
                        if c >= self.registers[TOTAL_HORIZ_DISP] as _ {
                            if self.is_teletext() {
                                self.state = VideoState::DisplayingLine(line_addr, l, 0, sl + 1);
                            }
                            else {
                                self.state = VideoState::DisplayingLine(line_addr + 1, l, 0, sl + 1);
                            }
                            continue;
                        }

                        if sl >= (self.registers[SCANLINES_PER_CHAR] as usize + 1) {
                            let next_line_addr = {
                                if self.is_teletext() {
                                    line_addr + self.registers[TOTAL_HORIZ_DISP] as u16
                                }
                                else {
                                    line_addr + ((self.registers[TOTAL_HORIZ_DISP] as u16 - 1) * 8)
                                }
                            };

                            self.state = VideoState::EndOfLine(next_line_addr, l + 1);
                            continue;
                        }

                        self.state = VideoState::DisplayingLine(line_addr, l, c + 1, sl);

                        if line_addr < 0x8000 {
                            let video_mem = &*mem.region((line_addr as usize)..0x8000)
                                                 .unwrap_or_else(|e| e.0);

                            if self.is_teletext() {
                                let byte = video_mem[c];

                                match byte.checked_sub(0x20) {
                                    Some(v) =>  {
                                        if let Some(glyph) = glyphs::glyph_expand_rows(v as usize) {
                                            self.render_glyph(glyph, fb, sl, c, l);
                                        }
                                    },
                                    _ => {},
                                }
                            }
                            else {
                                let byte = video_mem[c * 8];
                                self.render_char(byte, fb, sl, c, l);
                            }
                        }

                        break;
                    },
                    VideoState::EndOfLine(next_line_addr, lines_displayed) => {
                        if lines_displayed >= self.registers[TOTAL_VERT_DISP] as _ {
                            let start_addr = calc_start_addr(
                                self.registers[SCREEN_START_HI], 
                                self.registers[SCREEN_START_LO],
                                self.is_teletext()
                            );

                            self.state = VideoState::NewFrame(start_addr);
                            continue;
                        }

                        self.state = VideoState::DisplayingLine(next_line_addr, lines_displayed, 0, 0);
                    },
                    _ => { break; }
                }
            }
        }
    }
}
