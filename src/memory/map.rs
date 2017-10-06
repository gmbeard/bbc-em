use std::u16;
use std::ops::{Range, RangeFrom, RangeTo};

use memory::region::{Region, RegionMut};

const MEM_SIZE: usize = u16::MAX as usize + 1;

pub struct Map {
    bytes: Vec<u8>,
    last_hw_write: Option<(u16, u8)>,
    last_hw_read: Option<u16>,
    hw_ranges: Vec<Range<usize>>
}

#[derive(Debug)]
pub struct RawAccessToHardwareError<T>(T);

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

impl Map {
    pub fn new() -> Map {
        Map {
            bytes: vec![0; MEM_SIZE],
            last_hw_write: None,
            last_hw_read: None,
            hw_ranges: vec![]
        }
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

    pub fn last_hw_read(&self) -> Option<u16> {
        self.last_hw_read
    }

    pub fn last_hw_write(&self) -> Option<(u16, u8)> {
        self.last_hw_write
    }

    /// Panics if `loc` is greater than `u16::MAX + 1`
    pub fn write(&mut self, loc: u16, val: u8) {
        self.bytes[loc as usize] = val;
        if self.hw_ranges.iter()
                         .any(|r| value_within_range(loc as usize, &r))
        {
            self.last_hw_write = Some((loc, val));
        }
        else {
            self.last_hw_write = None
        }
    }

    /// Panics if `loc` is greater than `u16::MAX + 1`.
    ///
    /// This function requires `&mut self` because reading can potentially
    /// have side effects, such as clearing hardware registers, etc.
    pub fn read(&mut self, loc: u16) -> u8 {
        let val = self.bytes[loc as usize];
        if self.hw_ranges.iter()
                         .any(|r| value_within_range(loc as usize, &r))
        {
            self.last_hw_read = Some(loc);
        }
        else {
            self.last_hw_read = None
        }

        val
    }

    /// Allows raw, immutable access to the underlying memory region. 
    ///
    /// The function will return a `RawAccessToHardwareError(Region(..))` if the 
    /// requested region overlaps a hardware memory mapped region. However,
    /// The error response still contains the requested region. This serves
    /// to indicate to the caller that they're potentially accessing a 
    /// region of memory that would otherwise generate side effects
    pub fn region_mut<'a>(&'a mut self, range: Range<usize>) 
        -> Result<RegionMut<'a>, RawAccessToHardwareError<RegionMut<'a>>>
    {
        if self.hw_ranges.iter().any(|r| ranges_overlap(r.clone(), &range)) {
            Err(RawAccessToHardwareError(RegionMut(&mut self.bytes[range])))
        }
        else {
            Ok(RegionMut(&mut self.bytes[range]))
        }

    }

    /// Allows raw, mutable access to the underlying memory region. 
    ///
    /// The function will return a `RawAccessToHardwareError(Region(..))` if the 
    /// requested region overlaps a hardware memory mapped region. However,
    /// The error response still contains the requested region. This serves
    /// to indicate to the caller that they're potentially accessing a 
    /// region of memory that would otherwise generate side effects
    pub fn region<'a>(&'a self, range: Range<usize>) 
        -> Result<Region<'a>, RawAccessToHardwareError<Region<'a>>> 
    {
        if self.hw_ranges.iter().any(|r| ranges_overlap(r.clone(), &range)) {
            Err(RawAccessToHardwareError(Region(&self.bytes[range])))
        }
        else {
            Ok(Region(&self.bytes[range]))
        }

    }

    pub fn region_from<'a>(&'a self, range: RangeFrom<usize>)
        -> Result<Region<'a>, RawAccessToHardwareError<Region<'a>>>
    {
        let len = self.bytes.len();
        self.region(range.start..len)
    }

    pub fn region_from_mut<'a>(&'a mut self, range: RangeFrom<usize>)
        -> Result<RegionMut<'a>, RawAccessToHardwareError<RegionMut<'a>>>
    {
        let len = self.bytes.len();
        self.region_mut(range.start..len)
    }

    pub fn region_to<'a>(&'a self, range: RangeTo<usize>)
        -> Result<Region<'a>, RawAccessToHardwareError<Region<'a>>>
    {
        self.region(0..range.end)
    }

    pub fn region_to_mut<'a>(&'a mut self, range: RangeTo<usize>)
        -> Result<RegionMut<'a>, RawAccessToHardwareError<RegionMut<'a>>>
    {
        self.region_mut(0..range.end)
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

