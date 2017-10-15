use std::fmt;
use std::error::Error;

use memory::{MemoryMap, AsMemoryRegion, AsMemoryRegionMut};

#[derive(Debug, PartialEq)]
pub enum Addressing {
    Implied,
    Accumulator,
    Immediate(u8),
    Relative(i8),
    ZeroPage(u8),
    ZeroPageX(u8),
    ZeroPageY(u8),
    Absolute(u16),
    AbsoluteX(u16),
    AbsoluteY(u16),
    Indirect(u16),
    IndirectX(u8),
    IndirectY(u8),
}

impl fmt::Display for Addressing {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Addressing::*;

        match *self {
            Accumulator => write!(f, "A"),
            Immediate(ref a) => write!(f, "#{:02x}", *a),
            Relative(ref a) => write!(f, "{:02x}", *a),
            ZeroPage(ref a) => write!(f, "${:02x}", *a),
            ZeroPageX(ref a) => write!(f, "${:02x}, X", *a),
            ZeroPageY(ref a) => write!(f, "${:02x}, Y", *a),
            Absolute(ref a) => write!(f, "${:04x}", *a),
            AbsoluteX(ref a) => write!(f, "${:04x}, X", *a),
            AbsoluteY(ref a) => write!(f, "${:04x}, Y", *a),
            Indirect(ref a) => write!(f, "$({:04x})", *a),
            IndirectX(ref a) => write!(f, "$({:02x}, X)", *a),
            IndirectY(ref a) => write!(f, "$({:02x}), Y", *a),
            _ => return Ok(())
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum OpCode {
    Adc,
    And,
    Asl,
    Bcc,
    Bcs,
    Beq,
    Bit,
    Bmi,
    Bne,
    Bpl,
    Brk,
    Bvc,
    Bvs,
    Clc,
    Cld,
    Cli,
    Clv,
    Cmp,
    Cpx,
    Cpy,
    Dec,
    Dex,
    Dey,
    Eor,
    Inc,
    Inx,
    Iny,
    Jmp,
    Jsr,
    Lda,
    Ldx,
    Ldy,
    Lsr,
    Nop,
    Ora,
    Pha,
    Php,
    Pla,
    Plp,
    Rol,
    Ror,
    Rti,
    Rts,
    Sbc,
    Sec,
    Sed,
    Sei,
    Sta,
    Stx,
    Sty,
    Tax,
    Tay,
    Tsx,
    Txa,
    Txs,
    Tya,
}

#[derive(Debug, PartialEq)]
pub struct Instruction(OpCode, Addressing, usize);

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} {}", self.0, self.1)
    }
}

#[derive(Debug, PartialEq)]
pub struct InstructionDecodeError;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct StatusFlags {
    pub negative: bool,
    pub overflow: bool,
    pub brk: bool,
    pub decimal: bool,
    pub interrupt: bool,
    pub zero: bool,
    pub carry: bool,
}

impl StatusFlags  {
    fn new() -> StatusFlags {
        StatusFlags {
            negative: false,
            overflow: false,
            brk: false,
            decimal: false,
            interrupt: false,
            zero: false,
            carry: false,
        }
    }
}

impl<'a> From<&'a StatusFlags> for u8 {
    fn from(f: &'a StatusFlags) -> u8 {
        f.carry as u8 & 0x01 |
        (f.zero as u8 & 0x01) << 1 |
        (f.interrupt as u8 & 0x01) << 2 |
        (f.decimal as u8 & 0x01) << 3 |
        (f.brk as u8 & 0x01) << 4 |
        0x01 << 5 |
        (f.overflow as u8 & 0x01) << 6 |
        (f.negative as u8 & 0x01) << 7
    }
}

impl From<u8> for StatusFlags {
    fn from(b: u8) -> StatusFlags {
        StatusFlags {
            carry: 0 != b & 0x01,
            zero: 0 != b & 0x02,
            interrupt: 0 != b & 0x04,
            decimal: 0 != b & 0x08,
            brk: 0 != b & 0x10,
            overflow: 0 != b & 0x40,
            negative: 0 != b & 0x80
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Registers {
    pub pc: u16,
    pub sp: u8,
    pub acc: u8,
    pub x: u8,
    pub y: u8,
    pub status: StatusFlags
}


impl Registers {
    pub fn new() -> Registers {
        Registers {
            pc: 0,
            sp: 0,
            acc: 0,
            x: 0,
            y: 0,
            status: StatusFlags::new()
        }
    }
}

#[inline(always)]
fn decode_u8<'a, I>(iter: &mut I) -> Result<u8, InstructionDecodeError>
    where I: Iterator<Item=&'a u8>
{
    iter.next().map(|i| *i).ok_or_else(|| InstructionDecodeError)
}

#[inline(always)]
fn decode_i8<'a, I>(iter: &mut I) -> Result<i8, InstructionDecodeError>
    where I: Iterator<Item=&'a u8>
{
    iter.next().map(|i| *i as i8).ok_or_else(|| InstructionDecodeError)
}

#[inline(always)]
fn decode_u16<'a, I>(iter: &mut I) -> Result<u16, InstructionDecodeError>
    where I: Iterator<Item=&'a u8>
{
    let low = decode_u8(iter)?;
    let hi = decode_u8(iter)?;

    Ok(low as u16 | (hi as u16) << 8)
}

pub fn decode_instruction(mem: &[u8]) -> Result<(usize, Instruction), InstructionDecodeError> {
    use self::Addressing::*;
    use self::OpCode::*;

    let mut iter = mem.iter();
    let len = iter.as_slice().len();
    if let Some(opcode) = iter.next() {
        let ins = match *opcode {
            0x69 => Some(Instruction(Adc, Immediate(decode_u8(&mut iter)?), 2)),
            0x65 => Some(Instruction(Adc, ZeroPage(decode_u8(&mut iter)?),  3)),
            0x75 => Some(Instruction(Adc, ZeroPageX(decode_u8(&mut iter)?), 4)),
            0x6d => Some(Instruction(Adc, Absolute(decode_u16(&mut iter)?),4)),
            0x7d => Some(Instruction(Adc, AbsoluteX(decode_u16(&mut iter)?),4)),
            0x79 => Some(Instruction(Adc, AbsoluteY(decode_u16(&mut iter)?),4)),
            0x61 => Some(Instruction(Adc, IndirectX(decode_u8(&mut iter)?), 6)),
            0x71 => Some(Instruction(Adc, IndirectY(decode_u8(&mut iter)?), 5)),

            0x29 => Some(Instruction(And, Immediate(decode_u8(&mut iter)?), 2)),
            0x25 => Some(Instruction(And, ZeroPage(decode_u8(&mut iter)?),  3)),
            0x35 => Some(Instruction(And, ZeroPageX(decode_u8(&mut iter)?), 4)),
            0x2d => Some(Instruction(And, Absolute(decode_u16(&mut iter)?),4)),
            0x3d => Some(Instruction(And, AbsoluteX(decode_u16(&mut iter)?),4)),
            0x39 => Some(Instruction(And, AbsoluteY(decode_u16(&mut iter)?),4)),
            0x21 => Some(Instruction(And, IndirectX(decode_u8(&mut iter)?), 6)),
            0x31 => Some(Instruction(And, IndirectY(decode_u8(&mut iter)?), 5)),

            0x0a => Some(Instruction(Asl, Accumulator, 2)),
            0x06 => Some(Instruction(Asl, ZeroPage(decode_u8(&mut iter)?), 5)),
            0x16 => Some(Instruction(Asl, ZeroPageX(decode_u8(&mut iter)?), 6)),
            0x0e => Some(Instruction(Asl, Absolute(decode_u16(&mut iter)?), 6)),
            0x1e => Some(Instruction(Asl, AbsoluteX(decode_u16(&mut iter)?), 7)),

            0x90 => Some(Instruction(Bcc, Relative(decode_i8(&mut iter)?), 2)),

            0xb0 => Some(Instruction(Bcs, Relative(decode_i8(&mut iter)?), 2)),

            0xf0 => Some(Instruction(Beq, Relative(decode_i8(&mut iter)?), 2)),

            0x30 => Some(Instruction(Bmi, Relative(decode_i8(&mut iter)?), 2)),

            0xd0 => Some(Instruction(Bne, Relative(decode_i8(&mut iter)?), 2)),

            0x10 => Some(Instruction(Bpl, Relative(decode_i8(&mut iter)?), 2)),

            0x50 => Some(Instruction(Bvc, Relative(decode_i8(&mut iter)?), 2)),

            0x70 => Some(Instruction(Bvs, Relative(decode_i8(&mut iter)?), 2)),

            0x24 => Some(Instruction(Bit, ZeroPage(decode_u8(&mut iter)?), 3)),
            0x2c => Some(Instruction(Bit, Absolute(decode_u16(&mut iter)?), 4)),

            0x00 => Some(Instruction(Brk, Implied, 7)),

            0x18 => Some(Instruction(Clc, Implied, 2)),

            0xd8 => Some(Instruction(Cld, Implied, 2)),

            0x58 => Some(Instruction(Cli, Implied, 2)),

            0xb8 => Some(Instruction(Clv, Implied, 2)),

            0xc9 => Some(Instruction(Cmp, Immediate(decode_u8(&mut iter)?), 2)),
            0xc5 => Some(Instruction(Cmp, ZeroPage(decode_u8(&mut iter)?), 2)),
            0xd5 => Some(Instruction(Cmp, ZeroPageX(decode_u8(&mut iter)?), 4)),
            0xcd => Some(Instruction(Cmp, Absolute(decode_u16(&mut iter)?), 4)),
            0xdd => Some(Instruction(Cmp, AbsoluteX(decode_u16(&mut iter)?), 4)),
            0xd9 => Some(Instruction(Cmp, AbsoluteY(decode_u16(&mut iter)?), 4)),
            0xc1 => Some(Instruction(Cmp, IndirectX(decode_u8(&mut iter)?), 6)),
            0xd1 => Some(Instruction(Cmp, IndirectY(decode_u8(&mut iter)?), 5)),
            
            0xe0 => Some(Instruction(Cpx, Immediate(decode_u8(&mut iter)?), 2)),
            0xe4 => Some(Instruction(Cpx, ZeroPage(decode_u8(&mut iter)?), 3)),
            0xec => Some(Instruction(Cpx, Absolute(decode_u16(&mut iter)?), 4)),
            
            0xc0 => Some(Instruction(Cpy, Immediate(decode_u8(&mut iter)?), 2)),
            0xc4 => Some(Instruction(Cpy, ZeroPage(decode_u8(&mut iter)?), 3)),
            0xcc => Some(Instruction(Cpy, Absolute(decode_u16(&mut iter)?), 4)),
            
            0xc6 => Some(Instruction(Dec, ZeroPage(decode_u8(&mut iter)?), 5)),
            0xd6 => Some(Instruction(Dec, ZeroPageX(decode_u8(&mut iter)?), 6)),
            0xce => Some(Instruction(Dec, Absolute(decode_u16(&mut iter)?), 3)),
            0xde => Some(Instruction(Dec, AbsoluteX(decode_u16(&mut iter)?), 7)),
            
            0xca => Some(Instruction(Dex, Implied, 2)),

            0x88 => Some(Instruction(Dey, Implied, 2)),

            0x49 => Some(Instruction(Eor, Immediate(decode_u8(&mut iter)?), 2)),
            0x45 => Some(Instruction(Eor, ZeroPage(decode_u8(&mut iter)?), 3)),
            0x55 => Some(Instruction(Eor, ZeroPageX(decode_u8(&mut iter)?), 4)),
            0x4d => Some(Instruction(Eor, Absolute(decode_u16(&mut iter)?), 4)),
            0x5d => Some(Instruction(Eor, AbsoluteX(decode_u16(&mut iter)?), 4)),
            0x59 => Some(Instruction(Eor, AbsoluteY(decode_u16(&mut iter)?), 4)),
            0x41 => Some(Instruction(Eor, IndirectX(decode_u8(&mut iter)?), 6)),
            0x51 => Some(Instruction(Eor, IndirectY(decode_u8(&mut iter)?), 5)),
            
            0xe6 => Some(Instruction(Inc, ZeroPage(decode_u8(&mut iter)?), 5)),
            0xf6 => Some(Instruction(Inc, ZeroPageX(decode_u8(&mut iter)?), 6)),
            0xee => Some(Instruction(Inc, Absolute(decode_u16(&mut iter)?), 6)),
            0xfe => Some(Instruction(Inc, AbsoluteX(decode_u16(&mut iter)?), 7)),
            
            0xe8 => Some(Instruction(Inx, Implied, 2)),

            0xc8 => Some(Instruction(Iny, Implied, 2)),

            0x4c => Some(Instruction(Jmp, Absolute(decode_u16(&mut iter)?), 3)),
            0x6c => Some(Instruction(Jmp, Indirect(decode_u16(&mut iter)?), 5)),

            0x20 => Some(Instruction(Jsr, Absolute(decode_u16(&mut iter)?), 6)),

            0xa9 => Some(Instruction(Lda, Immediate(decode_u8(&mut iter)?), 2)),
            0xa5 => Some(Instruction(Lda, ZeroPage(decode_u8(&mut iter)?), 3)),
            0xb5 => Some(Instruction(Lda, ZeroPageX(decode_u8(&mut iter)?), 4)),
            0xad => Some(Instruction(Lda, Absolute(decode_u16(&mut iter)?), 4)),
            0xbd => Some(Instruction(Lda, AbsoluteX(decode_u16(&mut iter)?), 4)),
            0xb9 => Some(Instruction(Lda, AbsoluteY(decode_u16(&mut iter)?), 4)),
            0xa1 => Some(Instruction(Lda, IndirectX(decode_u8(&mut iter)?), 6)),
            0xb1 => Some(Instruction(Lda, IndirectY(decode_u8(&mut iter)?), 5)),
            
            0xa2 => Some(Instruction(Ldx, Immediate(decode_u8(&mut iter)?), 2)),
            0xa6 => Some(Instruction(Ldx, ZeroPage(decode_u8(&mut iter)?), 3)),
            0xb6 => Some(Instruction(Ldx, ZeroPageY(decode_u8(&mut iter)?), 4)),
            0xae => Some(Instruction(Ldx, Absolute(decode_u16(&mut iter)?), 4)),
            0xbe => Some(Instruction(Ldx, AbsoluteY(decode_u16(&mut iter)?), 4)),
            
            0xa0 => Some(Instruction(Ldy, Immediate(decode_u8(&mut iter)?), 2)),
            0xa4 => Some(Instruction(Ldy, ZeroPage(decode_u8(&mut iter)?), 3)),
            0xb4 => Some(Instruction(Ldy, ZeroPageX(decode_u8(&mut iter)?), 4)),
            0xac => Some(Instruction(Ldy, Absolute(decode_u16(&mut iter)?), 4)),
            0xbc => Some(Instruction(Ldy, AbsoluteX(decode_u16(&mut iter)?), 4)),
            
            0x4a => Some(Instruction(Lsr, Accumulator, 2)),
            0x46 => Some(Instruction(Lsr, ZeroPage(decode_u8(&mut iter)?), 5)),
            0x56 => Some(Instruction(Lsr, ZeroPageX(decode_u8(&mut iter)?), 6)),
            0x4e => Some(Instruction(Lsr, Absolute(decode_u16(&mut iter)?), 6)),
            0x5e => Some(Instruction(Lsr, AbsoluteX(decode_u16(&mut iter)?), 7)),

            0xea => Some(Instruction(Nop, Implied, 2)),

            0x09 => Some(Instruction(Ora, Immediate(decode_u8(&mut iter)?), 2)),
            0x05 => Some(Instruction(Ora, ZeroPage(decode_u8(&mut iter)?), 3)),
            0x15 => Some(Instruction(Ora, ZeroPageX(decode_u8(&mut iter)?), 4)),
            0x0d => Some(Instruction(Ora, Absolute(decode_u16(&mut iter)?), 4)),
            0x1d => Some(Instruction(Ora, AbsoluteX(decode_u16(&mut iter)?), 4)),
            0x19 => Some(Instruction(Ora, AbsoluteY(decode_u16(&mut iter)?), 4)),
            0x01 => Some(Instruction(Ora, IndirectX(decode_u8(&mut iter)?), 6)),
            0x11 => Some(Instruction(Ora, IndirectY(decode_u8(&mut iter)?), 5)),
            
            0x48 => Some(Instruction(Pha, Implied, 3)),

            0x08 => Some(Instruction(Php, Implied, 3)),

            0x68 => Some(Instruction(Pla, Implied, 4)),

            0x28 => Some(Instruction(Plp, Implied, 4)),

            0x2a => Some(Instruction(Rol, Accumulator, 2)),
            0x26 => Some(Instruction(Rol, ZeroPage(decode_u8(&mut iter)?), 5)),
            0x36 => Some(Instruction(Rol, ZeroPageX(decode_u8(&mut iter)?), 6)),
            0x2e => Some(Instruction(Rol, Absolute(decode_u16(&mut iter)?), 6)),
            0x3e => Some(Instruction(Rol, AbsoluteX(decode_u16(&mut iter)?), 7)),

            0x6a => Some(Instruction(Ror, Accumulator, 2)),
            0x66 => Some(Instruction(Ror, ZeroPage(decode_u8(&mut iter)?), 5)),
            0x76 => Some(Instruction(Ror, ZeroPageX(decode_u8(&mut iter)?), 6)),
            0x6e => Some(Instruction(Ror, Absolute(decode_u16(&mut iter)?), 6)),
            0x7e => Some(Instruction(Ror, AbsoluteX(decode_u16(&mut iter)?), 7)),

            0x40 => Some(Instruction(Rti, Implied, 6)),

            0x60 => Some(Instruction(Rts, Implied, 6)),

            0xe9 => Some(Instruction(Sbc, Immediate(decode_u8(&mut iter)?), 2)),
            0xe5 => Some(Instruction(Sbc, ZeroPage(decode_u8(&mut iter)?), 3)),
            0xf5 => Some(Instruction(Sbc, ZeroPageX(decode_u8(&mut iter)?), 4)),
            0xed => Some(Instruction(Sbc, Absolute(decode_u16(&mut iter)?), 4)),
            0xfd => Some(Instruction(Sbc, AbsoluteX(decode_u16(&mut iter)?), 4)),
            0xf9 => Some(Instruction(Sbc, AbsoluteY(decode_u16(&mut iter)?), 4)),
            0xe1 => Some(Instruction(Sbc, IndirectX(decode_u8(&mut iter)?), 6)),
            0xf1 => Some(Instruction(Sbc, IndirectY(decode_u8(&mut iter)?), 5)),
            
            0x38 => Some(Instruction(Sec, Implied, 2)),

            0xf8 => Some(Instruction(Sed, Implied, 2)),

            0x78 => Some(Instruction(Sei, Implied, 2)),

            0x85 => Some(Instruction(Sta, ZeroPage(decode_u8(&mut iter)?), 3)),
            0x95 => Some(Instruction(Sta, ZeroPageX(decode_u8(&mut iter)?), 4)),
            0x8d => Some(Instruction(Sta, Absolute(decode_u16(&mut iter)?), 4)),
            0x9d => Some(Instruction(Sta, AbsoluteX(decode_u16(&mut iter)?), 5)),
            0x99 => Some(Instruction(Sta, AbsoluteY(decode_u16(&mut iter)?), 5)),
            0x81 => Some(Instruction(Sta, IndirectX(decode_u8(&mut iter)?), 6)),
            0x91 => Some(Instruction(Sta, IndirectY(decode_u8(&mut iter)?), 6)),
            
            0x86 => Some(Instruction(Stx, ZeroPage(decode_u8(&mut iter)?), 3)),
            0x96 => Some(Instruction(Stx, ZeroPageY(decode_u8(&mut iter)?), 4)),
            0x8e => Some(Instruction(Stx, Absolute(decode_u16(&mut iter)?), 4)),

            0x84 => Some(Instruction(Sty, ZeroPage(decode_u8(&mut iter)?), 3)),
            0x94 => Some(Instruction(Sty, ZeroPageX(decode_u8(&mut iter)?), 4)),
            0x8c => Some(Instruction(Sty, Absolute(decode_u16(&mut iter)?), 4)),

            0xaa => Some(Instruction(Tax, Implied, 2)),

            0xa8 => Some(Instruction(Tay, Implied, 2)),

            0xba => Some(Instruction(Tsx, Implied, 2)),

            0x8a => Some(Instruction(Txa, Implied, 2)),

            0x9a => Some(Instruction(Txs, Implied, 2)),

            0x98 => Some(Instruction(Tya, Implied, 2)),

            _ => None
        };

        if let Some(ins) = ins {
            return Ok((len - iter.as_slice().len(), ins));
        }
    }

    Err(InstructionDecodeError)
}

#[derive(Debug)]
pub struct MemoryAccessError;

#[derive(Debug)]
pub enum StackError {
    Overflow,
    Underflow,
    Unavailable,
}

#[derive(Debug)]
pub enum CpuError {
    Memory(MemoryAccessError),
    Stack(StackError),
    InvalidInstruction,
    Paused,
}

impl Error for MemoryAccessError {
    fn description(&self) -> &str {
        "Attempted to access an invalid memory location"
    }
}

impl Error for StackError {
    fn description(&self) -> &str {
        match *self {
            StackError::Overflow => "Stack overflow",
            StackError::Underflow => "Stack underflow",
            StackError::Unavailable => "Stack not present",
        }
    }
}

impl Error for CpuError {
    fn description(&self) -> &str {
        match *self {
            CpuError::Memory(ref e) => e.description(),
            CpuError::Stack(ref e) => e.description(),
            CpuError::Paused => "CPU paused",
            CpuError::InvalidInstruction => "Invalid instruction",
        }
    }
}

impl fmt::Display for MemoryAccessError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", <Self as Error>::description(self))
    }
}

impl fmt::Display for StackError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", <Self as Error>::description(self))
    }
}

impl From<MemoryAccessError> for CpuError {
    fn from(e: MemoryAccessError) -> CpuError {
        CpuError::Memory(e)
    }
}

impl From<StackError> for CpuError {
    fn from(e: StackError) -> CpuError {
        CpuError::Stack(e)
    }
}

impl From<MemoryAccessError> for StackError {
    fn from(_: MemoryAccessError) -> StackError {
        StackError::Unavailable
    }
}

impl From<InstructionDecodeError> for CpuError {
    fn from(_: InstructionDecodeError) -> CpuError {
        CpuError::InvalidInstruction
    }
}

impl fmt::Display for CpuError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", <Self as Error>::description(self))
    }
}

const STACK_BOTTOM: u16 = 0x0100;

fn push_stack<M: MemoryMap>(val: u8, mut mem: M, reg: &mut Registers) -> Result<(), StackError> {
    mem.write(STACK_BOTTOM + reg.sp as u16, val);
    reg.sp.checked_sub(1)
        .map(|r| reg.sp = r)
        .ok_or_else(|| StackError::Overflow)?;

    Ok(())
}

fn pop_stack<M: MemoryMap>(mut mem: M, reg: &mut Registers) -> Result<u8, StackError> {
    reg.sp.checked_add(1)
        .map(|r| reg.sp = r)
        .ok_or_else(|| StackError::Underflow)?;

    let val = mem.read(STACK_BOTTOM + reg.sp as u16);
    Ok(val)
}

fn write_mem<M: MemoryMap>(val: u8, 
                           addr: &Addressing, 
                           mut mem: M, 
                           reg: &mut Registers) -> Result<(), MemoryAccessError> 
{
    use self::Addressing::*;

    match *addr {
        Accumulator => reg.acc = val,
        Absolute(ref loc) => mem.write(*loc, val),
        AbsoluteX(ref loc) => mem.write(*loc + reg.x as u16, val),
        AbsoluteY(ref loc) => mem.write(*loc + reg.y as u16, val),
        ZeroPage(ref loc) => mem.write(*loc as _, val),
        ZeroPageX(ref loc) => mem.write(loc.wrapping_add(reg.x) as _, val),
        IndirectX(ref loc) => {
            let loc = *loc as u16 + reg.x as u16;
            let target = mem.read(loc) as u16 | (mem.read(loc + 1) as u16) << 8;
//            let target_lo = mem.read(loc as _) as u16;
//            let target_hi = mem.read(loc as u16 + 1) as u16;
//            let target = target_lo as u16 | (target_hi as u16) << 8;
            mem.write(target as _, val);
        },
        IndirectY(ref loc) => {
            let target = mem.read(*loc as _) as u16 | (mem.read(*loc as u16 + 1) as u16) << 8;
//            let target_lo = mem.read(*loc as _);
//            let target_hi = mem.read(*loc as u16 + 1);
//            let target = target_lo as u16 | (target_hi as u16) << 8;
            mem.write(target.wrapping_add(reg.y as u16) as _, val);
        },
        _ => unreachable!()
    }

    Ok(())
}

fn page_crossed(from: u16, to: u16) -> bool {
    (from & 0xff00) != (to & 0xff00)
}

fn read_mem<M: MemoryMap>(addr: &Addressing, 
                          mut mem: M, 
                          reg: &Registers) -> Result<(u8, bool), MemoryAccessError> 
{
    use self::Addressing::*;

    match *addr {
        Accumulator => Ok((reg.acc, false)),
        Immediate(ref v) => Ok((*v, false)),
        Absolute(ref loc) => Ok((mem.read(*loc), false)),
        AbsoluteX(ref loc) => Ok((mem.read(*loc + reg.x as u16), page_crossed(reg.pc, *loc + reg.x as u16))),
        AbsoluteY(ref loc) => Ok((mem.read(*loc + reg.y as u16), page_crossed(reg.pc, *loc + reg.y as u16))),
        Indirect(ref loc) => {
            let target = mem.read(*loc as _) as u16 | (mem.read(*loc as u16 + 1) as u16) << 8;
            Ok((mem.read(target), false))
        },
        IndirectX(ref loc) => {
            let loc = *loc as u16 + reg.x as u16;
            let target = mem.read(loc) as u16 | (mem.read(loc + 1) as u16) << 8;
//            let target_lo = mem.read(loc as _) as u16;
//            let target_hi = mem.read(loc as u16 + 1) as u16;
//            let target = target_lo as u16 | (target_hi as u16) << 8;
            Ok((mem.read(target as _), false))
        },
        IndirectY(ref loc) => {
            let target = mem.read(*loc as _) as u16 | (mem.read(*loc as u16 + 1) as u16) << 8;
//            let target_lo = mem.read(*loc as _);
//            let target_hi = mem.read(*loc as u16 + 1);
//            let target = target_lo as u16 | (target_hi as u16) << 8;
            Ok((mem.read(target.wrapping_add(reg.y as u16)), page_crossed(reg.pc, target.wrapping_add(reg.y as u16))))
        },
        ZeroPage(ref loc) => Ok((mem.read(*loc as _), false)),
        ZeroPageX(ref loc) => Ok((mem.read(loc.wrapping_add(reg.x) as _), false)),
        ZeroPageY(ref loc) => Ok((mem.read(loc.wrapping_add(reg.y) as _), false)),
        Relative(_) | Implied => panic!(format!("Attempting to read mem for {:?}", addr))
    }
}

fn execute_instruction<M: MemoryMap>(ins: Instruction, 
                                     mut mem: M, 
                                     reg: &mut Registers) -> Result<usize, CpuError>
{
    use self::OpCode::*;

    match ins.0 {
        Adc => {
            assert!(!reg.status.decimal);

            let orig = reg.acc;
            let (val, cross_page) = read_mem(&ins.1, mem, reg)?;
            let (v, o) = reg.acc.overflowing_add(val);
            reg.acc = v;

            // Carry?
            reg.status.carry = o;
            // Zero?
            reg.status.zero = reg.acc == 0;
            // Overflow?
            reg.status.overflow = 0 != (orig & 0x80) ^ (reg.acc & 0x80);
            // Negative?
            reg.status.negative = bit_is_set!(reg.acc, 7);

            Ok(ins.2 + if cross_page { 1 } else { 0 })
        },
        And => {
            let (val, cross_page) = read_mem(&ins.1, mem, reg)?;
            reg.acc = val & reg.acc;

            reg.status.zero = reg.acc == 0;
            reg.status.negative = bit_is_set!(reg.acc, 7);

            Ok(ins.2 + if cross_page { 1 } else { 0 })
        },
        Asl => {
            let (val, _) = read_mem(&ins.1, &mut mem, reg)?;
            let (result, overflow) = val.overflowing_shl(1);
            reg.status.carry = overflow;
            reg.status.zero = result == 0;
            reg.status.negative = bit_is_set!(result, 7);
            write_mem(result, &ins.1, mem, reg)?;

            Ok(ins.2)
        },
        Bcc => {
            if !reg.status.carry {
                match ins.1 {
                    Addressing::Relative(ref offset) => {
                        let old_pos = reg.pc;
                        reg.pc = (reg.pc as i16 + *offset as i16) as u16;
                        let cross_page = (old_pos & 0xff00) != (reg.pc & 0xff00);
                        Ok(ins.2 + 1 + if cross_page { 1 } else { 0 })
                    },
                    _ => unreachable!()
                }
            }
            else {
                Ok(ins.2)
            }
        },
        Bcs => {
            if reg.status.carry {
                match ins.1 {
                    Addressing::Relative(ref offset) => {
                        let old_pos = reg.pc;
                        reg.pc = (reg.pc as i16 + *offset as i16) as u16;
                        let cross_page = (old_pos & 0xff00) != (reg.pc & 0xff00);
                        Ok(ins.2 + 1 + if cross_page { 1 } else { 0 })
                    },
                    _ => unreachable!()
                }
            }
            else {
                Ok(ins.2)
            }
        },
        Beq => {
            if reg.status.zero {
                match ins.1 {
                    Addressing::Relative(ref offset) => {
                        let old_pos = reg.pc;
                        reg.pc = (reg.pc as i16 + *offset as i16) as u16;
                        let cross_page = (old_pos & 0xff00) != (reg.pc & 0xff00);
                        Ok(ins.2 + 1 + if cross_page { 1 } else { 0 })
                    },
                    _ => unreachable!()
                }
            }
            else {
                Ok(ins.2)
            }
        },
        Bit => {
            let (val, _) = read_mem(&ins.1, mem, reg)?;
            reg.status.zero = 0 == val & reg.acc;
            reg.status.overflow = bit_is_set!(val, 6);
            reg.status.negative = bit_is_set!(val, 7);

            Ok(ins.2)
        },
        Bmi => {
            if reg.status.negative {
                match ins.1 {
                    Addressing::Relative(ref offset) => {
                        let old_pos = reg.pc;
                        reg.pc = (reg.pc as i16 + *offset as i16) as u16;
                        let cross_page = (old_pos & 0xff00) != (reg.pc & 0xff00);
                        Ok(ins.2 + 1 + if cross_page { 1 } else { 0 })
                    },
                    _ => unreachable!()
                }
            }
            else {
                Ok(ins.2)
            }
        },
        Bne => {
            if !reg.status.zero {
                match ins.1 {
                    Addressing::Relative(ref offset) => {
                        let old_pos = reg.pc;
                        reg.pc = (reg.pc as i16 + *offset as i16) as u16;
                        let cross_page = (old_pos & 0xff00) != (reg.pc & 0xff00);
                        Ok(ins.2 + 1 + if cross_page { 1 } else { 0 })
                    },
                    _ => unreachable!()
                }
            }
            else {
                Ok(ins.2)
            }
        },
        Bpl => {
            if !reg.status.negative {
                match ins.1 {
                    Addressing::Relative(ref offset) => {
                        let old_pos = reg.pc;
                        reg.pc = (reg.pc as i16 + *offset as i16) as u16;
                        let cross_page = (old_pos & 0xff00) != (reg.pc & 0xff00);
                        Ok(ins.2 + 1 + if cross_page { 1 } else { 0 })
                    },
                    _ => unreachable!()
                }
            }
            else {
                Ok(ins.2)
            }
        },
        Brk => {
            reg.status.brk = true;
            push_stack( ((reg.pc & 0xff00) >> 8) as u8, &mut mem, reg )?;
            push_stack( (reg.pc & 0x00ff) as u8, &mut mem, reg )?;
            push_stack( u8::from(&reg.status), &mut mem, reg )?;
            let lo = mem.read(0xfffe);
            let hi = mem.read(0xffff);
            reg.pc = (hi as u16) << 8 | lo as u16;
            reg.status.brk = true;

            Ok(7)
        },
        Bvc => {
            if !reg.status.overflow {
                match ins.1 {
                    Addressing::Relative(ref offset) => {
                        let old_pos = reg.pc;
                        reg.pc = (reg.pc as i16 + *offset as i16) as u16;
                        Ok(ins.2 + 1 + if page_crossed(old_pos, reg.pc) { 1 } else { 0 })
                    },
                    _ => unreachable!()
                }
            }
            else {
                Ok(ins.2)
            }
        }
        Bvs => {
            if reg.status.overflow {
                match ins.1 {
                    Addressing::Relative(ref offset) => {
                        let old_pos = reg.pc;
                        reg.pc = (reg.pc as i16 + *offset as i16) as u16;
                        Ok(ins.2 + 1 + if page_crossed(old_pos, reg.pc) { 1 } else { 0 })
                    },
                    _ => unreachable!()
                }
            }
            else {
                Ok(ins.2)
            }
        }
        Clc => {
            reg.status.carry = false;
            Ok(ins.2)
        },
        Cld => {
            reg.status.decimal = false;
            Ok(ins.2)
        },
        Cli => {
            reg.status.interrupt = false;
            Ok(ins.2)
        },
        Clv => {
            reg.status.overflow = false;
            Ok(ins.2)
        },
        Cmp => {
            let (val, cross_page) = read_mem(&ins.1, mem, reg)?;
            match reg.acc.overflowing_sub(val) {
                (val, true) => {
                    reg.status.carry = false;
                    reg.status.zero = false;
                    reg.status.negative = bit_is_set!(val, 7);
                },
                (val, false) => {
                    reg.status.carry = true;
                    reg.status.zero = val == 0;
                    reg.status.negative = false;
                }
            }

            Ok(ins.2 + if cross_page { 1 } else { 0 })
        },
        Cpx => {
            let (val, _) = read_mem(&ins.1, mem, reg)?;
            match reg.x.overflowing_sub(val) {
                (val, true) => {
                    reg.status.carry = false;
                    reg.status.zero = false;
                    reg.status.negative = bit_is_set!(val, 7);
                },
                (val, false) => {
                    reg.status.carry = true;
                    reg.status.zero = val == 0;
                    reg.status.negative = false;
                }
            }

            Ok(ins.2)
        },
        Cpy => {
            let (val, _) = read_mem(&ins.1, mem, reg)?;
            match reg.y.overflowing_sub(val) {
                (val, true) => {
                    reg.status.carry = false;
                    reg.status.zero = false;
                    reg.status.negative = bit_is_set!(val, 7);
                },
                (val, false) => {
                    reg.status.carry = true;
                    reg.status.zero = val == 0;
                    reg.status.negative = false;
                }
            }

            Ok(ins.2)
        },
        Dec => {
            let (mut val, _) = read_mem(&ins.1, &mut mem, reg)?;
            val = val.wrapping_sub(1); 
            write_mem(val, &ins.1, &mut mem, reg)?;
            reg.status.zero = 0 == val;
            reg.status.negative = bit_is_set!(val, 7);

            Ok(ins.2)
        },
        Dex => {
            reg.x = reg.x.wrapping_sub(1);
            reg.status.zero = 0 == reg.x;
            reg.status.negative = bit_is_set!(reg.x, 7);
            Ok(ins.2)
        },
        Dey => {
            reg.y = reg.y.wrapping_sub(1);
            reg.status.zero = 0 == reg.y;
            reg.status.negative = bit_is_set!(reg.y, 7);
            Ok(ins.2)
        },
        Eor => {
            let (val, cross_page) = read_mem(&ins.1, mem, reg)?;
            reg.acc ^= val;
            reg.status.zero = reg.acc == 0;
            reg.status.negative = bit_is_set!(reg.acc, 7);

            Ok(ins.2 + if cross_page { 1 } else { 0 })
        },
        Inc => {
            let (mut val, _) = read_mem(&ins.1, &mut mem, reg)?;
            val = val.wrapping_add(1); 
            write_mem(val, &ins.1, &mut mem, reg)?;
            reg.status.zero = 0 == val;
            reg.status.negative = bit_is_set!(val, 7);

            Ok(ins.2)
        },
        Inx => {
            reg.x = reg.x.wrapping_add(1);
            reg.status.zero = 0 == reg.x;
            reg.status.negative = bit_is_set!(reg.x, 7);
            Ok(ins.2)
        },
        Iny => {
            reg.y = reg.y.wrapping_add(1);
            reg.status.zero = 0 == reg.y;
            reg.status.negative = bit_is_set!(reg.y, 7);
            Ok(ins.2)
        },
        Jmp => {
            match ins.1 {
                Addressing::Absolute(ref loc) => reg.pc = *loc,
                Addressing::Indirect(ref vec) => {
                    let low = mem.read(*vec);
                    let hi = mem.read(*vec + 1);

                    reg.pc = (hi as u16) << 8 | (low as u16);
                }
                _ => unreachable!()
            }

            Ok(ins.2)
        },
        Jsr => {
            reg.pc -= 1;
            push_stack(((reg.pc & 0xff00) >> 8) as u8, &mut mem, reg)?;
            push_stack((reg.pc & 0x00ff) as u8, &mut mem, reg)?;
            if let Addressing::Absolute(ref loc) = ins.1 {
                reg.pc = *loc;
            }
            else {
                unreachable!();
            }
            Ok(6)
        },
        Lda => {
            let (val, cross_page) = read_mem(&ins.1, mem, reg)?;
            reg.acc = val;
            reg.status.zero = reg.acc == 0;
            reg.status.negative = bit_is_set!(reg.acc, 7);

            Ok(ins.2 + if cross_page { 1 } else { 0 })
        },
        Ldx => {
            let (val, cross_page) = read_mem(&ins.1, mem, reg)?;
            reg.x = val;
            reg.status.zero = reg.x == 0;
            reg.status.negative = bit_is_set!(reg.x, 7);

            Ok(ins.2 + if cross_page { 1 } else { 0 })
        },
        Ldy => {
            let (val, cross_page) = read_mem(&ins.1, mem, reg)?;
            reg.y = val;
            reg.status.zero = reg.y == 0;
            reg.status.negative = bit_is_set!(reg.y, 7);

            Ok(ins.2 + if cross_page { 1 } else { 0 })
        },
        Lsr => {
            let (mut val, _) = read_mem(&ins.1, &mut mem, reg)?;
            reg.status.carry = 0x01 == (0x01 & val);
            val = val >> 1;
            reg.status.zero = val == 0;
            reg.status.negative = bit_is_set!(val, 7);
            write_mem(val, &ins.1, mem, reg)?;

            Ok(ins.2)
        },
        Nop => Ok(2),
        Ora => {
            let (val, cross_page) = read_mem(&ins.1, mem, reg)?;
            reg.acc |= val;
            reg.status.zero = reg.acc == 0;
            reg.status.negative = bit_is_set!(reg.acc, 7);

            Ok(ins.2 + if cross_page { 1 } else { 0 })
        },
        Pha => {
            push_stack(reg.acc, mem, reg)?;
            Ok(ins.2)
        },
        Php => {
            push_stack(u8::from(&reg.status), mem, reg)?;
            Ok(ins.2)
        },
        Pla => {
            reg.acc = pop_stack(mem, reg)?;
            reg.status.zero = reg.acc == 0;
            reg.status.negative = bit_is_set!(reg.acc, 7);
            Ok(ins.2)
        },
        Plp => {
            reg.status = StatusFlags::from(pop_stack(mem, reg)?);
            Ok(ins.2)
        },
        Rol => {
            let (mut val, _) = read_mem(&ins.1, &mut mem, reg)?;
            let old_carry = reg.status.carry as u8;
            reg.status.zero = false;
            reg.status.carry = bit_is_set!(val, 7);
            val = (val & 0x7f) << 1;
            val = val | (old_carry & 0x01);
            reg.status.zero = val == 0;
            reg.status.negative = bit_is_set!(val, 7);
            write_mem(val, &ins.1, mem, reg)?;

            Ok(ins.2)
        },
        Ror => {
            let (mut val, _) = read_mem(&ins.1, &mut mem, reg)?;
            let old_carry = reg.status.carry as u8;
            reg.status.zero = false;
            reg.status.carry = bit_is_set!(val, 0);
            val = val >> 1;
            val = val | ((old_carry & 0x01) << 7);
            reg.status.negative = bit_is_set!(val, 7);
            reg.status.zero = val == 0;
            write_mem(val, &ins.1, mem, reg)?;

            Ok(ins.2)
        },
        Rti => {
            reg.status = StatusFlags::from(pop_stack(&mut mem, reg)?);
            reg.pc = (pop_stack(&mut mem, reg)? as u16) | ((pop_stack(&mut mem, reg)? as u16) << 8);
            Ok(6)
        },
        Rts => {
            reg.pc = (pop_stack(&mut mem, reg)? as u16) | ((pop_stack(&mut mem, reg)? as u16) << 8);
            reg.pc += 1;
            Ok(6)
        },
        Sbc => {
            assert!(!reg.status.decimal);

            let orig = reg.acc;
            let (val, cross_page) = read_mem(&ins.1, mem, reg)?;
            let (v, o) = reg.acc.overflowing_sub(val);
            reg.acc = v;

            // Carry?
            reg.status.carry = !o;
            // Zero?
            reg.status.zero = reg.acc == 0;
            // Overflow?
            reg.status.overflow = 0 != (orig & 0x80) ^ (reg.acc & 0x80);
            // Negative?
            reg.status.negative = bit_is_set!(reg.acc, 7);

            Ok(ins.2 + if cross_page { 1 } else { 0 })
        },
        Sec => {
            reg.status.carry = true;
            Ok(ins.2)
        },
        Sed => {
            reg.status.decimal = true;
            Ok(ins.2)
        },
        Sei => {
            reg.status.interrupt = true;
            Ok(ins.2)
        },
        Sta => {
            write_mem(reg.acc, &ins.1, mem, reg)?;
            Ok(ins.2)
        },
        Stx => {
            write_mem(reg.x, &ins.1, mem, reg)?;
            Ok(ins.2)
        },
        Sty => {
            write_mem(reg.y, &ins.1, mem, reg)?;
            Ok(ins.2)
        },
        Tax => {
            reg.x = reg.acc;
            Ok(ins.2)
        },
        Tay => {
            reg.y = reg.acc;
            Ok(ins.2)
        },
        Tsx => {
            reg.x = reg.sp;
            Ok(ins.2)
        },
        Txa => {
            reg.acc = reg.x;
            Ok(ins.2)
        },
        Txs => {
            reg.sp = reg.x;
            Ok(ins.2)
        },
        Tya => {
            reg.acc = reg.y;
            Ok(ins.2)
        },
    }
}

pub struct Cpu {
    registers: Registers,
}

fn push_cpu_state<M: MemoryMap>(cpu: &mut Cpu, mut mem: M) -> Result<(), CpuError> {
    push_stack(((cpu.registers.pc & 0xff00) >> 8) as u8, &mut mem, &mut cpu.registers)?;
    push_stack((cpu.registers.pc & 0x00ff) as u8, &mut mem, &mut cpu.registers)?;
    push_stack(u8::from(&cpu.registers.status), &mut mem, &mut cpu.registers)?;
    Ok(())
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            registers: Registers::new()
        }
    }

    pub fn program_counter(&self) -> u16 {
        self.registers.pc
    }

    pub fn registers(&self) -> &Registers {
        &self.registers
    }

    pub fn initialize<M>(&mut self, mut mem: M) -> Result<(), CpuError>
        where M: MemoryMap + AsMemoryRegionMut
    {
        for b in mem.region_mut(0xfe..0xff)
                    .unwrap_or_else(|r| r.0)
                    .iter_mut() 
        {
            *b = 0x00;
        }
        let low = mem.read(0xfffc);
        let hi = mem.read(0xfffd);
        self.registers.pc = ((hi as u16) << 8) | low as u16;
        Ok(())
    }

    pub fn step<M>(&mut self, mut mem: M) -> Result<usize, CpuError> 
        where M: MemoryMap + AsMemoryRegion
    {
        let (bytes, ins) = {
            let instruction_region = mem.region(self.registers.pc as _..self.registers.pc as usize + 4)
                                        .unwrap_or_else(|e| e.0);
            decode_instruction(&instruction_region).unwrap()
        };


        log_cpu!("{:04x}: {}", self.registers.pc, ins);
        self.registers.pc += bytes as u16;
        let result = execute_instruction(ins, &mut mem, &mut self.registers);
        result
    }

    pub fn non_maskable_interrupt<M: MemoryMap>(&mut self, mut mem: M) -> Result<(), CpuError> {
        log_cpu!("NMI");
        push_cpu_state(self, &mut mem)?;
        let low = mem.read(0xfffa);
        let hi = mem.read(0xfffb);
        self.registers.pc = ((hi as u16) << 8) | low as u16;
        Ok(())
    }

    pub fn interrupt_request<M>(&mut self, mut mem: M) -> Result<bool, CpuError>
        where M: MemoryMap
    {
        if !self.registers.status.interrupt {

            log_cpu!("IRQ");
//            mem.write(0xfe4d, 0xe0);
//            mem.write(0xfe4e, 0xe0);

            push_cpu_state(self, &mut mem)?;
            let low = mem.read(0xfffe);
            let hi = mem.read(0xffff);
            self.registers.pc = ((hi as u16) << 8) | low as u16;
            self.registers.status.interrupt = true;
            return Ok(true);
        }

        Ok(false)
    }
}

#[cfg(test)]
mod decode_should {
    use super::*;

    #[test]
    fn read_instruction() {
        let mem: &[u8] = &[0x7d, 0x00, 0x80];

        let (bytes, inst) = decode_instruction(mem).unwrap();
        assert_eq!(inst, Instruction(OpCode::Adc, Addressing::AbsoluteX(0x8000), 4));
        assert_eq!(bytes, 3);
    }

    #[test]
    fn read_multiple_instructions() {
        let mem: &[u8] = &[0x7d, 0x00, 0x80, 0x65, 0x01, 0x71, 0b10000001];
        let (first_bytes, first) = decode_instruction(mem).unwrap();
        let (second_bytes, second) = decode_instruction(&mem[first_bytes..]).unwrap();
        let (_, third) = decode_instruction(&mem[first_bytes+second_bytes..]).unwrap();

        assert_eq!(first, Instruction(OpCode::Adc, Addressing::AbsoluteX(0x8000), 4));
        assert_eq!(second, Instruction(OpCode::Adc, Addressing::ZeroPage(1), 3));
        assert_eq!(third, Instruction(OpCode::Adc, Addressing::IndirectY(0b10000001), 5));
    }
}

#[cfg(test)]
mod execute_should {
    use super::*;
    use memory::Map;

    #[test]
    fn add_to_accumulator_with_correct_overflow() {
        let mut mem = Map::new();
        let mut reg = Registers::new();
        reg.acc = 0x7f; 

        execute_instruction(
            Instruction(OpCode::Adc, Addressing::Immediate(0x7f), 2),
            &mut mem,
            &mut reg
        );

        assert!(reg.status.overflow);
        assert_eq!(0xfe, reg.acc);

        reg.acc = 0x3f;
        execute_instruction(
            Instruction(OpCode::Adc, Addressing::Immediate(0x3f), 2),
            &mut mem,
            &mut reg
        );

        assert!(!reg.status.overflow);
    }
}
