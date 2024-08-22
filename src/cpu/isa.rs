use core::fmt;
use std::fmt::Debug;

/// Decoded instruction with operands information.
///
/// The number of M-cycles(=4 T-cycles) needed to execute an instruction
/// is the number of bytes need to be read/wrote from/to the main memory for
/// its execution by the CPU.
/// Every instructions needs at least one M-cycle since it needs to be first
/// fetched from the memory. Instructions containing immediates or
/// register-indirect operand needs extra cycles.
///
/// For branch instructions cycles needed to execute depend on if the branch
/// was taken or not as the number of memory accesses can vary according to it.
/// If a branch is taken then all plus one extra M-cycle is consumed, presumably
/// for adjusting the PC(program counter) in the hardware.
#[derive(Clone, Copy)]
pub(crate) struct Instr {
    pub(crate) op: Opcode,
    pub(crate) op1: Operand,
    pub(crate) op2: Operand,
}

impl Default for Instr {
    fn default() -> Self {
        Instr {
            op: Opcode::Nop,
            op1: Operand::Absent,
            op2: Operand::Absent,
        }
    }
}

impl fmt::Display for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let opcode = format!("{:?}", self.op).to_ascii_uppercase();
        match (
            !matches!(self.op1, Operand::Absent),
            !matches!(self.op2, Operand::Absent),
        ) {
            (true, true) => write!(f, "{} {}, {}", opcode, self.op1, self.op2),
            (true, false) => write!(f, "{} {}", opcode, self.op1),
            (false, false) => write!(f, "{}", opcode),
            (false, true) => panic!("invalid: first operand absent but second present"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Operand {
    /// No operand
    Absent,
    /// Register value
    Reg(Reg),
    /// Register value as memory address
    RegMem(Reg),
    /// Branch condition
    Cond(Cond),
    /// Bit Index
    B3(u8),
    /// RST target vector value
    Tgt(u8),
    /// Unsigned 8-bit imm
    U8(u8),
    /// Signed 8-bit imm
    I8(i8),
    /// Unsigned 16-bit imme
    U16(u16),
    /// 8-bit imm as memory address
    A8(u8),
    /// 16-bit imm as memory address
    A16(u16),
    /// For the operand `SP + i8`
    SPplusI8(i8),
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operand::Absent => write!(f, "<?>"),
            Operand::Reg(r) => write!(f, "{r:?}"),
            Operand::RegMem(r) => write!(f, "[{r:?}]"),
            Operand::Cond(c) => write!(f, "{c:?}"),
            Operand::B3(b) => write!(f, "{b}"),
            Operand::Tgt(t) => write!(f, "${t:04X}"),
            Operand::U8(u) => write!(f, "${u:02X}"),
            Operand::I8(i) => write!(f, "#{i:+}"),
            Operand::U16(u) => write!(f, "${u:04X}"),
            Operand::A8(a) => write!(f, "[$FF00 + ${a:02X}]"),
            Operand::A16(a) => write!(f, "[${a:04X}]"),
            Operand::SPplusI8(i) => write!(f, "SP + ${i:02X}"),
        }
    }
}

// Operation to perform for an instrution.
// These values do not correspond in any way the actual opcodes.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Opcode {
    // Memory
    Ld,
    Ldh, // Adds 0xFF00 to its address operand
    Push,
    Pop,

    // Arithmetic
    Inc,
    Dec,
    Add,
    Adc,
    Sub,
    Sbc,
    And,
    Xor,
    Or,
    Cp,

    // Bit Shift and Rotations
    Rla,
    Rlca,
    Rra,
    Rrca,
    Rlc,
    Rrc,
    Rl,
    Rr,
    Sla,
    Sra,
    Srl,
    Swap,
    Bit,
    Res,
    Set,

    // Branch
    Jr,
    Jp,
    Call,
    Ret,
    Reti,
    Rst,

    // Interrupt and system control
    Di,
    Ei,
    Halt,
    Stop,

    // Misc
    Cpl,
    Ccf,
    Scf,
    Nop,
    Daa,
    Prefix,

    Illegal,
}

/// All register names present in r8, r16, r16mem and r16stk are
/// represented by a single type for simplicity.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Reg {
    A,
    // F, // never needed
    B,
    C,
    D,
    E,
    H,
    L,
    AF,
    BC,
    DE,
    HL,
    HLinc,
    HLdec,
    SP,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Cond {
    NZ,
    Z,
    NC,
    C,
}
