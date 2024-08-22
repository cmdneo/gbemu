use crate::cpu::isa::{Instr, Opcode, Operand};
use crate::mem::Mmu;

use super::table;

/// Decodes one instruction along with any immediates that follow it
/// and returns the decoder instruction and new PC.
///
/// Any overflows when calculating the new PC are ignored, it
/// should be checked by the caller to see if PC has wrapped around.
pub(crate) fn decode(mmu: &mut Mmu, pc: u16) -> (Instr, u16) {
    let (ins, pc) = decode_one(&table::INSTR_TABLE, mmu, pc);

    if matches!(ins.op, Opcode::Prefix) {
        let (ins, pc) = decode_one(&table::PREF_INSTR_TABLE, mmu, pc);
        (ins, pc)
    } else {
        (ins, pc)
    }
}

/// Decodes one-byte instruction using the given table.
fn decode_one(table: &[Instr], mmu: &mut Mmu, pc: u16) -> (Instr, u16) {
    let mut ins = table[mmu.read_cpu(pc) as usize];
    let pc = pc.wrapping_add(1);

    // Only one of the operands can be immediate at a time.
    let (op1, pc) = fill_in_if_imm(ins.op1, mmu, pc);
    let (op2, pc) = fill_in_if_imm(ins.op2, mmu, pc);
    ins.op1 = op1;
    ins.op2 = op2;

    (ins, pc)
}

/// Extracts immediate and returns its value as `Operand` and its size.  
/// If not an immediate. then returns the `operand` unchanged and 0 size.
fn fill_in_if_imm(operand: Operand, mmu: &mut Mmu, pc: u16) -> (Operand, u16) {
    use Operand::*;
    let as_u16 = || u16::from_le_bytes([mmu.read_cpu(pc), mmu.read_cpu(pc + 1)]);

    let (op, size) = match operand {
        A16(_) => (A16(as_u16()), 2),
        U16(_) => (U16(as_u16()), 2),

        A8(_) => (A8(mmu.read_cpu(pc)), 1),
        U8(_) => (U8(mmu.read_cpu(pc)), 1),
        I8(_) => (I8(mmu.read_cpu(pc) as i8), 1),
        SPplusI8(_) => (SPplusI8(mmu.read_cpu(pc) as i8), 1),

        _ => (operand, 0),
    };

    (op, pc.wrapping_add(size))
}
