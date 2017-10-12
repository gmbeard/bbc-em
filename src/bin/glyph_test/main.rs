extern crate glyphs;
extern crate minifb;

use std::thread;
use std::time::Duration;

use minifb::{Window, WindowOptions, Key};

fn fill_buffer(g: &[Vec<u8>], b: &mut [u32], width: usize, height: usize) {

    let mut i = g.iter();
    let max_per_row = width / 8;
    let max_rows = height / 20;

    for row in 0..max_rows {
        for col in 0..max_per_row-1 {
            match i.next() {
                Some(g) => {
                    for (y, scanline) in g.iter().enumerate() {
                        for (x, pixel) in glyphs::expand_byte_to_u32_array(*scanline).iter().enumerate() {
                            b[((width * row * 20) + (y * width)) + x + (col * 8)] = *pixel;
                        }
                    }
                },
                None => { return; }
            }
        }
    }
}

fn main() {
    const WIDTH: usize = 640;
    const HEIGHT: usize = 480;

    let mut window = Window::new("glyph test",
                                 WIDTH,
                                 HEIGHT,
                                 WindowOptions::default()).unwrap();
    let mut fb = vec![0; WIDTH * HEIGHT];

    let glyphs = (0..100).filter_map(|n| glyphs::glyph_expand_rows(n))
                         .map(|n| n.collect::<Vec<_>>())
                         .collect::<Vec<_>>();

    fill_buffer(&glyphs, &mut fb, WIDTH, HEIGHT);

    while window.is_open() { 
        window.update_with_buffer(&fb).unwrap();

        thread::sleep(Duration::from_millis(30));
    }
}
