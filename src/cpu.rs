mod decoder;
mod isa;
mod table;

use std::num::Wrapping;

use isa::{Cond, Instr, Opcode, Operand, Reg};

use crate::{
    info, log,
    macros::bit_fields,
    mem::Mmu,
    regs::{IntData, Key1},
};

/// LDH adds 0xFF00 to its memory address operands before using
/// them for accessing memory, it is for HRAM.  
/// Only LDH has such operands, they are: `[C]` and `[imm8]`.
const LDH_OFFSET: u16 = 0xFF00;

/// Gameboy CPU emulator with support for double speed mode.  
/// Instruction semantics are implemented as specified in:
/// https://rgbds.gbdev.io/docs/v0.8.0/gbz80.7
#[derive(Default)]
pub struct Cpu {
    // CPU owns the memory and can be borrowed when needed by other components.
    pub(crate) mmu: Mmu,
    /// When halted only the CPU is halted from executing instructions
    /// until an interrupt occurs, however, clocks still run.
    pub(crate) is_halted: bool,
    /// When stopped everything is stopped until a joystick interrupt.
    pub(crate) is_stopped: bool,
    pub(crate) trace_execution: bool,

    // Machine registers
    flags: Flags,
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    pub(crate) pc: Wrapping<u16>,
    pub(crate) sp: Wrapping<u16>,

    /// Interrupt master enable
    ime: bool,
    /// Set ime after this many M-cycles(includes current cycle).
    ime_set_delay: u32,

    /// Number of M-cycles elapsed for the current step.
    /// One read/write operation uses one M-cycle. Care not to double count.
    mcycles: u32,
}

bit_fields! {
    struct Flags<u8> {
        _0: 4,
        c:1,
        h:1,
        n:1,
        z:1,
    }
}

impl Cpu {
    pub(crate) fn new(mmu: Mmu) -> Self {
        Self {
            mmu,
            ..Default::default()
        }
    }

    /// Performs the next atomic step, that is, execute an instruction or
    /// handle a pending interrupt and return the number of cycles consumed.
    ///
    /// Returns a value of at least `info::SPEED_SWITCH_MCYCLES` on
    /// speed switch to dual-speed mode.
    pub(crate) fn step(&mut self) -> u32 {
        self.mcycles = 0;

        // Either handle an interrupt or run an instruction.
        if self.handle_interrupt() {
            if self.is_halted {
                return 1;
            }
            if self.is_stopped {
                return 0;
            }
        }

        self.exec_next_instr();

        if self.ime_set_delay > 0 {
            self.ime_set_delay = self.ime_set_delay.saturating_sub(self.mcycles);
            if !self.ime && self.ime_set_delay == 0 {
                self.ime = true;
            }
        }

        self.mcycles
    }

    /// Handle an interrupt if any and return true if handled.
    fn handle_interrupt(&mut self) -> bool {
        let ints = self.mmu.get_queued_ints();

        // Wakeup from low-power states when a servicable interrupts comes.
        // We do not emulate any of the halt/stop bugs.
        if ints.read() != 0 && (self.is_halted || self.is_stopped) {
            // log::info("cpu: wake-up");
            self.is_halted = false;
            self.is_stopped = false;
        }

        // No interrupts available or disabled.
        if !self.ime || ints.read() == 0 {
            return false;
        }

        let mut iflag = IntData::new(self.mmu.read(info::IO_IF));

        // According to interrupt priority.
        let new_pc = if ints.vblank == 1 {
            iflag.vblank = 0;
            info::INT_VBLANK_VEC
        } else if ints.stat == 1 {
            iflag.stat = 0;
            info::INT_STAT_VEC
        } else if ints.timer == 1 {
            iflag.timer = 0;
            info::INT_TIMER_VEC
        } else if ints.serial == 1 {
            iflag.serial = 0;
            info::INT_SERIAL_VEC
        } else if ints.joypad == 1 {
            iflag.joypad = 0;
            info::INT_JOYPAD_VEC
        } else {
            unreachable!()
        };

        // Reset handeled interrupt in IF and disable further interrupts.
        self.mmu.set_reg(info::IO_IF, iflag.read());
        self.ime = false;

        // Execute ISR. It takes a total of 5 M-cycles.
        self.mcycles += 2; // Run two wait states.
        self.do_push(self.pc.0);
        self.pc.0 = new_pc;
        self.mcycles += 1;

        true
    }

    fn exec_next_instr(&mut self) {
        let old_pc = self.pc.0;
        let ins = self.fetch();

        let (oa, ob) = (ins.op1, ins.op2);
        let a = self.get_op_val(oa);
        let b = self.get_op_val(ob);

        // Determine number of M-cycles to fetch/set operands. An operand is
        // either fetched or set [but not both] only once, so checking each
        // operand correctly calculates the number of bytes read/wrote.
        if is_mem_operand(oa) {
            self.mcycles += 1;
        }
        if is_mem_operand(ob) {
            self.mcycles += 1;
        }

        // M-cycles consumed for other memory accesses or operations by
        // instructions are calculated when they are run.
        use Opcode::*;
        match ins.op {
            Ld | Ldh => {
                // `LD [a16], SP` loads two bytes.
                if let (Operand::A16(a), Operand::Reg(Reg::SP)) = (oa, ob) {
                    let [h, l] = self.sp.0.to_be_bytes();
                    self.mmu.write_cpu(a, l);
                    self.mmu.write_cpu(a.wrapping_add(1), h);
                    self.mcycles += 1; // For the extra 1-byte load.
                } else {
                    self.set_op_val(oa, b);
                }

                // Only LD has [HL+] and [HL-] operands, inc/dec as required.
                let d = get_hl_delta(oa) + get_hl_delta(ob);
                let hl = self.get_reg(Reg::HL).wrapping_add_signed(d);
                self.set_reg(Reg::HL, hl);

                // In `LD HL, SP + e8` flags needs to be set.
                if let Operand::SPplusI8(e) = ob {
                    let v = (e as i16) as u16;
                    self.flags.write(0);
                    self.flags.h = is_carry(self.sp.0, v, 4);
                    self.flags.c = is_carry(self.sp.0, v, 8);
                }
            }

            Push => self.do_push(a),
            Pop => {
                let r = self.do_pop();
                self.set_op_val(oa, r);
            }

            Inc | Dec => {
                let r = self.do_inc_dec(matches!(ins.op, Inc), oa, a);
                self.set_op_val(oa, r);
            }

            // For "ADD HL, r16" and "ADD SP, e8".
            Add if is_reg16(oa) => {
                let r = self.do_add_r16(ob, a, b);
                self.set_op_val(oa, r);
            }

            Add | Adc | Sub | Sbc | Cp | And | Xor | Or => {
                let r = self.do_8bit_arith(ins.op, a as u8, b as u8);
                self.set_op_val(oa, r as u16);
            }

            Rlca | Rlc | Rrca | Rrc | Rla | Rl | Rra | Rr | Sla | Sra | Srl => {
                // These have Reg::A as their first operand implicitly.
                let (oa, a) = if matches!(ins.op, Rlca | Rrca | Rla | Rra) {
                    (Operand::Reg(Reg::A), self.a as u16)
                } else {
                    (oa, a)
                };
                let r = self.do_shift_or_rotate(ins.op, a as u8);
                self.set_op_val(oa, r as u16);
            }

            // Swap nibbles.
            Swap => {
                let r = ((a >> 4) & 0xF) | ((a & 0xF) << 4);
                self.set_cz00(0, r as u8);
                self.set_op_val(oa, r);
            }

            // Test bit if 0.
            Bit => {
                self.flags.z = is_zero((b >> a) & 1);
                self.flags.n = 0;
                self.flags.h = 1;
            }
            // Set bit to 0.
            Res => self.set_op_val(ob, b & !(1 << a)),
            // Set bit to 1.
            Set => self.set_op_val(ob, b | (1 << a)),

            // Branch
            Jr | Jp | Call | Ret | Reti | Rst => self.do_branch(ins.op, oa, a, b),

            // Interrupt and system control
            Di => self.ime = false,
            // Setting IME=1 by EI is delayed by one M-cycle plus this cycle.
            Ei => self.ime_set_delay = 2,
            // Halt CPU until an interrupt is recieved.
            Halt => self.is_halted = true,

            Stop => {
                let key = Key1::new(self.mmu.read(info::IO_KEY1));
                if self.mmu.cart.is_cgb && key.armed == 1 && key.speed == 0 {
                    log::info("cpu: switched to dual-speed/CGB mode");
                    self.pc.0 = 0;
                    self.do_speed_switch();
                } else {
                    self.is_stopped = true;
                }
                self.mmu.set_reg(info::IO_DIV, 0);
            }

            // Misc
            Cpl => {
                self.a = !self.a;
                self.flags.n = 1;
                self.flags.h = 1;
            }
            Ccf => {
                self.flags.c = !self.flags.c & 1;
                self.flags.n = 0;
                self.flags.h = 0;
            }
            Scf => {
                self.flags.c = 1;
                self.flags.n = 0;
                self.flags.h = 0;
            }
            Nop => (),
            Daa => self.do_daa(),

            Illegal | Prefix => log::warn("cpu: illegal instruction detected, skipping"),
        }

        if self.trace_execution {
            let newa = self.get_op_val(oa);
            let sx = format!("[{oa}={a}|{newa} {ob}={b}]");
            eprintln!(
                "{sx:30} [Z{} N{} C{}] [${:04X} {}I] {}",
                self.flags.z, self.flags.n, self.flags.c, old_pc, self.ime as u8, ins,
            );
        }
    }

    /// Fetch the instruction pointed by PC, point PC to the next instruction
    /// and increment `mcycles` according to the length of instruction.
    fn fetch(&mut self) -> Instr {
        let (ins, pc) = decoder::decode(&mut self.mmu, self.pc.0);
        if pc < self.pc.0 {
            log::warn("cpu: PC overflow, wrapped back to zero")
        }

        // Number of bytes read is the number of M-cycles needed.
        self.mcycles += pc.wrapping_sub(self.pc.0) as u32;
        self.pc.0 = pc;
        ins
    }

    /// Get numerical value for the operand.  
    /// For Cond 0 is returned as it has no numeric meaning.  
    fn get_op_val(&self, op: Operand) -> u16 {
        match op {
            Operand::Absent => 0,
            Operand::Reg(r) => self.get_reg(r),
            Operand::RegMem(r) => self.mmu.read_cpu(self.get_mem_addr(r)) as u16,

            // Cond is seperately inspected whenever needed, so just return 0.
            Operand::Cond(_) => 0,
            Operand::B3(b) => b as u16,
            Operand::Tgt(t) => t as u16,

            // 2's complement numbers can be added as unsigned numbers
            // giving the same result, ignoring any overflows.
            Operand::I8(i) => (i as i16) as u16,
            Operand::U8(u) => u as u16,
            Operand::U16(u) => u,
            // Flags should be set when `SP + e8` is used as a operand.
            Operand::SPplusI8(i) => (self.sp.0 as i32 + i as i32) as u16,

            // [imm8] is a memory operand for LDH, see `LDH_OFFSET`.
            Operand::A8(u) => self.mmu.read_cpu(u as u16 + LDH_OFFSET) as u16,
            Operand::A16(u) => self.mmu.read_cpu(u) as u16,
        }
    }

    /// Set value for the given operand.Panics if the operand is not a
    /// destination, that is,  
    /// either a register(direct or indirect) or a memory address.
    fn set_op_val(&mut self, op: Operand, val: u16) {
        match op {
            Operand::Reg(r) => self.set_reg(r, val),
            Operand::RegMem(r) => self.mmu.write_cpu(self.get_mem_addr(r), val as u8),

            // [imm8] is a memory operand for LDH, see `LDH_OFFSET`.
            Operand::A8(u) => self.mmu.write_cpu(u as u16 + LDH_OFFSET, val as u8),
            Operand::A16(u) => self.mmu.write_cpu(u, val as u8),

            _ => panic!("Operand is not a destination, it has no location"),
        }
    }

    /// Get address from register value for indirect addressing.
    /// Panics if register does not support indirect mode.
    fn get_mem_addr(&self, r: Reg) -> u16 {
        match r {
            // [C] is a memory operand for LDH, see `LDH_OFFSET`.
            Reg::C => self.get_reg(Reg::C) + LDH_OFFSET,
            Reg::BC | Reg::DE => self.get_reg(r),
            Reg::HL | Reg::HLinc | Reg::HLdec => self.get_reg(Reg::HL),

            _ => panic!("given register does not support indirect-addressing"),
        }
    }

    /// Get value stored in register.
    fn get_reg(&self, r: Reg) -> u16 {
        let bytes = match r {
            Reg::A => [0, self.a],
            Reg::B => [0, self.b],
            Reg::C => [0, self.c],
            Reg::D => [0, self.d],
            Reg::E => [0, self.e],
            Reg::H => [0, self.h],
            Reg::L => [0, self.l],
            Reg::AF => [self.a, self.flags.read()],
            Reg::BC => [self.b, self.c],
            Reg::DE => [self.d, self.e],
            Reg::HL => [self.h, self.l],
            Reg::SP => self.sp.0.to_be_bytes(),
            _ => unreachable!(),
        };

        u16::from_be_bytes(bytes)
    }

    /// Set register value.
    fn set_reg(&mut self, r: Reg, v: u16) {
        let [h, l] = v.to_be_bytes();

        match r {
            Reg::A => self.a = l,
            Reg::B => self.b = l,
            Reg::C => self.c = l,
            Reg::D => self.d = l,
            Reg::E => self.e = l,
            Reg::H => self.h = l,
            Reg::L => self.l = l,

            Reg::AF => {
                self.a = h;
                self.flags.write(l & 0xF0)
            }
            Reg::BC => (self.b, self.c) = (h, l),
            Reg::DE => (self.d, self.e) = (h, l),
            Reg::HL => (self.h, self.l) = (h, l),
            Reg::SP => self.sp = Wrapping(v),
            _ => unreachable!(),
        }
    }

    // Utility methods, these help evaluate a specific class if instructions.
    //-----------------------------------------------------------------------

    /// Push 2-bytes and increment `mcycles`.
    fn do_push(&mut self, v: u16) {
        let [h, l] = v.to_be_bytes();

        self.sp -= 1;
        self.mmu.write_cpu(self.sp.0, h);
        self.sp -= 1;
        self.mmu.write_cpu(self.sp.0, l);

        self.mcycles += 2;
    }

    /// Pop 2-bytes and increment `mcycles`.
    fn do_pop(&mut self) -> u16 {
        let l = self.mmu.read_cpu(self.sp.0);
        self.sp += 1;
        let h = self.mmu.read_cpu(self.sp.0);
        self.sp += 1;

        self.mcycles += 2;
        u16::from_be_bytes([h, l])
    }

    /// Executes INC and DEC for their both: 16-bit and 8-bit variants.
    fn do_inc_dec(&mut self, is_inc: bool, oa: Operand, a: u16) -> u16 {
        if is_reg16(oa) {
            // No flags are affected for "INC|DEC r16".
            if is_inc {
                a.wrapping_add(1)
            } else {
                a.wrapping_sub(1)
            }
        } else {
            let r = if is_inc {
                self.flags.n = 0;
                self.flags.h = is_carry(a, 1, 4);
                (a as u8).wrapping_add(1) as u16
            } else {
                self.flags.n = 1;
                self.flags.h = is_borrow(a, 1, 4);
                (a as u8).wrapping_sub(1) as u16
            };

            self.flags.z = is_zero(r);
            r
        }
    }

    /// Executes instruction "ADD SP, e8" or "ADD HL, r16" depending upon
    /// the type second argument, which needs to be passed.
    fn do_add_r16(&mut self, ob: Operand, a: u16, b: u16) -> u16 {
        let r = a.wrapping_add(b);

        // Overflow if r < [a or b], for on bit-x take only lower x+1 bits.
        let is_ovf = |bits: u32| is_carry(a, b, bits);

        if matches!(ob, Operand::I8(_)) {
            // For "ADD SP, e8"
            self.flags.z = 0;
            self.flags.h = is_ovf(4);
            self.flags.c = is_ovf(8);
        } else {
            // For "ADD HL, r16"
            self.flags.h = is_ovf(12);
            self.flags.c = is_ovf(16);
        }
        self.flags.n = 0;

        r
    }

    /// Does arithmetic and returns result and sets flags as required.  
    /// Only flags are set and result is unchanged(returns `a`) if op is `CP`.
    fn do_8bit_arith(&mut self, op: Opcode, a: u8, b: u8) -> u8 {
        let cb = self.flags.c;

        use Opcode::*;
        let r = match op {
            Add => a.wrapping_add(b),
            Adc => a.wrapping_add(b).wrapping_add(cb),

            Sub | Cp => a.wrapping_sub(b),
            Sbc => a.wrapping_sub(b).wrapping_sub(cb),

            And => a & b,
            Xor => a ^ b,
            Or => a | b,

            _ => unreachable!(),
        };

        self.flags.write(0);
        self.flags.z = is_zero(r as u16);
        self.flags.n = matches!(op, Sub | Sbc | Cp) as u8;

        let (ax, bx, cx) = (a as u16, b as u16, cb as u16); // As over/under-flows take u16.
        match op {
            Add => {
                self.flags.h = is_carry(ax, bx, 4);
                self.flags.c = is_carry(ax, bx, 8);
            }
            Adc => {
                self.flags.h = is_carry3(ax, bx, cx, 4);
                self.flags.c = is_carry3(ax, bx, cx, 8);
            }
            Sub | Cp => {
                self.flags.h = is_borrow(ax, bx, 4);
                self.flags.c = is_borrow(ax, bx, 8);
            }
            Sbc => {
                self.flags.h = is_borrow3(ax, bx, cx, 4);
                self.flags.c = is_borrow3(ax, bx, cx, 8);
            }
            And => self.flags.h = 1,
            _ => (),
        }

        if matches!(op, Cp) {
            a
        } else {
            r
        }
    }

    /// Does all kinds of shifts and rotations and sets flags as specified.
    fn do_shift_or_rotate(&mut self, op: Opcode, a: u8) -> u8 {
        // Bit Shift and Rotations, all done on 8-bit operands only.
        // So for left shifts just shift-right by 7 to get MSB of the
        // operand for determing the carry flag.

        use Opcode::*;
        let r = match op {
            // Rotate left.
            Rlca | Rlc => a.rotate_left(1),
            // Rotate right.
            Rrca | Rrc => a.rotate_right(1),
            // Rotate left via carry flag.
            Rla | Rl => a << 1 | self.flags.c,
            // Rotate right via carry flag.
            Rra | Rr => a >> 1 | self.flags.c << 7,
            // Arithmetic shift left.
            Sla => a << 1,
            // Arithmetic shift right.
            Sra => ((a as i8) >> 1) as u8,
            // Logical shift right.
            Srl => a >> 1,

            _ => unreachable!(),
        };

        match op {
            // For left shifts/rotates, MSB will go into carry.
            Rlca | Rlc | Rla | Rl | Sla => self.set_cz00(a >> 7, r),
            // For right shifts/rotates, LSB will go into carry.
            Rrca | Rrc | Rra | Rr | Sra | Srl => self.set_cz00(a, r),
            _ => unreachable!(),
        }

        // These set flag.Z to 0.
        if matches!(op, Rla | Rlca | Rrca | Rra) {
            self.flags.z = 0;
        }

        r
    }

    /// Execute branch instructions: JR, JP, RET, RETI, CALL and RST,
    /// set PC return if the branch was taken.
    fn do_branch(&mut self, op: Opcode, oa: Operand, a: u16, b: u16) {
        let taken = match oa {
            Operand::Cond(cc) => match cc {
                Cond::NC => self.flags.c == 0,
                Cond::NZ => self.flags.z == 0,
                Cond::C => self.flags.c == 1,
                Cond::Z => self.flags.z == 1,
            },
            _ => true,
        };

        if !taken {
            return;
        }

        let pc = if let Operand::Cond(_) = oa { b } else { a };

        use Opcode::*;
        let pc = match op {
            Jr => self.pc.0.wrapping_add(pc),
            Jp => pc,
            Call => {
                self.do_push(self.pc.0);
                pc
            }
            Ret => self.do_pop(),
            Reti => {
                self.ime = true;
                self.do_pop()
            }
            Rst => {
                self.do_push(self.pc.0);
                pc
            }

            _ => unreachable!(),
        };

        // If a branch is taken then it consumes an extra M-cycle.
        self.mcycles += 1;
        self.pc.0 = pc;
    }

    fn do_daa(&mut self) {
        let mut a = self.a;

        if self.flags.n == 0 {
            if self.flags.c == 1 || a > 0x99 {
                a = a.wrapping_add(0x60);
                self.flags.c = 1;
            }
            if self.flags.h == 1 || (a & 0x0f) > 0x09 {
                a = a.wrapping_add(0x6);
            }
        } else {
            if self.flags.c == 1 {
                a = a.wrapping_sub(0x60);
            }
            if self.flags.h == 1 {
                a = a.wrapping_sub(0x6);
            }
        }

        self.a = a;
        self.flags.z = is_zero(a as u16);
        self.flags.h = 0;
    }

    fn do_speed_switch(&mut self) {
        self.mmu.is_cgb = true;

        let mut key = Key1::new(0);
        key.speed = 1;
        key.armed = 0;
        self.mmu.set_reg(info::IO_KEY1, key.read());
        // Simulate time for which cpu stops on speed-switch and
        // also lets other tell that speed-switch has happened.
        self.mcycles += info::SPEED_SWITCH_MCYCLES;
    }

    /// Set carry(to carry.LSB==1) and zero(to zero==0) flags.
    /// Set rest of the flags to 0.
    fn set_cz00(&mut self, carry: u8, zero: u8) {
        self.flags.write(0);
        self.flags.c = carry & 1;
        self.flags.z = is_zero(zero as u16);
    }
}

/// Returns true if operand is a memory operand(`[...]`).
fn is_mem_operand(op: Operand) -> bool {
    matches!(op, Operand::RegMem(_) | Operand::A8(_) | Operand::A16(_))
}

/// Returns true is `op` is a reg16 operand.
fn is_reg16(op: Operand) -> bool {
    match op {
        Operand::Reg(r) => matches!(r, Reg::BC | Reg::DE | Reg::HL | Reg::SP),
        _ => false,
    }
}

// Functions for determining flag values, since flags is a bit_fields! struct,
// it has all its fields are u8 as opposed to being booleans.
#[inline]
fn is_zero(a: u16) -> u8 {
    (a == 0) as u8
}

fn is_carry3(a: u16, b: u16, c: u16, bits: u32) -> u8 {
    let m = mask(bits);
    let (a, b, c) = (a & m, b & m, c & m);
    let r = a.wrapping_add(b) & m;

    if r < a {
        1
    } else {
        (r.wrapping_add(c) & m < c) as u8
    }
}

fn is_borrow3(a: u16, b: u16, c: u16, bits: u32) -> u8 {
    let m = mask(bits);
    let (a, b, c) = (a & m, b & m, c & m);

    if b > a {
        1
    } else {
        (c > (a - b)) as u8
    }
}

fn is_carry(a: u16, b: u16, bits: u32) -> u8 {
    // Overflow for r=a+b: if r < [a or b]
    let m = mask(bits);
    let (a, b) = (a & m, b & m);
    (a.wrapping_add(b) & m < a) as u8
}

fn is_borrow(a: u16, b: u16, bits: u32) -> u8 {
    // Underflow for r=a-b: if b > a
    let m = mask(bits);
    let (a, b) = (a & m, b & m);
    (b > a) as u8
}

/// A mask with `bit_cnt` number of lower-order bits set to 1.
#[inline(always)]
const fn mask(bit_cnt: u32) -> u16 {
    if bit_cnt == 16 {
        !0
    } else {
        !(!0 << bit_cnt)
    }
}

/// Returns +1 for [HL+], -1 for [HL-] and otherwise 0.
fn get_hl_delta(op: Operand) -> i16 {
    match op {
        Operand::RegMem(r) => match r {
            Reg::HLinc => 1,
            Reg::HLdec => -1,
            _ => 0,
        },
        _ => 0,
    }
}
