//! Contains decoded instruction indexed by opcodes.
//! This provides us with constant time decoding given how
//! non-uniformly operands are encoded in different instructions and
//! have different number of cycles.
//! This does not require any complex logic to decode and is simple to understand.

use crate::cpu::isa::{Cond, Instr, Operand, Opcode, Reg};

macro_rules! ins {
    ($op:expr) => {
        Instr {
            op: $op,
            op1: Operand::Absent,
            op2: Operand::Absent,
        }
    };
    ($op:expr, $op1:expr) => {
        Instr {
            op: $op,
            op1: $op1,
            op2: Operand::Absent,
        }
    };
    ($op:expr, $op1:expr, $op2:expr) => {
        Instr {
            op: $op,
            op1: $op1,
            op2: $op2,
        }
    };
}

use Opcode::*;
type Op = Operand;

// Generated by: gen/genins.py
pub(crate) const INSTR_TABLE: [Instr; 256] = {
    let mut a = [ins!(Illegal); 256];
    a[0x00] = ins!(Nop); // #[4]
    a[0x01] = ins!(Ld, Op::Reg(Reg::BC), Op::U16(0)); // #[12]
    a[0x02] = ins!(Ld, Op::RegMem(Reg::BC), Op::Reg(Reg::A)); // #[8]
    a[0x03] = ins!(Inc, Op::Reg(Reg::BC)); // #[8]
    a[0x04] = ins!(Inc, Op::Reg(Reg::B)); // #[4]
    a[0x05] = ins!(Dec, Op::Reg(Reg::B)); // #[4]
    a[0x06] = ins!(Ld, Op::Reg(Reg::B), Op::U8(0)); // #[8]
    a[0x07] = ins!(Rlca); // #[4]
    a[0x08] = ins!(Ld, Op::A16(0), Op::Reg(Reg::SP)); // #[20]
    a[0x09] = ins!(Add, Op::Reg(Reg::HL), Op::Reg(Reg::BC)); // #[8]
    a[0x0A] = ins!(Ld, Op::Reg(Reg::A), Op::RegMem(Reg::BC)); // #[8]
    a[0x0B] = ins!(Dec, Op::Reg(Reg::BC)); // #[8]
    a[0x0C] = ins!(Inc, Op::Reg(Reg::C)); // #[4]
    a[0x0D] = ins!(Dec, Op::Reg(Reg::C)); // #[4]
    a[0x0E] = ins!(Ld, Op::Reg(Reg::C), Op::U8(0)); // #[8]
    a[0x0F] = ins!(Rrca); // #[4]
    a[0x10] = ins!(Stop, Op::U8(0)); // #[4]
    a[0x11] = ins!(Ld, Op::Reg(Reg::DE), Op::U16(0)); // #[12]
    a[0x12] = ins!(Ld, Op::RegMem(Reg::DE), Op::Reg(Reg::A)); // #[8]
    a[0x13] = ins!(Inc, Op::Reg(Reg::DE)); // #[8]
    a[0x14] = ins!(Inc, Op::Reg(Reg::D)); // #[4]
    a[0x15] = ins!(Dec, Op::Reg(Reg::D)); // #[4]
    a[0x16] = ins!(Ld, Op::Reg(Reg::D), Op::U8(0)); // #[8]
    a[0x17] = ins!(Rla); // #[4]
    a[0x18] = ins!(Jr, Op::I8(0)); // #[12]
    a[0x19] = ins!(Add, Op::Reg(Reg::HL), Op::Reg(Reg::DE)); // #[8]
    a[0x1A] = ins!(Ld, Op::Reg(Reg::A), Op::RegMem(Reg::DE)); // #[8]
    a[0x1B] = ins!(Dec, Op::Reg(Reg::DE)); // #[8]
    a[0x1C] = ins!(Inc, Op::Reg(Reg::E)); // #[4]
    a[0x1D] = ins!(Dec, Op::Reg(Reg::E)); // #[4]
    a[0x1E] = ins!(Ld, Op::Reg(Reg::E), Op::U8(0)); // #[8]
    a[0x1F] = ins!(Rra); // #[4]
    a[0x20] = ins!(Jr, Op::Cond(Cond::NZ), Op::I8(0)); // #[12, 8]
    a[0x21] = ins!(Ld, Op::Reg(Reg::HL), Op::U16(0)); // #[12]
    a[0x22] = ins!(Ld, Op::RegMem(Reg::HLinc), Op::Reg(Reg::A)); // #[8]
    a[0x23] = ins!(Inc, Op::Reg(Reg::HL)); // #[8]
    a[0x24] = ins!(Inc, Op::Reg(Reg::H)); // #[4]
    a[0x25] = ins!(Dec, Op::Reg(Reg::H)); // #[4]
    a[0x26] = ins!(Ld, Op::Reg(Reg::H), Op::U8(0)); // #[8]
    a[0x27] = ins!(Daa); // #[4]
    a[0x28] = ins!(Jr, Op::Cond(Cond::Z), Op::I8(0)); // #[12, 8]
    a[0x29] = ins!(Add, Op::Reg(Reg::HL), Op::Reg(Reg::HL)); // #[8]
    a[0x2A] = ins!(Ld, Op::Reg(Reg::A), Op::RegMem(Reg::HLinc)); // #[8]
    a[0x2B] = ins!(Dec, Op::Reg(Reg::HL)); // #[8]
    a[0x2C] = ins!(Inc, Op::Reg(Reg::L)); // #[4]
    a[0x2D] = ins!(Dec, Op::Reg(Reg::L)); // #[4]
    a[0x2E] = ins!(Ld, Op::Reg(Reg::L), Op::U8(0)); // #[8]
    a[0x2F] = ins!(Cpl); // #[4]
    a[0x30] = ins!(Jr, Op::Cond(Cond::NC), Op::I8(0)); // #[12, 8]
    a[0x31] = ins!(Ld, Op::Reg(Reg::SP), Op::U16(0)); // #[12]
    a[0x32] = ins!(Ld, Op::RegMem(Reg::HLdec), Op::Reg(Reg::A)); // #[8]
    a[0x33] = ins!(Inc, Op::Reg(Reg::SP)); // #[8]
    a[0x34] = ins!(Inc, Op::RegMem(Reg::HL)); // #[12]
    a[0x35] = ins!(Dec, Op::RegMem(Reg::HL)); // #[12]
    a[0x36] = ins!(Ld, Op::RegMem(Reg::HL), Op::U8(0)); // #[12]
    a[0x37] = ins!(Scf); // #[4]
    a[0x38] = ins!(Jr, Op::Cond(Cond::C), Op::I8(0)); // #[12, 8]
    a[0x39] = ins!(Add, Op::Reg(Reg::HL), Op::Reg(Reg::SP)); // #[8]
    a[0x3A] = ins!(Ld, Op::Reg(Reg::A), Op::RegMem(Reg::HLdec)); // #[8]
    a[0x3B] = ins!(Dec, Op::Reg(Reg::SP)); // #[8]
    a[0x3C] = ins!(Inc, Op::Reg(Reg::A)); // #[4]
    a[0x3D] = ins!(Dec, Op::Reg(Reg::A)); // #[4]
    a[0x3E] = ins!(Ld, Op::Reg(Reg::A), Op::U8(0)); // #[8]
    a[0x3F] = ins!(Ccf); // #[4]
    a[0x40] = ins!(Ld, Op::Reg(Reg::B), Op::Reg(Reg::B)); // #[4]
    a[0x41] = ins!(Ld, Op::Reg(Reg::B), Op::Reg(Reg::C)); // #[4]
    a[0x42] = ins!(Ld, Op::Reg(Reg::B), Op::Reg(Reg::D)); // #[4]
    a[0x43] = ins!(Ld, Op::Reg(Reg::B), Op::Reg(Reg::E)); // #[4]
    a[0x44] = ins!(Ld, Op::Reg(Reg::B), Op::Reg(Reg::H)); // #[4]
    a[0x45] = ins!(Ld, Op::Reg(Reg::B), Op::Reg(Reg::L)); // #[4]
    a[0x46] = ins!(Ld, Op::Reg(Reg::B), Op::RegMem(Reg::HL)); // #[8]
    a[0x47] = ins!(Ld, Op::Reg(Reg::B), Op::Reg(Reg::A)); // #[4]
    a[0x48] = ins!(Ld, Op::Reg(Reg::C), Op::Reg(Reg::B)); // #[4]
    a[0x49] = ins!(Ld, Op::Reg(Reg::C), Op::Reg(Reg::C)); // #[4]
    a[0x4A] = ins!(Ld, Op::Reg(Reg::C), Op::Reg(Reg::D)); // #[4]
    a[0x4B] = ins!(Ld, Op::Reg(Reg::C), Op::Reg(Reg::E)); // #[4]
    a[0x4C] = ins!(Ld, Op::Reg(Reg::C), Op::Reg(Reg::H)); // #[4]
    a[0x4D] = ins!(Ld, Op::Reg(Reg::C), Op::Reg(Reg::L)); // #[4]
    a[0x4E] = ins!(Ld, Op::Reg(Reg::C), Op::RegMem(Reg::HL)); // #[8]
    a[0x4F] = ins!(Ld, Op::Reg(Reg::C), Op::Reg(Reg::A)); // #[4]
    a[0x50] = ins!(Ld, Op::Reg(Reg::D), Op::Reg(Reg::B)); // #[4]
    a[0x51] = ins!(Ld, Op::Reg(Reg::D), Op::Reg(Reg::C)); // #[4]
    a[0x52] = ins!(Ld, Op::Reg(Reg::D), Op::Reg(Reg::D)); // #[4]
    a[0x53] = ins!(Ld, Op::Reg(Reg::D), Op::Reg(Reg::E)); // #[4]
    a[0x54] = ins!(Ld, Op::Reg(Reg::D), Op::Reg(Reg::H)); // #[4]
    a[0x55] = ins!(Ld, Op::Reg(Reg::D), Op::Reg(Reg::L)); // #[4]
    a[0x56] = ins!(Ld, Op::Reg(Reg::D), Op::RegMem(Reg::HL)); // #[8]
    a[0x57] = ins!(Ld, Op::Reg(Reg::D), Op::Reg(Reg::A)); // #[4]
    a[0x58] = ins!(Ld, Op::Reg(Reg::E), Op::Reg(Reg::B)); // #[4]
    a[0x59] = ins!(Ld, Op::Reg(Reg::E), Op::Reg(Reg::C)); // #[4]
    a[0x5A] = ins!(Ld, Op::Reg(Reg::E), Op::Reg(Reg::D)); // #[4]
    a[0x5B] = ins!(Ld, Op::Reg(Reg::E), Op::Reg(Reg::E)); // #[4]
    a[0x5C] = ins!(Ld, Op::Reg(Reg::E), Op::Reg(Reg::H)); // #[4]
    a[0x5D] = ins!(Ld, Op::Reg(Reg::E), Op::Reg(Reg::L)); // #[4]
    a[0x5E] = ins!(Ld, Op::Reg(Reg::E), Op::RegMem(Reg::HL)); // #[8]
    a[0x5F] = ins!(Ld, Op::Reg(Reg::E), Op::Reg(Reg::A)); // #[4]
    a[0x60] = ins!(Ld, Op::Reg(Reg::H), Op::Reg(Reg::B)); // #[4]
    a[0x61] = ins!(Ld, Op::Reg(Reg::H), Op::Reg(Reg::C)); // #[4]
    a[0x62] = ins!(Ld, Op::Reg(Reg::H), Op::Reg(Reg::D)); // #[4]
    a[0x63] = ins!(Ld, Op::Reg(Reg::H), Op::Reg(Reg::E)); // #[4]
    a[0x64] = ins!(Ld, Op::Reg(Reg::H), Op::Reg(Reg::H)); // #[4]
    a[0x65] = ins!(Ld, Op::Reg(Reg::H), Op::Reg(Reg::L)); // #[4]
    a[0x66] = ins!(Ld, Op::Reg(Reg::H), Op::RegMem(Reg::HL)); // #[8]
    a[0x67] = ins!(Ld, Op::Reg(Reg::H), Op::Reg(Reg::A)); // #[4]
    a[0x68] = ins!(Ld, Op::Reg(Reg::L), Op::Reg(Reg::B)); // #[4]
    a[0x69] = ins!(Ld, Op::Reg(Reg::L), Op::Reg(Reg::C)); // #[4]
    a[0x6A] = ins!(Ld, Op::Reg(Reg::L), Op::Reg(Reg::D)); // #[4]
    a[0x6B] = ins!(Ld, Op::Reg(Reg::L), Op::Reg(Reg::E)); // #[4]
    a[0x6C] = ins!(Ld, Op::Reg(Reg::L), Op::Reg(Reg::H)); // #[4]
    a[0x6D] = ins!(Ld, Op::Reg(Reg::L), Op::Reg(Reg::L)); // #[4]
    a[0x6E] = ins!(Ld, Op::Reg(Reg::L), Op::RegMem(Reg::HL)); // #[8]
    a[0x6F] = ins!(Ld, Op::Reg(Reg::L), Op::Reg(Reg::A)); // #[4]
    a[0x70] = ins!(Ld, Op::RegMem(Reg::HL), Op::Reg(Reg::B)); // #[8]
    a[0x71] = ins!(Ld, Op::RegMem(Reg::HL), Op::Reg(Reg::C)); // #[8]
    a[0x72] = ins!(Ld, Op::RegMem(Reg::HL), Op::Reg(Reg::D)); // #[8]
    a[0x73] = ins!(Ld, Op::RegMem(Reg::HL), Op::Reg(Reg::E)); // #[8]
    a[0x74] = ins!(Ld, Op::RegMem(Reg::HL), Op::Reg(Reg::H)); // #[8]
    a[0x75] = ins!(Ld, Op::RegMem(Reg::HL), Op::Reg(Reg::L)); // #[8]
    a[0x76] = ins!(Halt); // #[4]
    a[0x77] = ins!(Ld, Op::RegMem(Reg::HL), Op::Reg(Reg::A)); // #[8]
    a[0x78] = ins!(Ld, Op::Reg(Reg::A), Op::Reg(Reg::B)); // #[4]
    a[0x79] = ins!(Ld, Op::Reg(Reg::A), Op::Reg(Reg::C)); // #[4]
    a[0x7A] = ins!(Ld, Op::Reg(Reg::A), Op::Reg(Reg::D)); // #[4]
    a[0x7B] = ins!(Ld, Op::Reg(Reg::A), Op::Reg(Reg::E)); // #[4]
    a[0x7C] = ins!(Ld, Op::Reg(Reg::A), Op::Reg(Reg::H)); // #[4]
    a[0x7D] = ins!(Ld, Op::Reg(Reg::A), Op::Reg(Reg::L)); // #[4]
    a[0x7E] = ins!(Ld, Op::Reg(Reg::A), Op::RegMem(Reg::HL)); // #[8]
    a[0x7F] = ins!(Ld, Op::Reg(Reg::A), Op::Reg(Reg::A)); // #[4]
    a[0x80] = ins!(Add, Op::Reg(Reg::A), Op::Reg(Reg::B)); // #[4]
    a[0x81] = ins!(Add, Op::Reg(Reg::A), Op::Reg(Reg::C)); // #[4]
    a[0x82] = ins!(Add, Op::Reg(Reg::A), Op::Reg(Reg::D)); // #[4]
    a[0x83] = ins!(Add, Op::Reg(Reg::A), Op::Reg(Reg::E)); // #[4]
    a[0x84] = ins!(Add, Op::Reg(Reg::A), Op::Reg(Reg::H)); // #[4]
    a[0x85] = ins!(Add, Op::Reg(Reg::A), Op::Reg(Reg::L)); // #[4]
    a[0x86] = ins!(Add, Op::Reg(Reg::A), Op::RegMem(Reg::HL)); // #[8]
    a[0x87] = ins!(Add, Op::Reg(Reg::A), Op::Reg(Reg::A)); // #[4]
    a[0x88] = ins!(Adc, Op::Reg(Reg::A), Op::Reg(Reg::B)); // #[4]
    a[0x89] = ins!(Adc, Op::Reg(Reg::A), Op::Reg(Reg::C)); // #[4]
    a[0x8A] = ins!(Adc, Op::Reg(Reg::A), Op::Reg(Reg::D)); // #[4]
    a[0x8B] = ins!(Adc, Op::Reg(Reg::A), Op::Reg(Reg::E)); // #[4]
    a[0x8C] = ins!(Adc, Op::Reg(Reg::A), Op::Reg(Reg::H)); // #[4]
    a[0x8D] = ins!(Adc, Op::Reg(Reg::A), Op::Reg(Reg::L)); // #[4]
    a[0x8E] = ins!(Adc, Op::Reg(Reg::A), Op::RegMem(Reg::HL)); // #[8]
    a[0x8F] = ins!(Adc, Op::Reg(Reg::A), Op::Reg(Reg::A)); // #[4]
    a[0x90] = ins!(Sub, Op::Reg(Reg::A), Op::Reg(Reg::B)); // #[4]
    a[0x91] = ins!(Sub, Op::Reg(Reg::A), Op::Reg(Reg::C)); // #[4]
    a[0x92] = ins!(Sub, Op::Reg(Reg::A), Op::Reg(Reg::D)); // #[4]
    a[0x93] = ins!(Sub, Op::Reg(Reg::A), Op::Reg(Reg::E)); // #[4]
    a[0x94] = ins!(Sub, Op::Reg(Reg::A), Op::Reg(Reg::H)); // #[4]
    a[0x95] = ins!(Sub, Op::Reg(Reg::A), Op::Reg(Reg::L)); // #[4]
    a[0x96] = ins!(Sub, Op::Reg(Reg::A), Op::RegMem(Reg::HL)); // #[8]
    a[0x97] = ins!(Sub, Op::Reg(Reg::A), Op::Reg(Reg::A)); // #[4]
    a[0x98] = ins!(Sbc, Op::Reg(Reg::A), Op::Reg(Reg::B)); // #[4]
    a[0x99] = ins!(Sbc, Op::Reg(Reg::A), Op::Reg(Reg::C)); // #[4]
    a[0x9A] = ins!(Sbc, Op::Reg(Reg::A), Op::Reg(Reg::D)); // #[4]
    a[0x9B] = ins!(Sbc, Op::Reg(Reg::A), Op::Reg(Reg::E)); // #[4]
    a[0x9C] = ins!(Sbc, Op::Reg(Reg::A), Op::Reg(Reg::H)); // #[4]
    a[0x9D] = ins!(Sbc, Op::Reg(Reg::A), Op::Reg(Reg::L)); // #[4]
    a[0x9E] = ins!(Sbc, Op::Reg(Reg::A), Op::RegMem(Reg::HL)); // #[8]
    a[0x9F] = ins!(Sbc, Op::Reg(Reg::A), Op::Reg(Reg::A)); // #[4]
    a[0xA0] = ins!(And, Op::Reg(Reg::A), Op::Reg(Reg::B)); // #[4]
    a[0xA1] = ins!(And, Op::Reg(Reg::A), Op::Reg(Reg::C)); // #[4]
    a[0xA2] = ins!(And, Op::Reg(Reg::A), Op::Reg(Reg::D)); // #[4]
    a[0xA3] = ins!(And, Op::Reg(Reg::A), Op::Reg(Reg::E)); // #[4]
    a[0xA4] = ins!(And, Op::Reg(Reg::A), Op::Reg(Reg::H)); // #[4]
    a[0xA5] = ins!(And, Op::Reg(Reg::A), Op::Reg(Reg::L)); // #[4]
    a[0xA6] = ins!(And, Op::Reg(Reg::A), Op::RegMem(Reg::HL)); // #[8]
    a[0xA7] = ins!(And, Op::Reg(Reg::A), Op::Reg(Reg::A)); // #[4]
    a[0xA8] = ins!(Xor, Op::Reg(Reg::A), Op::Reg(Reg::B)); // #[4]
    a[0xA9] = ins!(Xor, Op::Reg(Reg::A), Op::Reg(Reg::C)); // #[4]
    a[0xAA] = ins!(Xor, Op::Reg(Reg::A), Op::Reg(Reg::D)); // #[4]
    a[0xAB] = ins!(Xor, Op::Reg(Reg::A), Op::Reg(Reg::E)); // #[4]
    a[0xAC] = ins!(Xor, Op::Reg(Reg::A), Op::Reg(Reg::H)); // #[4]
    a[0xAD] = ins!(Xor, Op::Reg(Reg::A), Op::Reg(Reg::L)); // #[4]
    a[0xAE] = ins!(Xor, Op::Reg(Reg::A), Op::RegMem(Reg::HL)); // #[8]
    a[0xAF] = ins!(Xor, Op::Reg(Reg::A), Op::Reg(Reg::A)); // #[4]
    a[0xB0] = ins!(Or, Op::Reg(Reg::A), Op::Reg(Reg::B)); // #[4]
    a[0xB1] = ins!(Or, Op::Reg(Reg::A), Op::Reg(Reg::C)); // #[4]
    a[0xB2] = ins!(Or, Op::Reg(Reg::A), Op::Reg(Reg::D)); // #[4]
    a[0xB3] = ins!(Or, Op::Reg(Reg::A), Op::Reg(Reg::E)); // #[4]
    a[0xB4] = ins!(Or, Op::Reg(Reg::A), Op::Reg(Reg::H)); // #[4]
    a[0xB5] = ins!(Or, Op::Reg(Reg::A), Op::Reg(Reg::L)); // #[4]
    a[0xB6] = ins!(Or, Op::Reg(Reg::A), Op::RegMem(Reg::HL)); // #[8]
    a[0xB7] = ins!(Or, Op::Reg(Reg::A), Op::Reg(Reg::A)); // #[4]
    a[0xB8] = ins!(Cp, Op::Reg(Reg::A), Op::Reg(Reg::B)); // #[4]
    a[0xB9] = ins!(Cp, Op::Reg(Reg::A), Op::Reg(Reg::C)); // #[4]
    a[0xBA] = ins!(Cp, Op::Reg(Reg::A), Op::Reg(Reg::D)); // #[4]
    a[0xBB] = ins!(Cp, Op::Reg(Reg::A), Op::Reg(Reg::E)); // #[4]
    a[0xBC] = ins!(Cp, Op::Reg(Reg::A), Op::Reg(Reg::H)); // #[4]
    a[0xBD] = ins!(Cp, Op::Reg(Reg::A), Op::Reg(Reg::L)); // #[4]
    a[0xBE] = ins!(Cp, Op::Reg(Reg::A), Op::RegMem(Reg::HL)); // #[8]
    a[0xBF] = ins!(Cp, Op::Reg(Reg::A), Op::Reg(Reg::A)); // #[4]
    a[0xC0] = ins!(Ret, Op::Cond(Cond::NZ)); // #[20, 8]
    a[0xC1] = ins!(Pop, Op::Reg(Reg::BC)); // #[12]
    a[0xC2] = ins!(Jp, Op::Cond(Cond::NZ), Op::U16(0)); // #[16, 12]
    a[0xC3] = ins!(Jp, Op::U16(0)); // #[16]
    a[0xC4] = ins!(Call, Op::Cond(Cond::NZ), Op::U16(0)); // #[24, 12]
    a[0xC5] = ins!(Push, Op::Reg(Reg::BC)); // #[16]
    a[0xC6] = ins!(Add, Op::Reg(Reg::A), Op::U8(0)); // #[8]
    a[0xC7] = ins!(Rst, Op::Tgt(0x00)); // #[16]
    a[0xC8] = ins!(Ret, Op::Cond(Cond::Z)); // #[20, 8]
    a[0xC9] = ins!(Ret); // #[16]
    a[0xCA] = ins!(Jp, Op::Cond(Cond::Z), Op::U16(0)); // #[16, 12]
    a[0xCB] = ins!(Prefix); // #[4]
    a[0xCC] = ins!(Call, Op::Cond(Cond::Z), Op::U16(0)); // #[24, 12]
    a[0xCD] = ins!(Call, Op::U16(0)); // #[24]
    a[0xCE] = ins!(Adc, Op::Reg(Reg::A), Op::U8(0)); // #[8]
    a[0xCF] = ins!(Rst, Op::Tgt(0x08)); // #[16]
    a[0xD0] = ins!(Ret, Op::Cond(Cond::NC)); // #[20, 8]
    a[0xD1] = ins!(Pop, Op::Reg(Reg::DE)); // #[12]
    a[0xD2] = ins!(Jp, Op::Cond(Cond::NC), Op::U16(0)); // #[16, 12]
    a[0xD3] = ins!(Illegal); // #[4]
    a[0xD4] = ins!(Call, Op::Cond(Cond::NC), Op::U16(0)); // #[24, 12]
    a[0xD5] = ins!(Push, Op::Reg(Reg::DE)); // #[16]
    a[0xD6] = ins!(Sub, Op::Reg(Reg::A), Op::U8(0)); // #[8]
    a[0xD7] = ins!(Rst, Op::Tgt(0x10)); // #[16]
    a[0xD8] = ins!(Ret, Op::Cond(Cond::C)); // #[20, 8]
    a[0xD9] = ins!(Reti); // #[16]
    a[0xDA] = ins!(Jp, Op::Cond(Cond::C), Op::U16(0)); // #[16, 12]
    a[0xDB] = ins!(Illegal); // #[4]
    a[0xDC] = ins!(Call, Op::Cond(Cond::C), Op::U16(0)); // #[24, 12]
    a[0xDD] = ins!(Illegal); // #[4]
    a[0xDE] = ins!(Sbc, Op::Reg(Reg::A), Op::U8(0)); // #[8]
    a[0xDF] = ins!(Rst, Op::Tgt(0x18)); // #[16]
    a[0xE0] = ins!(Ldh, Op::A8(0), Op::Reg(Reg::A)); // #[12]
    a[0xE1] = ins!(Pop, Op::Reg(Reg::HL)); // #[12]
    a[0xE2] = ins!(Ld, Op::RegMem(Reg::C), Op::Reg(Reg::A)); // #[8]
    a[0xE3] = ins!(Illegal); // #[4]
    a[0xE4] = ins!(Illegal); // #[4]
    a[0xE5] = ins!(Push, Op::Reg(Reg::HL)); // #[16]
    a[0xE6] = ins!(And, Op::Reg(Reg::A), Op::U8(0)); // #[8]
    a[0xE7] = ins!(Rst, Op::Tgt(0x20)); // #[16]
    a[0xE8] = ins!(Add, Op::Reg(Reg::SP), Op::I8(0)); // #[16]
    a[0xE9] = ins!(Jp, Op::Reg(Reg::HL)); // #[4]
    a[0xEA] = ins!(Ld, Op::A16(0), Op::Reg(Reg::A)); // #[16]
    a[0xEB] = ins!(Illegal); // #[4]
    a[0xEC] = ins!(Illegal); // #[4]
    a[0xED] = ins!(Illegal); // #[4]
    a[0xEE] = ins!(Xor, Op::Reg(Reg::A), Op::U8(0)); // #[8]
    a[0xEF] = ins!(Rst, Op::Tgt(0x28)); // #[16]
    a[0xF0] = ins!(Ldh, Op::Reg(Reg::A), Op::A8(0)); // #[12]
    a[0xF1] = ins!(Pop, Op::Reg(Reg::AF)); // #[12]
    a[0xF2] = ins!(Ld, Op::Reg(Reg::A), Op::RegMem(Reg::C)); // #[8]
    a[0xF3] = ins!(Di); // #[4]
    a[0xF4] = ins!(Illegal); // #[4]
    a[0xF5] = ins!(Push, Op::Reg(Reg::AF)); // #[16]
    a[0xF6] = ins!(Or, Op::Reg(Reg::A), Op::U8(0)); // #[8]
    a[0xF7] = ins!(Rst, Op::Tgt(0x30)); // #[16]
    a[0xF8] = ins!(Ld, Op::Reg(Reg::HL), Op::SPplusI8(0)); // #[12]
    a[0xF9] = ins!(Ld, Op::Reg(Reg::SP), Op::Reg(Reg::HL)); // #[8]
    a[0xFA] = ins!(Ld, Op::Reg(Reg::A), Op::A16(0)); // #[16]
    a[0xFB] = ins!(Ei); // #[4]
    a[0xFC] = ins!(Illegal); // #[4]
    a[0xFD] = ins!(Illegal); // #[4]
    a[0xFE] = ins!(Cp, Op::Reg(Reg::A), Op::U8(0)); // #[8]
    a[0xFF] = ins!(Rst, Op::Tgt(0x38)); // #[16]

    a
};

// Generated by: gen/genins.py
pub(crate) const PREF_INSTR_TABLE: [Instr; 256] = {
    let mut a = [ins!(Illegal); 256];
    a[0x00] = ins!(Rlc, Op::Reg(Reg::B)); // #[8]
    a[0x01] = ins!(Rlc, Op::Reg(Reg::C)); // #[8]
    a[0x02] = ins!(Rlc, Op::Reg(Reg::D)); // #[8]
    a[0x03] = ins!(Rlc, Op::Reg(Reg::E)); // #[8]
    a[0x04] = ins!(Rlc, Op::Reg(Reg::H)); // #[8]
    a[0x05] = ins!(Rlc, Op::Reg(Reg::L)); // #[8]
    a[0x06] = ins!(Rlc, Op::RegMem(Reg::HL)); // #[16]
    a[0x07] = ins!(Rlc, Op::Reg(Reg::A)); // #[8]
    a[0x08] = ins!(Rrc, Op::Reg(Reg::B)); // #[8]
    a[0x09] = ins!(Rrc, Op::Reg(Reg::C)); // #[8]
    a[0x0A] = ins!(Rrc, Op::Reg(Reg::D)); // #[8]
    a[0x0B] = ins!(Rrc, Op::Reg(Reg::E)); // #[8]
    a[0x0C] = ins!(Rrc, Op::Reg(Reg::H)); // #[8]
    a[0x0D] = ins!(Rrc, Op::Reg(Reg::L)); // #[8]
    a[0x0E] = ins!(Rrc, Op::RegMem(Reg::HL)); // #[16]
    a[0x0F] = ins!(Rrc, Op::Reg(Reg::A)); // #[8]
    a[0x10] = ins!(Rl, Op::Reg(Reg::B)); // #[8]
    a[0x11] = ins!(Rl, Op::Reg(Reg::C)); // #[8]
    a[0x12] = ins!(Rl, Op::Reg(Reg::D)); // #[8]
    a[0x13] = ins!(Rl, Op::Reg(Reg::E)); // #[8]
    a[0x14] = ins!(Rl, Op::Reg(Reg::H)); // #[8]
    a[0x15] = ins!(Rl, Op::Reg(Reg::L)); // #[8]
    a[0x16] = ins!(Rl, Op::RegMem(Reg::HL)); // #[16]
    a[0x17] = ins!(Rl, Op::Reg(Reg::A)); // #[8]
    a[0x18] = ins!(Rr, Op::Reg(Reg::B)); // #[8]
    a[0x19] = ins!(Rr, Op::Reg(Reg::C)); // #[8]
    a[0x1A] = ins!(Rr, Op::Reg(Reg::D)); // #[8]
    a[0x1B] = ins!(Rr, Op::Reg(Reg::E)); // #[8]
    a[0x1C] = ins!(Rr, Op::Reg(Reg::H)); // #[8]
    a[0x1D] = ins!(Rr, Op::Reg(Reg::L)); // #[8]
    a[0x1E] = ins!(Rr, Op::RegMem(Reg::HL)); // #[16]
    a[0x1F] = ins!(Rr, Op::Reg(Reg::A)); // #[8]
    a[0x20] = ins!(Sla, Op::Reg(Reg::B)); // #[8]
    a[0x21] = ins!(Sla, Op::Reg(Reg::C)); // #[8]
    a[0x22] = ins!(Sla, Op::Reg(Reg::D)); // #[8]
    a[0x23] = ins!(Sla, Op::Reg(Reg::E)); // #[8]
    a[0x24] = ins!(Sla, Op::Reg(Reg::H)); // #[8]
    a[0x25] = ins!(Sla, Op::Reg(Reg::L)); // #[8]
    a[0x26] = ins!(Sla, Op::RegMem(Reg::HL)); // #[16]
    a[0x27] = ins!(Sla, Op::Reg(Reg::A)); // #[8]
    a[0x28] = ins!(Sra, Op::Reg(Reg::B)); // #[8]
    a[0x29] = ins!(Sra, Op::Reg(Reg::C)); // #[8]
    a[0x2A] = ins!(Sra, Op::Reg(Reg::D)); // #[8]
    a[0x2B] = ins!(Sra, Op::Reg(Reg::E)); // #[8]
    a[0x2C] = ins!(Sra, Op::Reg(Reg::H)); // #[8]
    a[0x2D] = ins!(Sra, Op::Reg(Reg::L)); // #[8]
    a[0x2E] = ins!(Sra, Op::RegMem(Reg::HL)); // #[16]
    a[0x2F] = ins!(Sra, Op::Reg(Reg::A)); // #[8]
    a[0x30] = ins!(Swap, Op::Reg(Reg::B)); // #[8]
    a[0x31] = ins!(Swap, Op::Reg(Reg::C)); // #[8]
    a[0x32] = ins!(Swap, Op::Reg(Reg::D)); // #[8]
    a[0x33] = ins!(Swap, Op::Reg(Reg::E)); // #[8]
    a[0x34] = ins!(Swap, Op::Reg(Reg::H)); // #[8]
    a[0x35] = ins!(Swap, Op::Reg(Reg::L)); // #[8]
    a[0x36] = ins!(Swap, Op::RegMem(Reg::HL)); // #[16]
    a[0x37] = ins!(Swap, Op::Reg(Reg::A)); // #[8]
    a[0x38] = ins!(Srl, Op::Reg(Reg::B)); // #[8]
    a[0x39] = ins!(Srl, Op::Reg(Reg::C)); // #[8]
    a[0x3A] = ins!(Srl, Op::Reg(Reg::D)); // #[8]
    a[0x3B] = ins!(Srl, Op::Reg(Reg::E)); // #[8]
    a[0x3C] = ins!(Srl, Op::Reg(Reg::H)); // #[8]
    a[0x3D] = ins!(Srl, Op::Reg(Reg::L)); // #[8]
    a[0x3E] = ins!(Srl, Op::RegMem(Reg::HL)); // #[16]
    a[0x3F] = ins!(Srl, Op::Reg(Reg::A)); // #[8]
    a[0x40] = ins!(Bit, Op::B3(0), Op::Reg(Reg::B)); // #[8]
    a[0x41] = ins!(Bit, Op::B3(0), Op::Reg(Reg::C)); // #[8]
    a[0x42] = ins!(Bit, Op::B3(0), Op::Reg(Reg::D)); // #[8]
    a[0x43] = ins!(Bit, Op::B3(0), Op::Reg(Reg::E)); // #[8]
    a[0x44] = ins!(Bit, Op::B3(0), Op::Reg(Reg::H)); // #[8]
    a[0x45] = ins!(Bit, Op::B3(0), Op::Reg(Reg::L)); // #[8]
    a[0x46] = ins!(Bit, Op::B3(0), Op::RegMem(Reg::HL)); // #[12]
    a[0x47] = ins!(Bit, Op::B3(0), Op::Reg(Reg::A)); // #[8]
    a[0x48] = ins!(Bit, Op::B3(1), Op::Reg(Reg::B)); // #[8]
    a[0x49] = ins!(Bit, Op::B3(1), Op::Reg(Reg::C)); // #[8]
    a[0x4A] = ins!(Bit, Op::B3(1), Op::Reg(Reg::D)); // #[8]
    a[0x4B] = ins!(Bit, Op::B3(1), Op::Reg(Reg::E)); // #[8]
    a[0x4C] = ins!(Bit, Op::B3(1), Op::Reg(Reg::H)); // #[8]
    a[0x4D] = ins!(Bit, Op::B3(1), Op::Reg(Reg::L)); // #[8]
    a[0x4E] = ins!(Bit, Op::B3(1), Op::RegMem(Reg::HL)); // #[12]
    a[0x4F] = ins!(Bit, Op::B3(1), Op::Reg(Reg::A)); // #[8]
    a[0x50] = ins!(Bit, Op::B3(2), Op::Reg(Reg::B)); // #[8]
    a[0x51] = ins!(Bit, Op::B3(2), Op::Reg(Reg::C)); // #[8]
    a[0x52] = ins!(Bit, Op::B3(2), Op::Reg(Reg::D)); // #[8]
    a[0x53] = ins!(Bit, Op::B3(2), Op::Reg(Reg::E)); // #[8]
    a[0x54] = ins!(Bit, Op::B3(2), Op::Reg(Reg::H)); // #[8]
    a[0x55] = ins!(Bit, Op::B3(2), Op::Reg(Reg::L)); // #[8]
    a[0x56] = ins!(Bit, Op::B3(2), Op::RegMem(Reg::HL)); // #[12]
    a[0x57] = ins!(Bit, Op::B3(2), Op::Reg(Reg::A)); // #[8]
    a[0x58] = ins!(Bit, Op::B3(3), Op::Reg(Reg::B)); // #[8]
    a[0x59] = ins!(Bit, Op::B3(3), Op::Reg(Reg::C)); // #[8]
    a[0x5A] = ins!(Bit, Op::B3(3), Op::Reg(Reg::D)); // #[8]
    a[0x5B] = ins!(Bit, Op::B3(3), Op::Reg(Reg::E)); // #[8]
    a[0x5C] = ins!(Bit, Op::B3(3), Op::Reg(Reg::H)); // #[8]
    a[0x5D] = ins!(Bit, Op::B3(3), Op::Reg(Reg::L)); // #[8]
    a[0x5E] = ins!(Bit, Op::B3(3), Op::RegMem(Reg::HL)); // #[12]
    a[0x5F] = ins!(Bit, Op::B3(3), Op::Reg(Reg::A)); // #[8]
    a[0x60] = ins!(Bit, Op::B3(4), Op::Reg(Reg::B)); // #[8]
    a[0x61] = ins!(Bit, Op::B3(4), Op::Reg(Reg::C)); // #[8]
    a[0x62] = ins!(Bit, Op::B3(4), Op::Reg(Reg::D)); // #[8]
    a[0x63] = ins!(Bit, Op::B3(4), Op::Reg(Reg::E)); // #[8]
    a[0x64] = ins!(Bit, Op::B3(4), Op::Reg(Reg::H)); // #[8]
    a[0x65] = ins!(Bit, Op::B3(4), Op::Reg(Reg::L)); // #[8]
    a[0x66] = ins!(Bit, Op::B3(4), Op::RegMem(Reg::HL)); // #[12]
    a[0x67] = ins!(Bit, Op::B3(4), Op::Reg(Reg::A)); // #[8]
    a[0x68] = ins!(Bit, Op::B3(5), Op::Reg(Reg::B)); // #[8]
    a[0x69] = ins!(Bit, Op::B3(5), Op::Reg(Reg::C)); // #[8]
    a[0x6A] = ins!(Bit, Op::B3(5), Op::Reg(Reg::D)); // #[8]
    a[0x6B] = ins!(Bit, Op::B3(5), Op::Reg(Reg::E)); // #[8]
    a[0x6C] = ins!(Bit, Op::B3(5), Op::Reg(Reg::H)); // #[8]
    a[0x6D] = ins!(Bit, Op::B3(5), Op::Reg(Reg::L)); // #[8]
    a[0x6E] = ins!(Bit, Op::B3(5), Op::RegMem(Reg::HL)); // #[12]
    a[0x6F] = ins!(Bit, Op::B3(5), Op::Reg(Reg::A)); // #[8]
    a[0x70] = ins!(Bit, Op::B3(6), Op::Reg(Reg::B)); // #[8]
    a[0x71] = ins!(Bit, Op::B3(6), Op::Reg(Reg::C)); // #[8]
    a[0x72] = ins!(Bit, Op::B3(6), Op::Reg(Reg::D)); // #[8]
    a[0x73] = ins!(Bit, Op::B3(6), Op::Reg(Reg::E)); // #[8]
    a[0x74] = ins!(Bit, Op::B3(6), Op::Reg(Reg::H)); // #[8]
    a[0x75] = ins!(Bit, Op::B3(6), Op::Reg(Reg::L)); // #[8]
    a[0x76] = ins!(Bit, Op::B3(6), Op::RegMem(Reg::HL)); // #[12]
    a[0x77] = ins!(Bit, Op::B3(6), Op::Reg(Reg::A)); // #[8]
    a[0x78] = ins!(Bit, Op::B3(7), Op::Reg(Reg::B)); // #[8]
    a[0x79] = ins!(Bit, Op::B3(7), Op::Reg(Reg::C)); // #[8]
    a[0x7A] = ins!(Bit, Op::B3(7), Op::Reg(Reg::D)); // #[8]
    a[0x7B] = ins!(Bit, Op::B3(7), Op::Reg(Reg::E)); // #[8]
    a[0x7C] = ins!(Bit, Op::B3(7), Op::Reg(Reg::H)); // #[8]
    a[0x7D] = ins!(Bit, Op::B3(7), Op::Reg(Reg::L)); // #[8]
    a[0x7E] = ins!(Bit, Op::B3(7), Op::RegMem(Reg::HL)); // #[12]
    a[0x7F] = ins!(Bit, Op::B3(7), Op::Reg(Reg::A)); // #[8]
    a[0x80] = ins!(Res, Op::B3(0), Op::Reg(Reg::B)); // #[8]
    a[0x81] = ins!(Res, Op::B3(0), Op::Reg(Reg::C)); // #[8]
    a[0x82] = ins!(Res, Op::B3(0), Op::Reg(Reg::D)); // #[8]
    a[0x83] = ins!(Res, Op::B3(0), Op::Reg(Reg::E)); // #[8]
    a[0x84] = ins!(Res, Op::B3(0), Op::Reg(Reg::H)); // #[8]
    a[0x85] = ins!(Res, Op::B3(0), Op::Reg(Reg::L)); // #[8]
    a[0x86] = ins!(Res, Op::B3(0), Op::RegMem(Reg::HL)); // #[16]
    a[0x87] = ins!(Res, Op::B3(0), Op::Reg(Reg::A)); // #[8]
    a[0x88] = ins!(Res, Op::B3(1), Op::Reg(Reg::B)); // #[8]
    a[0x89] = ins!(Res, Op::B3(1), Op::Reg(Reg::C)); // #[8]
    a[0x8A] = ins!(Res, Op::B3(1), Op::Reg(Reg::D)); // #[8]
    a[0x8B] = ins!(Res, Op::B3(1), Op::Reg(Reg::E)); // #[8]
    a[0x8C] = ins!(Res, Op::B3(1), Op::Reg(Reg::H)); // #[8]
    a[0x8D] = ins!(Res, Op::B3(1), Op::Reg(Reg::L)); // #[8]
    a[0x8E] = ins!(Res, Op::B3(1), Op::RegMem(Reg::HL)); // #[16]
    a[0x8F] = ins!(Res, Op::B3(1), Op::Reg(Reg::A)); // #[8]
    a[0x90] = ins!(Res, Op::B3(2), Op::Reg(Reg::B)); // #[8]
    a[0x91] = ins!(Res, Op::B3(2), Op::Reg(Reg::C)); // #[8]
    a[0x92] = ins!(Res, Op::B3(2), Op::Reg(Reg::D)); // #[8]
    a[0x93] = ins!(Res, Op::B3(2), Op::Reg(Reg::E)); // #[8]
    a[0x94] = ins!(Res, Op::B3(2), Op::Reg(Reg::H)); // #[8]
    a[0x95] = ins!(Res, Op::B3(2), Op::Reg(Reg::L)); // #[8]
    a[0x96] = ins!(Res, Op::B3(2), Op::RegMem(Reg::HL)); // #[16]
    a[0x97] = ins!(Res, Op::B3(2), Op::Reg(Reg::A)); // #[8]
    a[0x98] = ins!(Res, Op::B3(3), Op::Reg(Reg::B)); // #[8]
    a[0x99] = ins!(Res, Op::B3(3), Op::Reg(Reg::C)); // #[8]
    a[0x9A] = ins!(Res, Op::B3(3), Op::Reg(Reg::D)); // #[8]
    a[0x9B] = ins!(Res, Op::B3(3), Op::Reg(Reg::E)); // #[8]
    a[0x9C] = ins!(Res, Op::B3(3), Op::Reg(Reg::H)); // #[8]
    a[0x9D] = ins!(Res, Op::B3(3), Op::Reg(Reg::L)); // #[8]
    a[0x9E] = ins!(Res, Op::B3(3), Op::RegMem(Reg::HL)); // #[16]
    a[0x9F] = ins!(Res, Op::B3(3), Op::Reg(Reg::A)); // #[8]
    a[0xA0] = ins!(Res, Op::B3(4), Op::Reg(Reg::B)); // #[8]
    a[0xA1] = ins!(Res, Op::B3(4), Op::Reg(Reg::C)); // #[8]
    a[0xA2] = ins!(Res, Op::B3(4), Op::Reg(Reg::D)); // #[8]
    a[0xA3] = ins!(Res, Op::B3(4), Op::Reg(Reg::E)); // #[8]
    a[0xA4] = ins!(Res, Op::B3(4), Op::Reg(Reg::H)); // #[8]
    a[0xA5] = ins!(Res, Op::B3(4), Op::Reg(Reg::L)); // #[8]
    a[0xA6] = ins!(Res, Op::B3(4), Op::RegMem(Reg::HL)); // #[16]
    a[0xA7] = ins!(Res, Op::B3(4), Op::Reg(Reg::A)); // #[8]
    a[0xA8] = ins!(Res, Op::B3(5), Op::Reg(Reg::B)); // #[8]
    a[0xA9] = ins!(Res, Op::B3(5), Op::Reg(Reg::C)); // #[8]
    a[0xAA] = ins!(Res, Op::B3(5), Op::Reg(Reg::D)); // #[8]
    a[0xAB] = ins!(Res, Op::B3(5), Op::Reg(Reg::E)); // #[8]
    a[0xAC] = ins!(Res, Op::B3(5), Op::Reg(Reg::H)); // #[8]
    a[0xAD] = ins!(Res, Op::B3(5), Op::Reg(Reg::L)); // #[8]
    a[0xAE] = ins!(Res, Op::B3(5), Op::RegMem(Reg::HL)); // #[16]
    a[0xAF] = ins!(Res, Op::B3(5), Op::Reg(Reg::A)); // #[8]
    a[0xB0] = ins!(Res, Op::B3(6), Op::Reg(Reg::B)); // #[8]
    a[0xB1] = ins!(Res, Op::B3(6), Op::Reg(Reg::C)); // #[8]
    a[0xB2] = ins!(Res, Op::B3(6), Op::Reg(Reg::D)); // #[8]
    a[0xB3] = ins!(Res, Op::B3(6), Op::Reg(Reg::E)); // #[8]
    a[0xB4] = ins!(Res, Op::B3(6), Op::Reg(Reg::H)); // #[8]
    a[0xB5] = ins!(Res, Op::B3(6), Op::Reg(Reg::L)); // #[8]
    a[0xB6] = ins!(Res, Op::B3(6), Op::RegMem(Reg::HL)); // #[16]
    a[0xB7] = ins!(Res, Op::B3(6), Op::Reg(Reg::A)); // #[8]
    a[0xB8] = ins!(Res, Op::B3(7), Op::Reg(Reg::B)); // #[8]
    a[0xB9] = ins!(Res, Op::B3(7), Op::Reg(Reg::C)); // #[8]
    a[0xBA] = ins!(Res, Op::B3(7), Op::Reg(Reg::D)); // #[8]
    a[0xBB] = ins!(Res, Op::B3(7), Op::Reg(Reg::E)); // #[8]
    a[0xBC] = ins!(Res, Op::B3(7), Op::Reg(Reg::H)); // #[8]
    a[0xBD] = ins!(Res, Op::B3(7), Op::Reg(Reg::L)); // #[8]
    a[0xBE] = ins!(Res, Op::B3(7), Op::RegMem(Reg::HL)); // #[16]
    a[0xBF] = ins!(Res, Op::B3(7), Op::Reg(Reg::A)); // #[8]
    a[0xC0] = ins!(Set, Op::B3(0), Op::Reg(Reg::B)); // #[8]
    a[0xC1] = ins!(Set, Op::B3(0), Op::Reg(Reg::C)); // #[8]
    a[0xC2] = ins!(Set, Op::B3(0), Op::Reg(Reg::D)); // #[8]
    a[0xC3] = ins!(Set, Op::B3(0), Op::Reg(Reg::E)); // #[8]
    a[0xC4] = ins!(Set, Op::B3(0), Op::Reg(Reg::H)); // #[8]
    a[0xC5] = ins!(Set, Op::B3(0), Op::Reg(Reg::L)); // #[8]
    a[0xC6] = ins!(Set, Op::B3(0), Op::RegMem(Reg::HL)); // #[16]
    a[0xC7] = ins!(Set, Op::B3(0), Op::Reg(Reg::A)); // #[8]
    a[0xC8] = ins!(Set, Op::B3(1), Op::Reg(Reg::B)); // #[8]
    a[0xC9] = ins!(Set, Op::B3(1), Op::Reg(Reg::C)); // #[8]
    a[0xCA] = ins!(Set, Op::B3(1), Op::Reg(Reg::D)); // #[8]
    a[0xCB] = ins!(Set, Op::B3(1), Op::Reg(Reg::E)); // #[8]
    a[0xCC] = ins!(Set, Op::B3(1), Op::Reg(Reg::H)); // #[8]
    a[0xCD] = ins!(Set, Op::B3(1), Op::Reg(Reg::L)); // #[8]
    a[0xCE] = ins!(Set, Op::B3(1), Op::RegMem(Reg::HL)); // #[16]
    a[0xCF] = ins!(Set, Op::B3(1), Op::Reg(Reg::A)); // #[8]
    a[0xD0] = ins!(Set, Op::B3(2), Op::Reg(Reg::B)); // #[8]
    a[0xD1] = ins!(Set, Op::B3(2), Op::Reg(Reg::C)); // #[8]
    a[0xD2] = ins!(Set, Op::B3(2), Op::Reg(Reg::D)); // #[8]
    a[0xD3] = ins!(Set, Op::B3(2), Op::Reg(Reg::E)); // #[8]
    a[0xD4] = ins!(Set, Op::B3(2), Op::Reg(Reg::H)); // #[8]
    a[0xD5] = ins!(Set, Op::B3(2), Op::Reg(Reg::L)); // #[8]
    a[0xD6] = ins!(Set, Op::B3(2), Op::RegMem(Reg::HL)); // #[16]
    a[0xD7] = ins!(Set, Op::B3(2), Op::Reg(Reg::A)); // #[8]
    a[0xD8] = ins!(Set, Op::B3(3), Op::Reg(Reg::B)); // #[8]
    a[0xD9] = ins!(Set, Op::B3(3), Op::Reg(Reg::C)); // #[8]
    a[0xDA] = ins!(Set, Op::B3(3), Op::Reg(Reg::D)); // #[8]
    a[0xDB] = ins!(Set, Op::B3(3), Op::Reg(Reg::E)); // #[8]
    a[0xDC] = ins!(Set, Op::B3(3), Op::Reg(Reg::H)); // #[8]
    a[0xDD] = ins!(Set, Op::B3(3), Op::Reg(Reg::L)); // #[8]
    a[0xDE] = ins!(Set, Op::B3(3), Op::RegMem(Reg::HL)); // #[16]
    a[0xDF] = ins!(Set, Op::B3(3), Op::Reg(Reg::A)); // #[8]
    a[0xE0] = ins!(Set, Op::B3(4), Op::Reg(Reg::B)); // #[8]
    a[0xE1] = ins!(Set, Op::B3(4), Op::Reg(Reg::C)); // #[8]
    a[0xE2] = ins!(Set, Op::B3(4), Op::Reg(Reg::D)); // #[8]
    a[0xE3] = ins!(Set, Op::B3(4), Op::Reg(Reg::E)); // #[8]
    a[0xE4] = ins!(Set, Op::B3(4), Op::Reg(Reg::H)); // #[8]
    a[0xE5] = ins!(Set, Op::B3(4), Op::Reg(Reg::L)); // #[8]
    a[0xE6] = ins!(Set, Op::B3(4), Op::RegMem(Reg::HL)); // #[16]
    a[0xE7] = ins!(Set, Op::B3(4), Op::Reg(Reg::A)); // #[8]
    a[0xE8] = ins!(Set, Op::B3(5), Op::Reg(Reg::B)); // #[8]
    a[0xE9] = ins!(Set, Op::B3(5), Op::Reg(Reg::C)); // #[8]
    a[0xEA] = ins!(Set, Op::B3(5), Op::Reg(Reg::D)); // #[8]
    a[0xEB] = ins!(Set, Op::B3(5), Op::Reg(Reg::E)); // #[8]
    a[0xEC] = ins!(Set, Op::B3(5), Op::Reg(Reg::H)); // #[8]
    a[0xED] = ins!(Set, Op::B3(5), Op::Reg(Reg::L)); // #[8]
    a[0xEE] = ins!(Set, Op::B3(5), Op::RegMem(Reg::HL)); // #[16]
    a[0xEF] = ins!(Set, Op::B3(5), Op::Reg(Reg::A)); // #[8]
    a[0xF0] = ins!(Set, Op::B3(6), Op::Reg(Reg::B)); // #[8]
    a[0xF1] = ins!(Set, Op::B3(6), Op::Reg(Reg::C)); // #[8]
    a[0xF2] = ins!(Set, Op::B3(6), Op::Reg(Reg::D)); // #[8]
    a[0xF3] = ins!(Set, Op::B3(6), Op::Reg(Reg::E)); // #[8]
    a[0xF4] = ins!(Set, Op::B3(6), Op::Reg(Reg::H)); // #[8]
    a[0xF5] = ins!(Set, Op::B3(6), Op::Reg(Reg::L)); // #[8]
    a[0xF6] = ins!(Set, Op::B3(6), Op::RegMem(Reg::HL)); // #[16]
    a[0xF7] = ins!(Set, Op::B3(6), Op::Reg(Reg::A)); // #[8]
    a[0xF8] = ins!(Set, Op::B3(7), Op::Reg(Reg::B)); // #[8]
    a[0xF9] = ins!(Set, Op::B3(7), Op::Reg(Reg::C)); // #[8]
    a[0xFA] = ins!(Set, Op::B3(7), Op::Reg(Reg::D)); // #[8]
    a[0xFB] = ins!(Set, Op::B3(7), Op::Reg(Reg::E)); // #[8]
    a[0xFC] = ins!(Set, Op::B3(7), Op::Reg(Reg::H)); // #[8]
    a[0xFD] = ins!(Set, Op::B3(7), Op::Reg(Reg::L)); // #[8]
    a[0xFE] = ins!(Set, Op::B3(7), Op::RegMem(Reg::HL)); // #[16]
    a[0xFF] = ins!(Set, Op::B3(7), Op::Reg(Reg::A)); // #[8]

    a
};