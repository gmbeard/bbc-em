
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

#[derive(Debug)]
struct InstructionDecodeError;

fn decode_instruction(mem: &[u8]) -> Result<(usize, Instruction), InstructionDecodeError> {
    use self::Addressing::*;
    use self::OpCode::*;

    let mut iter = mem.iter();
    let len = iter.as_slice().len();
    if let Some(opcode) = iter.next() {
        let ins = match *opcode {
            0x69 => Some(Instruction(Adc, Immediate(*iter.next().unwrap()))),
            0x65 => Some(Instruction(Adc, ZeroPage(*iter.next().unwrap()))),
            0x75 => Some(Instruction(Adc, ZeroPageX(*iter.next().unwrap()))),
            0x6d => Some(Instruction(Adc, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x7d => Some(Instruction(Adc, AbsoluteX(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x79 => Some(Instruction(Adc, AbsoluteY(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x61 => Some(Instruction(Adc, IndirectX(*iter.next().unwrap()))),
            0x71 => Some(Instruction(Adc, IndirectY(*iter.next().unwrap()))),

            0x29 => Some(Instruction(And, Immediate(*iter.next().unwrap()))),
            0x25 => Some(Instruction(And, ZeroPage(*iter.next().unwrap()))),
            0x35 => Some(Instruction(And, ZeroPageX(*iter.next().unwrap()))),
            0x2d => Some(Instruction(And, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x3d => Some(Instruction(And, AbsoluteX(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x39 => Some(Instruction(And, AbsoluteY(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x21 => Some(Instruction(And, IndirectX(*iter.next().unwrap()))),
            0x31 => Some(Instruction(And, IndirectY(*iter.next().unwrap()))),

            0x0a => Some(Instruction(Asl, Accumulator)),
            0x06 => Some(Instruction(Asl, ZeroPage(*iter.next().unwrap()))),
            0x16 => Some(Instruction(Asl, ZeroPageX(*iter.next().unwrap()))),
            0x0e => Some(Instruction(Asl, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x1e => Some(Instruction(Asl, AbsoluteX(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),

            0x90 => Some(Instruction(Bcc, Relative(*iter.next().unwrap() as i8))),

            0xb0 => Some(Instruction(Bcs, Relative(*iter.next().unwrap() as i8))),

            0xf0 => Some(Instruction(Beq, Relative(*iter.next().unwrap() as i8))),

            0x30 => Some(Instruction(Bmi, Relative(*iter.next().unwrap() as i8))),

            0xd0 => Some(Instruction(Bne, Relative(*iter.next().unwrap() as i8))),

            0x10 => Some(Instruction(Bpl, Relative(*iter.next().unwrap() as i8))),

            0x50 => Some(Instruction(Bvc, Relative(*iter.next().unwrap() as i8))),

            0x70 => Some(Instruction(Bvs, Relative(*iter.next().unwrap() as i8))),

            0x24 => Some(Instruction(Bit, ZeroPage(*iter.next().unwrap()))),
            0x2c => Some(Instruction(Bit, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),

            0x00 => Some(Instruction(Brk, Implied)),

            0x18 => Some(Instruction(Clc, Implied)),

            0xd8 => Some(Instruction(Cld, Implied)),

            0x58 => Some(Instruction(Cli, Implied)),

            0xb8 => Some(Instruction(Clv, Implied)),

            0xc9 => Some(Instruction(Cmp, Immediate(*iter.next().unwrap()))),
            0xc5 => Some(Instruction(Cmp, ZeroPage(*iter.next().unwrap()))),
            0xd5 => Some(Instruction(Cmp, ZeroPageX(*iter.next().unwrap()))),
            0xcd => Some(Instruction(Cmp, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0xdd => Some(Instruction(Cmp, AbsoluteX(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0xd9 => Some(Instruction(Cmp, AbsoluteY(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0xc1 => Some(Instruction(Cmp, IndirectX(*iter.next().unwrap()))),
            0xd1 => Some(Instruction(Cmp, IndirectY(*iter.next().unwrap()))),
            
            0xe0 => Some(Instruction(Cpx, Immediate(*iter.next().unwrap()))),
            0xe4 => Some(Instruction(Cpx, ZeroPage(*iter.next().unwrap()))),
            0xec => Some(Instruction(Cpx, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            
            0xc0 => Some(Instruction(Cpy, Immediate(*iter.next().unwrap()))),
            0xc4 => Some(Instruction(Cpy, ZeroPage(*iter.next().unwrap()))),
            0xcc => Some(Instruction(Cpy, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            
            0xc6 => Some(Instruction(Dec, ZeroPage(*iter.next().unwrap()))),
            0xd6 => Some(Instruction(Dec, ZeroPageX(*iter.next().unwrap()))),
            0xce => Some(Instruction(Dec, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0xde => Some(Instruction(Dec, AbsoluteX(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            
            0xca => Some(Instruction(Dex, Implied)),

            0x88 => Some(Instruction(Dey, Implied)),

            0x49 => Some(Instruction(Eor, Immediate(*iter.next().unwrap()))),
            0x45 => Some(Instruction(Eor, ZeroPage(*iter.next().unwrap()))),
            0x55 => Some(Instruction(Eor, ZeroPageX(*iter.next().unwrap()))),
            0x4d => Some(Instruction(Eor, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x5d => Some(Instruction(Eor, AbsoluteX(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x59 => Some(Instruction(Eor, AbsoluteY(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x41 => Some(Instruction(Eor, IndirectX(*iter.next().unwrap()))),
            0x51 => Some(Instruction(Eor, IndirectY(*iter.next().unwrap()))),
            
            0xe6 => Some(Instruction(Inc, ZeroPage(*iter.next().unwrap()))),
            0xf6 => Some(Instruction(Inc, ZeroPageX(*iter.next().unwrap()))),
            0xee => Some(Instruction(Inc, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0xfe => Some(Instruction(Inc, AbsoluteX(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            
            0xe8 => Some(Instruction(Inx, Implied)),

            0xc8 => Some(Instruction(Iny, Implied)),

            0x4c => Some(Instruction(Jmp, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x6c => Some(Instruction(Jmp, Indirect(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),

            0x20 => Some(Instruction(Jsr, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),

            0xa9 => Some(Instruction(Lda, Immediate(*iter.next().unwrap()))),
            0xa5 => Some(Instruction(Lda, ZeroPage(*iter.next().unwrap()))),
            0xb5 => Some(Instruction(Lda, ZeroPageX(*iter.next().unwrap()))),
            0xad => Some(Instruction(Lda, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0xbd => Some(Instruction(Lda, AbsoluteX(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0xb9 => Some(Instruction(Lda, AbsoluteY(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0xa1 => Some(Instruction(Lda, IndirectX(*iter.next().unwrap()))),
            0xb1 => Some(Instruction(Lda, IndirectY(*iter.next().unwrap()))),
            
            0xa2 => Some(Instruction(Ldx, Immediate(*iter.next().unwrap()))),
            0xa6 => Some(Instruction(Ldx, ZeroPage(*iter.next().unwrap()))),
            0xb6 => Some(Instruction(Ldx, ZeroPageY(*iter.next().unwrap()))),
            0xae => Some(Instruction(Ldx, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0xbe => Some(Instruction(Ldx, AbsoluteY(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            
            0xa0 => Some(Instruction(Ldy, Immediate(*iter.next().unwrap()))),
            0xa4 => Some(Instruction(Ldy, ZeroPage(*iter.next().unwrap()))),
            0xb4 => Some(Instruction(Ldy, ZeroPageX(*iter.next().unwrap()))),
            0xac => Some(Instruction(Ldy, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0xbc => Some(Instruction(Ldy, AbsoluteX(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            
            0x4a => Some(Instruction(Lsr, Accumulator)),
            0x46 => Some(Instruction(Lsr, ZeroPage(*iter.next().unwrap()))),
            0x56 => Some(Instruction(Lsr, ZeroPageX(*iter.next().unwrap()))),
            0x4e => Some(Instruction(Lsr, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x5e => Some(Instruction(Lsr, AbsoluteX(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),

            0xea => Some(Instruction(Nop, Implied)),

            0x09 => Some(Instruction(Ora, Immediate(*iter.next().unwrap()))),
            0x05 => Some(Instruction(Ora, ZeroPage(*iter.next().unwrap()))),
            0x15 => Some(Instruction(Ora, ZeroPageX(*iter.next().unwrap()))),
            0x0d => Some(Instruction(Ora, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x1d => Some(Instruction(Ora, AbsoluteX(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x19 => Some(Instruction(Ora, AbsoluteY(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x01 => Some(Instruction(Ora, IndirectX(*iter.next().unwrap()))),
            0x11 => Some(Instruction(Ora, IndirectY(*iter.next().unwrap()))),
            
            0x48 => Some(Instruction(Pha, Implied)),

            0x08 => Some(Instruction(Php, Implied)),

            0x68 => Some(Instruction(Pla, Implied)),

            0x28 => Some(Instruction(Plp, Implied)),

            0x2a => Some(Instruction(Rol, Accumulator)),
            0x26 => Some(Instruction(Rol, ZeroPage(*iter.next().unwrap()))),
            0x36 => Some(Instruction(Rol, ZeroPageX(*iter.next().unwrap()))),
            0x2e => Some(Instruction(Rol, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x3e => Some(Instruction(Rol, AbsoluteX(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),

            0x6a => Some(Instruction(Ror, Accumulator)),
            0x66 => Some(Instruction(Ror, ZeroPage(*iter.next().unwrap()))),
            0x76 => Some(Instruction(Ror, ZeroPageX(*iter.next().unwrap()))),
            0x6e => Some(Instruction(Ror, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x7e => Some(Instruction(Ror, AbsoluteX(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),

            0x40 => Some(Instruction(Rti, Implied)),

            0x60 => Some(Instruction(Rts, Implied)),

            0xe9 => Some(Instruction(Sbc, Immediate(*iter.next().unwrap()))),
            0xe5 => Some(Instruction(Sbc, ZeroPage(*iter.next().unwrap()))),
            0xf5 => Some(Instruction(Sbc, ZeroPageX(*iter.next().unwrap()))),
            0xed => Some(Instruction(Sbc, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0xfd => Some(Instruction(Sbc, AbsoluteX(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0xf9 => Some(Instruction(Sbc, AbsoluteY(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0xe1 => Some(Instruction(Sbc, IndirectX(*iter.next().unwrap()))),
            0xf1 => Some(Instruction(Sbc, IndirectY(*iter.next().unwrap()))),
            
            0x38 => Some(Instruction(Sec, Implied)),

            0xf8 => Some(Instruction(Sed, Implied)),

            0x78 => Some(Instruction(Sei, Implied)),

            0x85 => Some(Instruction(Sta, ZeroPage(*iter.next().unwrap()))),
            0x95 => Some(Instruction(Sta, ZeroPageX(*iter.next().unwrap()))),
            0x8d => Some(Instruction(Sta, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x9d => Some(Instruction(Sta, AbsoluteX(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x99 => Some(Instruction(Sta, AbsoluteY(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),
            0x81 => Some(Instruction(Sta, IndirectX(*iter.next().unwrap()))),
            0x91 => Some(Instruction(Sta, IndirectY(*iter.next().unwrap()))),
            
            0x86 => Some(Instruction(Stx, ZeroPage(*iter.next().unwrap()))),
            0x96 => Some(Instruction(Stx, ZeroPageY(*iter.next().unwrap()))),
            0x8e => Some(Instruction(Stx, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),

            0x84 => Some(Instruction(Sty, ZeroPage(*iter.next().unwrap()))),
            0x94 => Some(Instruction(Sty, ZeroPageX(*iter.next().unwrap()))),
            0x8c => Some(Instruction(Sty, Absolute(*iter.next().unwrap() as u16 | (*iter.next().unwrap() as u16) << 8))),

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
        let mem: &[u8] = &[0x7d, 0x00, 0x80, 0x65, 0x01, 0x71, 0b00000001];
        let (first_bytes, first) = decode_instruction(mem).unwrap();
        let (second_bytes, second) = decode_instruction(&mem[first_bytes..]).unwrap();
        let (_, third) = decode_instruction(&mem[first_bytes+second_bytes..]).unwrap();

        assert_eq!(first, Instruction(OpCode::Adc, Addressing::AbsoluteX(0x8000)));
        assert_eq!(second, Instruction(OpCode::Adc, Addressing::ZeroPage(1)));
        assert_eq!(third, Instruction(OpCode::Adc, Addressing::IndirectY(1)));
    }
}
