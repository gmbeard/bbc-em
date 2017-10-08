use std::ops::{Deref, DerefMut};

pub struct FrameBuffer {
    pub width: usize,
    pub height: usize,
    bytes: Vec<u32>,
}

impl FrameBuffer {
    pub fn new(width: usize, height: usize) -> FrameBuffer {
        FrameBuffer {
            width: width,
            height: height,
            bytes: vec![0; width * height]
        }
    }

    pub fn bytes(&self) -> &[u32] {
        &self.bytes
    }

    pub fn bytes_mut(&mut self) -> &mut [u32] {
        &mut self.bytes
    }
}

impl Deref for FrameBuffer {
    type Target = [u32];
    fn deref(&self) -> &Self::Target {
        self.bytes()
    }
}

impl DerefMut for FrameBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.bytes_mut()
    }
}

