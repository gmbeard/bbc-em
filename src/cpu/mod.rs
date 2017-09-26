use std::fmt;

#[derive(Debug, PartialEq)]
enum Addressing {
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
            Immediate(ref a) => write!(f, "#{}", *a),
            Relative(ref a) => write!(f, "{:x}", *a),
            ZeroPage(ref a) => write!(f, "${:x}", *a),
            ZeroPageX(ref a) => write!(f, "${:x}, X", *a),
            ZeroPageY(ref a) => write!(f, "${:x}, Y", *a),
            Absolute(ref a) => write!(f, "${:x}", *a),
            AbsoluteX(ref a) => write!(f, "${:x}, X", *a),
            AbsoluteY(ref a) => write!(f, "${:x}, Y", *a),
            Indirect(ref a) => write!(f, "$({:x})", *a),
            IndirectX(ref a) => write!(f, "$({:x}, X)", *a),
            IndirectY(ref a) => write!(f, "$({:x}), Y", *a),
            _ => return Ok(())
        }
    }
}

#[derive(Debug, PartialEq)]
enum OpCode {
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
struct Instruction(OpCode, Addressing);

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} {}", self.0, self.1)
    }
}

#[derive(Debug, PartialEq)]
struct InstructionDecodeError;

#[derive(Debug, PartialEq)]
struct Registers {
    pc: u16,
    sp: u8,
    acc: u8,
    x: u8,
    y: u8,
    status: u8
}

impl Registers {
    fn new() -> Registers {
        Registers {
            pc: 0,
            sp: 0,
            acc: 0,
            x: 0,
            y: 0,
            status: 0
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

fn decode_instruction(mem: &[u8]) -> Result<(usize, Instruction), InstructionDecodeError> {
    use self::Addressing::*;
    use self::OpCode::*;

    let mut iter = mem.iter();
    let len = iter.as_slice().len();
    if let Some(opcode) = iter.next() {
        let ins = match *opcode {
            0x69 => Some(Instruction(Adc, Immediate(decode_u8(&mut iter)?))),
            0x65 => Some(Instruction(Adc, ZeroPage(decode_u8(&mut iter)?))),
            0x75 => Some(Instruction(Adc, ZeroPageX(decode_u8(&mut iter)?))),
            0x6d => Some(Instruction(Adc, Absolute(decode_u16(&mut iter)?))),
            0x7d => Some(Instruction(Adc, AbsoluteX(decode_u16(&mut iter)?))),
            0x79 => Some(Instruction(Adc, AbsoluteY(decode_u16(&mut iter)?))),
            0x61 => Some(Instruction(Adc, IndirectX(decode_u8(&mut iter)?))),
            0x71 => Some(Instruction(Adc, IndirectY(decode_u8(&mut iter)?))),

            0x29 => Some(Instruction(And, Immediate(decode_u8(&mut iter)?))),
            0x25 => Some(Instruction(And, ZeroPage(decode_u8(&mut iter)?))),
            0x35 => Some(Instruction(And, ZeroPageX(decode_u8(&mut iter)?))),
            0x2d => Some(Instruction(And, Absolute(decode_u16(&mut iter)?))),
            0x3d => Some(Instruction(And, AbsoluteX(decode_u16(&mut iter)?))),
            0x39 => Some(Instruction(And, AbsoluteY(decode_u16(&mut iter)?))),
            0x21 => Some(Instruction(And, IndirectX(decode_u8(&mut iter)?))),
            0x31 => Some(Instruction(And, IndirectY(decode_u8(&mut iter)?))),

            0x0a => Some(Instruction(Asl, Accumulator)),
            0x06 => Some(Instruction(Asl, ZeroPage(decode_u8(&mut iter)?))),
            0x16 => Some(Instruction(Asl, ZeroPageX(decode_u8(&mut iter)?))),
            0x0e => Some(Instruction(Asl, Absolute(decode_u16(&mut iter)?))),
            0x1e => Some(Instruction(Asl, AbsoluteX(decode_u16(&mut iter)?))),

            0x90 => Some(Instruction(Bcc, Relative(decode_i8(&mut iter)?))),

            0xb0 => Some(Instruction(Bcs, Relative(decode_i8(&mut iter)?))),

            0xf0 => Some(Instruction(Beq, Relative(decode_i8(&mut iter)?))),

            0x30 => Some(Instruction(Bmi, Relative(decode_i8(&mut iter)?))),

            0xd0 => Some(Instruction(Bne, Relative(decode_i8(&mut iter)?))),

            0x10 => Some(Instruction(Bpl, Relative(decode_i8(&mut iter)?))),

            0x50 => Some(Instruction(Bvc, Relative(decode_i8(&mut iter)?))),

            0x70 => Some(Instruction(Bvs, Relative(decode_i8(&mut iter)?))),

            0x24 => Some(Instruction(Bit, ZeroPage(decode_u8(&mut iter)?))),
            0x2c => Some(Instruction(Bit, Absolute(decode_u16(&mut iter)?))),

            0x00 => Some(Instruction(Brk, Implied)),

            0x18 => Some(Instruction(Clc, Implied)),

            0xd8 => Some(Instruction(Cld, Implied)),

            0x58 => Some(Instruction(Cli, Implied)),

            0xb8 => Some(Instruction(Clv, Implied)),

            0xc9 => Some(Instruction(Cmp, Immediate(decode_u8(&mut iter)?))),
            0xc5 => Some(Instruction(Cmp, ZeroPage(decode_u8(&mut iter)?))),
            0xd5 => Some(Instruction(Cmp, ZeroPageX(decode_u8(&mut iter)?))),
            0xcd => Some(Instruction(Cmp, Absolute(decode_u16(&mut iter)?))),
            0xdd => Some(Instruction(Cmp, AbsoluteX(decode_u16(&mut iter)?))),
            0xd9 => Some(Instruction(Cmp, AbsoluteY(decode_u16(&mut iter)?))),
            0xc1 => Some(Instruction(Cmp, IndirectX(decode_u8(&mut iter)?))),
            0xd1 => Some(Instruction(Cmp, IndirectY(decode_u8(&mut iter)?))),
            
            0xe0 => Some(Instruction(Cpx, Immediate(decode_u8(&mut iter)?))),
            0xe4 => Some(Instruction(Cpx, ZeroPage(decode_u8(&mut iter)?))),
            0xec => Some(Instruction(Cpx, Absolute(decode_u16(&mut iter)?))),
            
            0xc0 => Some(Instruction(Cpy, Immediate(decode_u8(&mut iter)?))),
            0xc4 => Some(Instruction(Cpy, ZeroPage(decode_u8(&mut iter)?))),
            0xcc => Some(Instruction(Cpy, Absolute(decode_u16(&mut iter)?))),
            
            0xc6 => Some(Instruction(Dec, ZeroPage(decode_u8(&mut iter)?))),
            0xd6 => Some(Instruction(Dec, ZeroPageX(decode_u8(&mut iter)?))),
            0xce => Some(Instruction(Dec, Absolute(decode_u16(&mut iter)?))),
            0xde => Some(Instruction(Dec, AbsoluteX(decode_u16(&mut iter)?))),
            
            0xca => Some(Instruction(Dex, Implied)),

            0x88 => Some(Instruction(Dey, Implied)),

            0x49 => Some(Instruction(Eor, Immediate(decode_u8(&mut iter)?))),
            0x45 => Some(Instruction(Eor, ZeroPage(decode_u8(&mut iter)?))),
            0x55 => Some(Instruction(Eor, ZeroPageX(decode_u8(&mut iter)?))),
            0x4d => Some(Instruction(Eor, Absolute(decode_u16(&mut iter)?))),
            0x5d => Some(Instruction(Eor, AbsoluteX(decode_u16(&mut iter)?))),
            0x59 => Some(Instruction(Eor, AbsoluteY(decode_u16(&mut iter)?))),
            0x41 => Some(Instruction(Eor, IndirectX(decode_u8(&mut iter)?))),
            0x51 => Some(Instruction(Eor, IndirectY(decode_u8(&mut iter)?))),
            
            0xe6 => Some(Instruction(Inc, ZeroPage(decode_u8(&mut iter)?))),
            0xf6 => Some(Instruction(Inc, ZeroPageX(decode_u8(&mut iter)?))),
            0xee => Some(Instruction(Inc, Absolute(decode_u16(&mut iter)?))),
            0xfe => Some(Instruction(Inc, AbsoluteX(decode_u16(&mut iter)?))),
            
            0xe8 => Some(Instruction(Inx, Implied)),

            0xc8 => Some(Instruction(Iny, Implied)),

            0x4c => Some(Instruction(Jmp, Absolute(decode_u16(&mut iter)?))),
            0x6c => Some(Instruction(Jmp, Indirect(decode_u16(&mut iter)?))),

            0x20 => Some(Instruction(Jsr, Absolute(decode_u16(&mut iter)?))),

            0xa9 => Some(Instruction(Lda, Immediate(decode_u8(&mut iter)?))),
            0xa5 => Some(Instruction(Lda, ZeroPage(decode_u8(&mut iter)?))),
            0xb5 => Some(Instruction(Lda, ZeroPageX(decode_u8(&mut iter)?))),
            0xad => Some(Instruction(Lda, Absolute(decode_u16(&mut iter)?))),
            0xbd => Some(Instruction(Lda, AbsoluteX(decode_u16(&mut iter)?))),
            0xb9 => Some(Instruction(Lda, AbsoluteY(decode_u16(&mut iter)?))),
            0xa1 => Some(Instruction(Lda, IndirectX(decode_u8(&mut iter)?))),
            0xb1 => Some(Instruction(Lda, IndirectY(decode_u8(&mut iter)?))),
            
            0xa2 => Some(Instruction(Ldx, Immediate(decode_u8(&mut iter)?))),
            0xa6 => Some(Instruction(Ldx, ZeroPage(decode_u8(&mut iter)?))),
            0xb6 => Some(Instruction(Ldx, ZeroPageY(decode_u8(&mut iter)?))),
            0xae => Some(Instruction(Ldx, Absolute(decode_u16(&mut iter)?))),
            0xbe => Some(Instruction(Ldx, AbsoluteY(decode_u16(&mut iter)?))),
            
            0xa0 => Some(Instruction(Ldy, Immediate(decode_u8(&mut iter)?))),
            0xa4 => Some(Instruction(Ldy, ZeroPage(decode_u8(&mut iter)?))),
            0xb4 => Some(Instruction(Ldy, ZeroPageX(decode_u8(&mut iter)?))),
            0xac => Some(Instruction(Ldy, Absolute(decode_u16(&mut iter)?))),
            0xbc => Some(Instruction(Ldy, AbsoluteX(decode_u16(&mut iter)?))),
            
            0x4a => Some(Instruction(Lsr, Accumulator)),
            0x46 => Some(Instruction(Lsr, ZeroPage(decode_u8(&mut iter)?))),
            0x56 => Some(Instruction(Lsr, ZeroPageX(decode_u8(&mut iter)?))),
            0x4e => Some(Instruction(Lsr, Absolute(decode_u16(&mut iter)?))),
            0x5e => Some(Instruction(Lsr, AbsoluteX(decode_u16(&mut iter)?))),

            0xea => Some(Instruction(Nop, Implied)),

            0x09 => Some(Instruction(Ora, Immediate(decode_u8(&mut iter)?))),
            0x05 => Some(Instruction(Ora, ZeroPage(decode_u8(&mut iter)?))),
            0x15 => Some(Instruction(Ora, ZeroPageX(decode_u8(&mut iter)?))),
            0x0d => Some(Instruction(Ora, Absolute(decode_u16(&mut iter)?))),
            0x1d => Some(Instruction(Ora, AbsoluteX(decode_u16(&mut iter)?))),
            0x19 => Some(Instruction(Ora, AbsoluteY(decode_u16(&mut iter)?))),
            0x01 => Some(Instruction(Ora, IndirectX(decode_u8(&mut iter)?))),
            0x11 => Some(Instruction(Ora, IndirectY(decode_u8(&mut iter)?))),
            
            0x48 => Some(Instruction(Pha, Implied)),

            0x08 => Some(Instruction(Php, Implied)),

            0x68 => Some(Instruction(Pla, Implied)),

            0x28 => Some(Instruction(Plp, Implied)),

            0x2a => Some(Instruction(Rol, Accumulator)),
            0x26 => Some(Instruction(Rol, ZeroPage(decode_u8(&mut iter)?))),
            0x36 => Some(Instruction(Rol, ZeroPageX(decode_u8(&mut iter)?))),
            0x2e => Some(Instruction(Rol, Absolute(decode_u16(&mut iter)?))),
            0x3e => Some(Instruction(Rol, AbsoluteX(decode_u16(&mut iter)?))),

            0x6a => Some(Instruction(Ror, Accumulator)),
            0x66 => Some(Instruction(Ror, ZeroPage(decode_u8(&mut iter)?))),
            0x76 => Some(Instruction(Ror, ZeroPageX(decode_u8(&mut iter)?))),
            0x6e => Some(Instruction(Ror, Absolute(decode_u16(&mut iter)?))),
            0x7e => Some(Instruction(Ror, AbsoluteX(decode_u16(&mut iter)?))),

            0x40 => Some(Instruction(Rti, Implied)),

            0x60 => Some(Instruction(Rts, Implied)),

            0xe9 => Some(Instruction(Sbc, Immediate(decode_u8(&mut iter)?))),
            0xe5 => Some(Instruction(Sbc, ZeroPage(decode_u8(&mut iter)?))),
            0xf5 => Some(Instruction(Sbc, ZeroPageX(decode_u8(&mut iter)?))),
            0xed => Some(Instruction(Sbc, Absolute(decode_u16(&mut iter)?))),
            0xfd => Some(Instruction(Sbc, AbsoluteX(decode_u16(&mut iter)?))),
            0xf9 => Some(Instruction(Sbc, AbsoluteY(decode_u16(&mut iter)?))),
            0xe1 => Some(Instruction(Sbc, IndirectX(decode_u8(&mut iter)?))),
            0xf1 => Some(Instruction(Sbc, IndirectY(decode_u8(&mut iter)?))),
            
            0x38 => Some(Instruction(Sec, Implied)),

            0xf8 => Some(Instruction(Sed, Implied)),

            0x78 => Some(Instruction(Sei, Implied)),

            0x85 => Some(Instruction(Sta, ZeroPage(decode_u8(&mut iter)?))),
            0x95 => Some(Instruction(Sta, ZeroPageX(decode_u8(&mut iter)?))),
            0x8d => Some(Instruction(Sta, Absolute(decode_u16(&mut iter)?))),
            0x9d => Some(Instruction(Sta, AbsoluteX(decode_u16(&mut iter)?))),
            0x99 => Some(Instruction(Sta, AbsoluteY(decode_u16(&mut iter)?))),
            0x81 => Some(Instruction(Sta, IndirectX(decode_u8(&mut iter)?))),
            0x91 => Some(Instruction(Sta, IndirectY(decode_u8(&mut iter)?))),
            
            0x86 => Some(Instruction(Stx, ZeroPage(decode_u8(&mut iter)?))),
            0x96 => Some(Instruction(Stx, ZeroPageY(decode_u8(&mut iter)?))),
            0x8e => Some(Instruction(Stx, Absolute(decode_u16(&mut iter)?))),

            0x84 => Some(Instruction(Sty, ZeroPage(decode_u8(&mut iter)?))),
            0x94 => Some(Instruction(Sty, ZeroPageX(decode_u8(&mut iter)?))),
            0x8c => Some(Instruction(Sty, Absolute(decode_u16(&mut iter)?))),

            0xaa => Some(Instruction(Tax, Implied)),

            0xa8 => Some(Instruction(Tay, Implied)),

            0xba => Some(Instruction(Tsx, Implied)),

            0x8a => Some(Instruction(Txa, Implied)),

            0x9a => Some(Instruction(Txs, Implied)),

            0x98 => Some(Instruction(Tya, Implied)),

            _ => None
        };

        if let Some(ins) = ins {
            return Ok((len - iter.as_slice().len(), ins));
        }
    }

    Err(InstructionDecodeError)
}

fn execute_instruction(ins: Instruction, reg: &mut Registers, mem: &mut [u8]) -> usize {
    use self::OpCode::*;
    use self::Addressing::*;

    match ins.0 {
        Adc => match ins.1 {
            Immediate(i) => { 
                let orig = reg.acc;
                let (v, o) = reg.acc.overflowing_add(i);
                reg.acc = v;

                // Carry?
                reg.status |= (o as u8) & 0x01;
                // Zero?
                reg.status |= (((reg.acc == 0) as u8) & 0x01) << 1;
                // Overflow?
                reg.status |= (orig & 0x80) ^ (reg.acc & 0x80) >> 1;
                // Negative?
                reg.status |= reg.acc & 0x80;

                2
            },
            _ => 0
        },
        And => 0,
        Asl => 0,
        Bcc => 0,
        Bcs => 0,
        Beq => 0,
        Bit => 0,
        Bmi => 0,
        Bne => 0,
        Bpl => 0,
        Brk => 0,
        Bvc => 0,
        Bvs => 0,
        Clc => 0,
        Cld => 0,
        Cli => 0,
        Clv => 0,
        Cmp => 0,
        Cpx => 0,
        Cpy => 0,
        Dec => 0,
        Dex => 0,
        Dey => 0,
        Eor => 0,
        Inc => 0,
        Inx => 0,
        Iny => 0,
        Jmp => 0,
        Jsr => 0,
        Lda => 0,
        Ldx => 0,
        Ldy => 0,
        Lsr => 0,
        Nop => 0,
        Ora => 0,
        Pha => 0,
        Php => 0,
        Pla => 0,
        Plp => 0,
        Rol => 0,
        Ror => 0,
        Rti => 0,
        Rts => 0,
        Sbc => 0,
        Sec => 0,
        Sed => 0,
        Sei => 0,
        Sta => 0,
        Stx => 0,
        Sty => 0,
        Tax => 0,
        Tay => 0,
        Tsx => 0,
        Txa => 0,
        Txs => 0,
        Tya => 0,
    }
}

#[cfg(test)]
mod decode_should {
    use super::*;

    #[test]
    fn read_instruction() {
        let mem: &[u8] = &[0x7d, 0x00, 0x80];

        let (bytes, inst) = decode_instruction(mem).unwrap();
        assert_eq!(inst, Instruction(OpCode::Adc, Addressing::AbsoluteX(0x8000)));
        assert_eq!(bytes, 3);
    }

    #[test]
    fn read_multiple_instructions() {
        let mem: &[u8] = &[0x7d, 0x00, 0x80, 0x65, 0x01, 0x71, 0b10000001];
        let (first_bytes, first) = decode_instruction(mem).unwrap();
        let (second_bytes, second) = decode_instruction(&mem[first_bytes..]).unwrap();
        let (_, third) = decode_instruction(&mem[first_bytes+second_bytes..]).unwrap();

        assert_eq!(first, Instruction(OpCode::Adc, Addressing::AbsoluteX(0x8000)));
        assert_eq!(second, Instruction(OpCode::Adc, Addressing::ZeroPage(1)));
        assert_eq!(third, Instruction(OpCode::Adc, Addressing::IndirectY(0b10000001)));
    }
}
