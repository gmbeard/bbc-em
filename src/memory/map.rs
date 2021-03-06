use std::u16;
use std::ops::{Range, RangeFrom, RangeTo};
use std::io;

use memory::region::{Region, RegionMut};

const MEM_SIZE: usize = u16::MAX as usize + 1;

pub struct Map {
    bytes: Vec<u8>,
    last_hw_write: Option<(u16, u8)>,
    last_hw_read: Option<u16>,
    hw_ranges: Vec<Range<usize>>,
    paged_roms: Vec<Vec<u8>>,
    current_paged_rom: Option<usize>,
}

#[derive(Debug)]
pub struct RawAccessToHardwareError<T>(pub T);

fn ranges_overlap<T>(section: Range<T>, rhs: &Range<T>) -> bool 
    where T: PartialOrd
{
    (section.start >= rhs.start && section.start < rhs.end) ||
    (section.end > rhs.start && section.end <= rhs.end) ||
    (rhs.start <= section.start && rhs.end >= section.end) ||
    (rhs.start >= section.start && rhs.end <= section.end)
}

fn value_within_range<T>(val: T, range: &Range<T>) -> bool
    where T: PartialOrd
{
    (val >= range.start && val <= range.end)
}

pub trait MemoryMap {
    fn last_hw_read(&self) -> Option<u16>;
    fn last_hw_write(&self) -> Option<(u16, u8)>;
    fn write(&mut self, loc: u16, val: u8);
    fn read(&mut self, loc: u16) -> u8;
    fn clear_last_hw_access(&mut self);
}

pub trait AsMemoryRegionMut : AsMemoryRegion {
    fn region_mut<'a>(&'a mut self, range: Range<usize>)
        -> Result<RegionMut<'a>, RawAccessToHardwareError<RegionMut<'a>>>;

    fn region_from_mut<'a>(&'a mut self, range: RangeFrom<usize>)
        -> Result<RegionMut<'a>, RawAccessToHardwareError<RegionMut<'a>>>
    {
        let len = self.len();
        self.region_mut(range.start..len)
    }

    fn region_to_mut<'a>(&'a mut self, range: RangeTo<usize>)
        -> Result<RegionMut<'a>, RawAccessToHardwareError<RegionMut<'a>>>
    {
        self.region_mut(0..range.end)
    }
}

pub trait AsMemoryRegion {
    fn len(&self) -> usize;

    fn region<'a>(&'a self, range: Range<usize>) 
        -> Result<Region<'a>, RawAccessToHardwareError<Region<'a>>>;

    fn region_from<'a>(&'a self, range: RangeFrom<usize>)
        -> Result<Region<'a>, RawAccessToHardwareError<Region<'a>>>
    {
        let len = self.len();
        self.region(range.start..len)
    }

    fn region_to<'a>(&'a self, range: RangeTo<usize>)
        -> Result<Region<'a>, RawAccessToHardwareError<Region<'a>>>
    {
        self.region(0..range.end)
    }
}

impl<'a, T> MemoryMap for &'a mut T
    where T: MemoryMap
{    
    fn last_hw_read(&self) -> Option<u16> {
        T::last_hw_read(self)
    }

    fn last_hw_write(&self) -> Option<(u16, u8)> {
        T::last_hw_write(self)
    }

    /// Panics if `loc` is greater than `u16::MAX + 1`
    fn write(&mut self, loc: u16, val: u8) {
        T::write(self, loc, val)
    }

    /// Panics if `loc` is greater than `u16::MAX + 1`.
    ///
    /// This function requires `&mut self` because reading can potentially
    /// have side effects, such as clearing hardware registers, etc.
    fn read(&mut self, loc: u16) -> u8 {
        T::read(self, loc)
    }

    fn clear_last_hw_access(&mut self) {
        T::clear_last_hw_access(self);
    }
}

impl<'a, T> AsMemoryRegion for &'a T
    where T: AsMemoryRegion
{
    fn len(&self) -> usize {
        T::len(self)
    }

    fn region<'b>(&'b self, range: Range<usize>) 
        -> Result<Region<'b>, RawAccessToHardwareError<Region<'b>>> 
    {
        T::region(self, range)
    }
}

impl<'a, T> AsMemoryRegion for &'a mut T
    where T: AsMemoryRegion
{
    fn len(&self) -> usize {
        T::len(self)
    }

    fn region<'b>(&'b self, range: Range<usize>) 
        -> Result<Region<'b>, RawAccessToHardwareError<Region<'b>>> 
    {
        T::region(self, range)
    }
}

impl<'a, T> AsMemoryRegionMut for &'a mut T
    where T: AsMemoryRegionMut
{
    fn region_mut<'b>(&'b mut self, range: Range<usize>) 
        -> Result<RegionMut<'b>, RawAccessToHardwareError<RegionMut<'b>>> 
    {
        T::region_mut(self, range)
    }
}

const PAGED_ROM_REGISTER: u16 = 0xfe30;
const PAGED_ROM_MEMORY_RANGE: Range<usize> = 0x8000..0xc000;

impl Map {
    pub fn new() -> Map {
        Map {
            bytes: vec![0; MEM_SIZE],
            last_hw_write: None,
            last_hw_read: None,
            hw_ranges: vec![],
            paged_roms: vec![],
            current_paged_rom: None,
        }
    }

    pub fn add_paged_rom(&mut self, rom: Vec<u8>) {
        self.paged_roms.push(rom)
    }

    pub fn with_hw_range(mut self, range: Range<usize>) -> Map
    {
        self.hw_ranges.push(range);
        self
    }

    pub fn with_hw_ranges<R>(mut self, ranges: R) -> Map
        where R: IntoIterator<Item=Range<usize>>
    {
        self.hw_ranges.extend(ranges.into_iter().collect::<Vec<_>>());
        self
    }

    fn switch_paged_rom_to(&mut self, num: usize) {
        if let None =  self.paged_roms.get(num) {
            return;
        }

        self.current_paged_rom.map(|n|{
            io::copy(
                &mut &self.bytes[PAGED_ROM_MEMORY_RANGE], 
                &mut &mut self.paged_roms[n][..]).unwrap();
        });

        io::copy(
            &mut &self.paged_roms[num][..], 
            &mut &mut self.bytes[PAGED_ROM_MEMORY_RANGE]).unwrap();
    }
}

impl MemoryMap for Map {
    fn last_hw_read(&self) -> Option<u16> {
        self.last_hw_read
    }

    fn last_hw_write(&self) -> Option<(u16, u8)> {
        self.last_hw_write
    }

    /// Panics if `loc` is greater than `u16::MAX + 1`
    fn write(&mut self, loc: u16, val: u8) {
        if loc == PAGED_ROM_REGISTER {
            self.switch_paged_rom_to(val as usize);
        }
        self.bytes[loc as usize] = val;
        self.last_hw_write = 
            self.hw_ranges.iter()
                          .find(|r| value_within_range(loc as usize, r))
                          .map(|_| {
                              log_mem!("HW Write {:02x} -> {:04x}", val, loc);
                              (loc, val)
                          })
                          .or_else(|| {
                              log_mem!("RAM Write {:02x} -> {:04x}", val, loc);
                              None
                          });
                               
    }

    /// Panics if `loc` is greater than `u16::MAX + 1`.
    ///
    /// This function requires `&mut self` because reading can potentially
    /// have side effects, such as clearing hardware registers, etc.
    fn read(&mut self, loc: u16) -> u8 {
        let val = self.bytes[loc as usize];
        self.last_hw_read = 
            self.hw_ranges.iter()
                          .find(|r| value_within_range(loc as usize, r))
                          .map(|_| {
                              log_mem!("HW Read {:02x} <- {:04x}", val, loc);
                              loc
                          })
                          .or_else(|| {
                              log_mem!("RAM Read {:02x} <- {:04x}", val, loc);
                              None
                          });
        val
    }

    fn clear_last_hw_access(&mut self) {
        self.last_hw_read = None;
        self.last_hw_write = None;
    }

}

impl AsMemoryRegion for Map {
    fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Allows raw, mutable access to the underlying memory region. 
    ///
    /// The function will return a `RawAccessToHardwareError(Region(..))` if the 
    /// requested region overlaps a hardware memory mapped region. However,
    /// The error response still contains the requested region. This serves
    /// to indicate to the caller that they're potentially accessing a 
    /// region of memory that would otherwise generate side effects
    fn region<'a>(&'a self, range: Range<usize>) 
        -> Result<Region<'a>, RawAccessToHardwareError<Region<'a>>> 
    {
        if self.hw_ranges.iter().any(|r| ranges_overlap(r.clone(), &range)) {
            Err(RawAccessToHardwareError(Region(&self.bytes[range])))
        }
        else {
            Ok(Region(&self.bytes[range]))
        }
    }
}

impl AsMemoryRegionMut for Map {
    /// Allows raw, immutable access to the underlying memory region. 
    ///
    /// The function will return a `RawAccessToHardwareError(Region(..))` if the 
    /// requested region overlaps a hardware memory mapped region. However,
    /// The error response still contains the requested region. This serves
    /// to indicate to the caller that they're potentially accessing a 
    /// region of memory that would otherwise generate side effects
    fn region_mut<'a>(&'a mut self, range: Range<usize>) 
        -> Result<RegionMut<'a>, RawAccessToHardwareError<RegionMut<'a>>>
    {
        if self.hw_ranges.iter().any(|r| ranges_overlap(r.clone(), &range)) {
            Err(RawAccessToHardwareError(RegionMut(&mut self.bytes[range])))
        }
        else {
            Ok(RegionMut(&mut self.bytes[range]))
        }

    }

}

#[cfg(test)]
mod map_should {
    use super::*;

    #[test]
    fn return_err_when_accessing_hw_region() {
        let mut map = Map::new()
            .with_hw_range(0xfe00 as usize..0xff00 as usize);

        assert!(map.region(0xfd04 as usize..0xfe25 as usize).is_err());
        assert!(map.region(0xfe04 as usize..0xff25 as usize).is_err());
        assert!(map.region(0xfd04 as usize..0xff25 as usize).is_err());
        assert!(map.region_from_mut(0xfedc as usize..).is_err());
        assert!(map.region_to_mut(..0xfedc).is_err());
    }

    #[test]
    fn return_ok_when_not_accessing_hw_region() {
        let map = Map::new()
            .with_hw_range(0xfe00 as usize..0xff00 as usize);

        assert!(map.region(0x0000 as usize..0x0300 as usize).is_ok());
        assert!(map.region_from(0xff00 as usize..).is_ok());
        assert!(map.region_to(..0xfe00).is_ok());
    }

    #[test]
    fn deref_to_mut_slice_for_range() {
        fn use_slice(_slice: &mut [u8]) { }
       
        let mut map = Map::new();
        use_slice(&mut map.region_to_mut(..0x0100).unwrap());
    }

    #[test]
    fn deref_to_slice_for_range() {
        fn use_slice(_slice: &[u8]) { }
       
        let map = Map::new();
        use_slice(&map.region_to(..0x0100).unwrap());
    }

    #[test]
    fn should_report_last_hw_read() {
        let mut map = Map::new()
            .with_hw_range(0xfe00 as usize..0xff00 as usize);

        let _ = map.read(0xfe40);
        assert_eq!(Some(0xfe40), map.last_hw_read());

        let _ = map.read(0x0100);
        assert_eq!(None, map.last_hw_read());
    }

    #[test]
    fn should_report_last_hw_write() {
        let mut map = Map::new()
            .with_hw_ranges(vec![0xfe00 as usize..0xff00 as usize]);

        map.write(0xfe40, 0xde);
        assert_eq!(Some((0xfe40, 0xde)), map.last_hw_write());

        map.write(0x0001, 0xde);
        assert_eq!(None, map.last_hw_write());
    }
}

