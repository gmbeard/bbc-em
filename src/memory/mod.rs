pub mod map;
pub mod region;
pub use self::map::{
    Map, 
    MemoryMap, 
    AsMemoryRegion, 
    AsMemoryRegionMut, 
    RawAccessToHardwareError
};
pub use self::region::{Region, RegionMut};

