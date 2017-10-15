use std::u8;

#[derive(Default, Clone, Copy)]
pub struct Io(pub(crate) u8);
#[derive(Default, Clone, Copy)]
pub struct DataDirection(pub(crate) u8);

impl From<Io> for u8 {
    fn from(p: Io) -> u8 {
        p.0
    }
}

impl From<DataDirection> for u8 {
    fn from(dd: DataDirection) -> u8 {
        dd.0
    }
}

#[derive(Default)]
pub struct PeripheralPort {
    io: Io,
    ddr: DataDirection,
}

impl PeripheralPort {
    pub fn new(io: Io, ddr: DataDirection) -> PeripheralPort {
        PeripheralPort {
            io: io,
            ddr: ddr,
        }
    }

    pub fn set_data_direction(&mut self, val: u8) {
        self.ddr = DataDirection(val);
    }

    pub fn data_direction(&self) -> DataDirection {
        self.ddr
    }

    pub fn set_io(&mut self, io: Io) {
        self.io = io;
    }

    pub fn io(&self) -> Io {
        self.io
    }

    pub fn read(&self) -> u8 {
        u8::from(self.io) & !u8::from(self.ddr)
    }

    pub fn write(&mut self, val: u8) {
        self.io = Io(val & u8::from(self.ddr))
    }
}

