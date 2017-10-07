use std::ops::{Deref, DerefMut};
use std::convert::{AsRef, AsMut};
use std::slice::{Iter, IterMut};

#[derive(Debug)]
pub struct Region<'a>(pub(crate) &'a [u8]);
#[derive(Debug)]
pub struct RegionMut<'a>(pub(crate) &'a mut [u8]);

impl<'a> Region<'a> {
    fn iter(&'a self) -> Iter<'a, u8> {
        self.0.iter()
    }
}

impl<'a> RegionMut<'a> {
    fn iter(&'a self) -> Iter<'a, u8> {
        self.0.iter()
    }

    fn iter_mut(&'a mut self) -> IterMut<'a, u8> {
        self.0.iter_mut()
    }
}

impl<'a> Deref for Region<'a> {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a> Deref for RegionMut<'a> {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a> DerefMut for RegionMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<'a> AsRef<[u8]> for Region<'a> {
    fn as_ref(&self) -> &[u8] {
        self.0
    }
}

impl<'a> AsRef<[u8]> for RegionMut<'a> {
    fn as_ref(&self) -> &[u8] {
        self.0
    }
}

impl<'a> AsMut<[u8]> for RegionMut<'a> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0
    }
}

