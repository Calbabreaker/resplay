mod bus;
mod disassembler;
mod opcode;

pub use bus::{CpuBus, IrqStatus};
pub use disassembler::Disassembler;
pub use opcode::{AddrMode, Inst, Opcode};

/// Number of clock cycles per second
pub const CLOCK_SPEED_HZ: f32 = 1789773.;
pub const CYCLES_PER_FRAME: f32 = 29780.5;

bitflags::bitflags! {
    /// Flags for the cpu register
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Flags: u8 {
        const CARRY = 1;
        const ZERO = 1 << 1;
        const INTERRUPT = 1 << 2;
        const DECIMAL = 1 << 3;
        // Set for pushing with BRK and PHP instructions will always be the same internally
        #[bitflags(flag_name = "")]
        const BREAK = 1 << 4;
        /// Always set
        #[bitflags(flag_name = "")]
        const UNUSED = 1 << 5;
        /// Set if arithmetic overflowed 8-bit signed number
        const OVERFLOW = 1 << 6;
        const NEGATIVE = 1 << 7;
    }
}

impl Default for Flags {
    fn default() -> Self {
        Self::UNUSED
    }
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum CpuError {
    #[error("CPU encountered a halt instruction")]
    Halted,
}

const NMI_LOAD_VECTOR: u16 = 0xfffa;
const RESET_LOAD_VECTOR: u16 = 0xfffc;
const IRQ_LOAD_VECTOR: u16 = 0xfffe;

/// Emulated 6502 CPU
#[derive(Default)]
pub struct Cpu {
    /// Program counter
    pub pc: u16,
    // Stack pointer
    pub sp: u8,
    // Accumulator
    pub a: u8,
    // X register
    pub x: u8,
    // Y register
    pub y: u8,
    pub flags: Flags,
    pub bus: CpuBus,
}

impl Cpu {
    /// Execute the next instruction at the pc
    /// Returns the number of cpu cycles that the instruction took to execute
    pub fn execute_next(&mut self) -> Result<u32, CpuError> {
        self.bus.cpu_cycles_since_inst = 0;

        if self.bus.require_nmi() {
            self.interrupt(NMI_LOAD_VECTOR);
        } else if self.bus.irq_status() && !self.flags.contains(Flags::INTERRUPT) {
            self.interrupt(IRQ_LOAD_VECTOR);
        }

        let byte = self.read_at_pc();
        let opcode = Opcode::from_byte(byte);
        self.execute(opcode)?;

        Ok(self.bus.cpu_cycles_since_inst)
    }

    pub fn reset(&mut self) {
        self.bus.cpu_cycles_total = 0;
        self.bus.cpu_cycles_since_inst = 0;
        self.bus.apu.reset();
        self.bus.ppu.registers.reset();

        self.flags = Flags::default() | Flags::INTERRUPT;
        self.pc = self.bus.read_u16(RESET_LOAD_VECTOR);
        self.sp = self.sp.wrapping_sub(3);
        for _ in 0..5 {
            self.bus.clock();
        }
    }

    fn read_at_pc(&mut self) -> u8 {
        let pc = self.pc;
        self.pc = self.pc.wrapping_add(1);
        self.bus.read(pc)
    }

    fn read_u16_at_pc(&mut self) -> u16 {
        let pc = self.pc;
        self.pc = self.pc.wrapping_add(2);
        self.bus.read_u16(pc)
    }

    fn address_add_offset(&mut self, address: u16, offset: u8, mode: AddrMode) -> u16 {
        let address_offset = address.wrapping_add(offset as u16);
        let force_dummy = matches!(
            mode,
            AddrMode::AbsoluteYForceDummy
                | AddrMode::AbsoluteXForceDummy
                | AddrMode::IndirectYForceDummy
        );
        // The CPU will first add the offset with the low byte of the address and try to read from there
        // Then it will check if the page has been crossed and read with the correct high byte if needs be
        if force_dummy || address_offset & 0xff00 != address & 0xff00 {
            // The dummy read if page crossed
            self.bus
                .read((address & 0xff00) | (address_offset & 0x00ff));
        }
        address_offset
    }

    /// Returns the target address of the value based on the addressing mode and the operand
    fn read_operand_address(&mut self, mode: AddrMode) -> u16 {
        match mode {
            AddrMode::Accumulator | AddrMode::Implied => {
                unreachable!("read operand address when no operand available")
            }
            AddrMode::Immediate => {
                let address = self.pc;
                self.pc = self.pc.wrapping_add(1);
                address
            }
            AddrMode::ZeroPage => self.read_at_pc() as u16,
            AddrMode::ZeroPageX => {
                self.bus.clock();
                self.read_at_pc().wrapping_add(self.x) as u16
            }
            AddrMode::ZeroPageY => {
                self.bus.clock();
                self.read_at_pc().wrapping_add(self.y) as u16
            }
            AddrMode::Absolute => self.read_u16_at_pc(),
            AddrMode::AbsoluteX | AddrMode::AbsoluteXForceDummy => {
                let address = self.read_u16_at_pc();
                self.address_add_offset(address, self.x, mode)
            }
            AddrMode::AbsoluteY | AddrMode::AbsoluteYForceDummy => {
                let address = self.read_u16_at_pc();
                self.address_add_offset(address, self.y, mode)
            }
            AddrMode::Indirect => {
                let indirect_address = self.read_u16_at_pc();
                self.bus.read_u16_wrapped(indirect_address)
            }
            AddrMode::IndirectX => {
                let indirect_address = self.read_at_pc().wrapping_add(self.x);
                self.bus.clock();
                self.bus.read_u16_wrapped(indirect_address as u16)
            }
            AddrMode::IndirectY | AddrMode::IndirectYForceDummy => {
                let indirect_address = self.read_at_pc();
                let address = self.bus.read_u16_wrapped(indirect_address as u16);
                self.address_add_offset(address, self.y, mode)
            }
            AddrMode::Relative => {
                let offset = self.read_at_pc() as i8 as u16;
                self.pc.wrapping_add(offset)
            }
        }
    }

    /// Returns (operand address, operand value)
    fn read_operand(&mut self, mode: AddrMode) -> (Option<u16>, u8) {
        if mode == AddrMode::Accumulator {
            (None, self.a)
        } else {
            let address = self.read_operand_address(mode);
            (Some(address), self.bus.read(address))
        }
    }

    fn execute(&mut self, opcode: Opcode) -> Result<(), CpuError> {
        // Temp value storing the operand address
        match opcode.instruction {
            // -- Stack --
            Inst::Pha => self.pha(),
            Inst::Php => self.php(),
            Inst::Pla => self.pla(),
            Inst::Plp => self.plp(),

            // -- Shift and rotate --
            Inst::Asl => drop(self.asl(opcode.addr_mode)),
            Inst::Lsr => drop(self.lsr(opcode.addr_mode)),
            Inst::Rol => drop(self.rol(opcode.addr_mode)),
            Inst::Ror => drop(self.ror(opcode.addr_mode)),

            Inst::Slo => self.slo(opcode.addr_mode),
            Inst::Rla => self.rla(opcode.addr_mode),
            Inst::Sre => self.sre(opcode.addr_mode),
            Inst::Rra => self.rra(opcode.addr_mode),

            // -- Arithmetic --
            Inst::Adc => self.adc(opcode.addr_mode),
            Inst::Sbc => self.sbc(opcode.addr_mode),

            // -- Increment and decrement --
            Inst::Inc => drop(self.inc(opcode.addr_mode)),
            Inst::Dec => drop(self.dec(opcode.addr_mode)),
            Inst::Inx => self.inx(),
            Inst::Iny => self.iny(),
            Inst::Dex => self.dex(),
            Inst::Dey => self.dey(),
            Inst::Isc => self.isc(opcode.addr_mode),
            Inst::Dcp => self.dcp(opcode.addr_mode),

            // -- Register loads --
            Inst::Lda => self.lda(opcode.addr_mode),
            Inst::Ldx => self.ldx(opcode.addr_mode),
            Inst::Ldy => self.ldy(opcode.addr_mode),
            Inst::Lax => self.lax(opcode.addr_mode),
            Inst::Las => self.las(opcode.addr_mode),

            // -- Register stores --
            Inst::Sta => self.sta(opcode.addr_mode),
            Inst::Stx => self.stx(opcode.addr_mode),
            Inst::Sty => self.sty(opcode.addr_mode),
            Inst::Sax => self.sax(opcode.addr_mode),
            Inst::Sha => self.sha(opcode.addr_mode),
            Inst::Shs => self.shs(opcode.addr_mode),
            Inst::Shy => self.shy(opcode.addr_mode),
            Inst::Shx => self.shx(opcode.addr_mode),

            // -- Register transfers --
            Inst::Tax => self.tax(),
            Inst::Tay => self.tay(),
            Inst::Tsx => self.tsx(),
            Inst::Txa => self.txa(),
            Inst::Tya => self.tya(),
            Inst::Txs => self.txs(),
            Inst::Ane => self.ane(opcode.addr_mode),
            Inst::Axs => self.axs(opcode.addr_mode),

            // -- Flag clear and set --
            Inst::Clc => self.clc(),
            Inst::Cld => self.cld(),
            Inst::Cli => self.cli(),
            Inst::Clv => self.clv(),
            Inst::Sec => self.sec(),
            Inst::Sed => self.sed(),
            Inst::Sei => self.sei(),

            // -- Logic --
            Inst::And => self.and(opcode.addr_mode),
            Inst::Eor => self.eor(opcode.addr_mode),
            Inst::Ora => self.ora(opcode.addr_mode),
            Inst::Bit => self.bit(opcode.addr_mode),
            Inst::Anc => self.anc(opcode.addr_mode),
            Inst::Asr => self.asr(opcode.addr_mode),
            Inst::Arr => self.arr(opcode.addr_mode),

            Inst::Cmp => self.cmp(opcode.addr_mode),
            Inst::Cpx => self.cpx(opcode.addr_mode),
            Inst::Cpy => self.cpy(opcode.addr_mode),

            // -- Control flow --
            Inst::Jmp => self.jmp(opcode.addr_mode),
            Inst::Jsr => self.jsr(opcode.addr_mode),
            Inst::Rts => self.rts(),
            Inst::Brk => self.brk(),
            Inst::Rti => self.rti(),

            Inst::Bcc => self.bcc(opcode.addr_mode),
            Inst::Bcs => self.bcs(opcode.addr_mode),
            Inst::Beq => self.beq(opcode.addr_mode),
            Inst::Bmi => self.bmi(opcode.addr_mode),
            Inst::Bne => self.bne(opcode.addr_mode),
            Inst::Bpl => self.bpl(opcode.addr_mode),
            Inst::Bvc => self.bvc(opcode.addr_mode),
            Inst::Bvs => self.bvs(opcode.addr_mode),

            // Does nothing
            Inst::Nop => self.nop(opcode.addr_mode),
            Inst::Hlt => return Err(CpuError::Halted),
        }
        Ok(())
    }

    //
    // -- Utility functions --
    //

    fn set_zero_neg_flags(&mut self, value: u8) -> u8 {
        self.flags.set(Flags::ZERO, value == 0);
        self.flags.set(Flags::NEGATIVE, value & 0b1000_0000 != 0);
        value
    }

    // This just returns the value but also clocks the bus and sets zero and neg flags for certain instructions
    fn transfer(&mut self, value: u8) -> u8 {
        self.bus.clock();
        self.set_zero_neg_flags(value);
        value
    }

    fn stack_push(&mut self, value: u8) {
        self.bus.write(0x100 + self.sp as u16, value);
        self.sp = self.sp.wrapping_sub(1);
        self.bus.clock();
    }

    fn stack_push_u16(&mut self, value: u16) {
        self.sp = self.sp.wrapping_sub(2);
        self.bus
            .write_u16(0x100 + self.sp.wrapping_add(1) as u16, value);
    }

    fn stack_pop(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        self.bus.clock();
        self.bus.clock();
        self.bus.read(0x100 + self.sp as u16)
    }

    fn stack_pop_u16(&mut self) -> u16 {
        self.sp = self.sp.wrapping_add(2);
        self.bus.read_u16(0x100 + self.sp.wrapping_sub(1) as u16)
    }

    fn left_shift(&mut self, value: u8, carry: bool) -> u8 {
        self.flags.set(Flags::CARRY, value & 0b1000_0000 != 0);
        (value << 1) | carry as u8
    }

    fn right_shift(&mut self, value: u8, carry: bool) -> u8 {
        self.flags.set(Flags::CARRY, value & 0b0000_0001 != 0);
        (value >> 1) | ((carry as u8) << 7)
    }

    fn write_read_result(&mut self, address: Option<u16>, value: u8, result: u8) -> u8 {
        // Read-modify-update instructions like shift whatever immediately writes the value in the
        // same cycle as performing the operation
        if let Some(operand_address) = address {
            self.bus.write(operand_address, value);
            self.bus.write(operand_address, result);
        } else {
            // Still a clock for the op when accumulator addressing mode
            self.bus.clock();
            self.a = result;
        }
        self.set_zero_neg_flags(result)
    }

    /// Set overflow if the resulting addition overflowed a (negative) 8-bit number with 2's compliment
    fn set_overflow_flag(&mut self, a: u8, adder: u8, result: u8) {
        let adder_same_sign = (a ^ adder) & 0b1000_0000 == 0;
        let result_changed_sign = (a ^ result) & 0b1000_0000 != 0;
        self.flags
            .set(Flags::OVERFLOW, adder_same_sign && result_changed_sign);
    }

    fn add_carry(&mut self, adder: u8) {
        let carry = self.flags.contains(Flags::CARRY);
        let result = adder as u16 + self.a as u16 + carry as u16;
        self.flags.set(Flags::CARRY, result > 0xff);

        self.set_overflow_flag(self.a, adder, result as u8);
        self.set_zero_neg_flags(result as u8);
        self.a = result as u8;
    }

    fn set_compare_flags(&mut self, register: u8, value: u8) {
        self.flags.set(Flags::CARRY, register >= value);
        self.set_zero_neg_flags(register.wrapping_sub(value));
    }

    fn interrupt(&mut self, mut load_vector: u16) {
        self.stack_push_u16(self.pc);
        self.stack_push(self.flags.bits());
        // If there is a nmi when we're about the load the load vector then the nmi hijacks it
        // No later than cycle 4
        if self.bus.require_nmi() {
            load_vector = NMI_LOAD_VECTOR;
        }
        self.bus.clock();
        self.flags.set(Flags::INTERRUPT, true);
        self.pc = self.bus.read_u16(load_vector);
    }

    fn branch(&mut self, mode: AddrMode, condition: bool) {
        let address = self.read_operand_address(mode);
        if condition {
            self.bus.clock();
            if address & 0xff00 != self.pc & 0xff00 {
                self.bus.clock();
            }
            self.pc = address;
        }
    }

    //
    // -- Actual cpu instructions --
    //

    fn pha(&mut self) {
        self.stack_push(self.a);
    }

    fn php(&mut self) {
        self.stack_push((self.flags | Flags::BREAK).bits());
    }

    fn pla(&mut self) {
        self.a = self.stack_pop();
        self.set_zero_neg_flags(self.a);
    }

    fn plp(&mut self) {
        self.flags = Flags::from_bits(self.stack_pop()).unwrap();
        self.flags.insert(Flags::UNUSED);
        self.flags.remove(Flags::BREAK);
    }

    fn asl(&mut self, mode: AddrMode) -> u8 {
        let (address, value) = self.read_operand(mode);
        let result = self.left_shift(value, false);
        self.write_read_result(address, value, result)
    }

    fn lsr(&mut self, mode: AddrMode) -> u8 {
        let (address, value) = self.read_operand(mode);
        let result = self.right_shift(value, false);
        self.write_read_result(address, value, result)
    }

    fn rol(&mut self, mode: AddrMode) -> u8 {
        let (address, value) = self.read_operand(mode);
        let result = self.left_shift(value, self.flags.contains(Flags::CARRY));
        self.write_read_result(address, value, result)
    }

    fn ror(&mut self, mode: AddrMode) -> u8 {
        let (address, value) = self.read_operand(mode);
        let result = self.right_shift(value, self.flags.contains(Flags::CARRY));
        self.write_read_result(address, value, result)
    }

    fn slo(&mut self, mode: AddrMode) {
        // asl + ora
        self.a |= self.asl(mode);
        self.set_zero_neg_flags(self.a);
    }

    fn rla(&mut self, mode: AddrMode) {
        // rol + and
        self.a &= self.rol(mode);
        self.set_zero_neg_flags(self.a);
    }

    fn sre(&mut self, mode: AddrMode) {
        // lsr + eor
        self.a ^= self.lsr(mode);
        self.set_zero_neg_flags(self.a);
    }

    fn rra(&mut self, mode: AddrMode) {
        // ror + adc
        let adder = self.ror(mode);
        self.add_carry(adder);
    }

    fn adc(&mut self, mode: AddrMode) {
        let (_, value) = self.read_operand(mode);
        self.add_carry(value);
    }

    fn sbc(&mut self, mode: AddrMode) {
        // Inverting results in inverting the sign so the adc can be resued for sbc
        let (_, value) = self.read_operand(mode);
        self.add_carry(!value);
    }

    fn inc(&mut self, mode: AddrMode) -> u8 {
        let (address, value) = self.read_operand(mode);
        let result = value.wrapping_add(1);
        self.write_read_result(address, value, result)
    }

    fn dec(&mut self, mode: AddrMode) -> u8 {
        let (address, value) = self.read_operand(mode);
        let result = value.wrapping_sub(1);
        self.write_read_result(address, value, result)
    }

    fn inx(&mut self) {
        self.x = self.transfer(self.x.wrapping_add(1));
    }

    fn iny(&mut self) {
        self.y = self.transfer(self.y.wrapping_add(1));
    }

    fn dex(&mut self) {
        self.x = self.transfer(self.x.wrapping_sub(1));
    }

    fn dey(&mut self) {
        self.y = self.transfer(self.y.wrapping_sub(1));
    }

    fn isc(&mut self, mode: AddrMode) {
        // inc + sbc
        let adder = !self.inc(mode);
        self.add_carry(adder);
    }

    fn dcp(&mut self, mode: AddrMode) {
        // dec + cmp
        let value = self.dec(mode);
        self.set_compare_flags(self.a, value);
    }

    fn lda(&mut self, mode: AddrMode) {
        self.a = self.read_operand(mode).1;
        self.set_zero_neg_flags(self.a);
    }

    fn ldx(&mut self, mode: AddrMode) {
        self.x = self.read_operand(mode).1;
        self.set_zero_neg_flags(self.x);
    }

    fn ldy(&mut self, mode: AddrMode) {
        self.y = self.read_operand(mode).1;
        self.set_zero_neg_flags(self.y);
    }

    fn lax(&mut self, mode: AddrMode) {
        self.lda(mode);
        self.x = self.a;
    }

    fn las(&mut self, mode: AddrMode) {
        self.a = self.read_operand(mode).1 & self.sp;
        self.x = self.set_zero_neg_flags(self.a);
        self.sp = self.a;
    }

    fn sta(&mut self, mode: AddrMode) {
        let address = self.read_operand_address(mode);
        self.bus.write(address, self.a);
    }

    fn stx(&mut self, mode: AddrMode) {
        let address = self.read_operand_address(mode);
        self.bus.write(address, self.x);
    }

    fn sty(&mut self, mode: AddrMode) {
        let address = self.read_operand_address(mode);
        self.bus.write(address, self.y);
    }

    fn sax(&mut self, mode: AddrMode) {
        let address = self.read_operand_address(mode);
        self.bus.write(address, self.a & self.x);
    }

    fn sha(&mut self, mode: AddrMode) {}

    fn shs(&mut self, mode: AddrMode) {}

    fn shx(&mut self, mode: AddrMode) {}

    fn shy(&mut self, mode: AddrMode) {}

    fn tax(&mut self) {
        self.x = self.transfer(self.a);
    }

    fn tay(&mut self) {
        self.y = self.transfer(self.a);
    }

    fn tsx(&mut self) {
        self.x = self.transfer(self.sp);
    }

    fn txa(&mut self) {
        self.a = self.transfer(self.x);
    }

    fn tya(&mut self) {
        self.a = self.transfer(self.y);
    }

    fn txs(&mut self) {
        self.sp = self.x;
        self.bus.clock();
    }

    fn ane(&mut self, mode: AddrMode) {
        self.a = self.read_operand(mode).1 & self.x;
        self.set_zero_neg_flags(self.a);
    }

    fn axs(&mut self, mode: AddrMode) {
        self.x &= self.a;
        let value = self.read_operand(mode).1;
        self.set_compare_flags(self.x, value);
        self.x = self.x.wrapping_sub(value);
    }

    fn clc(&mut self) {
        self.flags.remove(Flags::CARRY);
        self.bus.clock();
    }

    fn cld(&mut self) {
        self.flags.remove(Flags::DECIMAL);
        self.bus.clock();
    }

    fn cli(&mut self) {
        self.flags.remove(Flags::INTERRUPT);
        self.bus.clock();
    }

    fn clv(&mut self) {
        self.flags.remove(Flags::OVERFLOW);
        self.bus.clock();
    }

    fn sec(&mut self) {
        self.flags.insert(Flags::CARRY);
        self.bus.clock();
    }

    fn sed(&mut self) {
        self.flags.insert(Flags::DECIMAL);
        self.bus.clock();
    }

    fn sei(&mut self) {
        self.flags.insert(Flags::INTERRUPT);
        self.bus.clock();
    }

    fn and(&mut self, mode: AddrMode) {
        self.a &= self.read_operand(mode).1;
        self.set_zero_neg_flags(self.a);
    }

    fn eor(&mut self, mode: AddrMode) {
        self.a ^= self.read_operand(mode).1;
        self.set_zero_neg_flags(self.a);
    }

    fn ora(&mut self, mode: AddrMode) {
        self.a |= self.read_operand(mode).1;
        self.set_zero_neg_flags(self.a);
    }

    fn anc(&mut self, mode: AddrMode) {
        self.a &= self.read_operand(mode).1;
        self.flags.set(Flags::CARRY, self.a & 0b1000_0000 != 0);
        self.set_zero_neg_flags(self.a);
    }

    fn asr(&mut self, mode: AddrMode) {
        self.a &= self.read_operand(mode).1;
        self.a = self.right_shift(self.a, false);
        self.set_zero_neg_flags(self.a);
    }

    fn arr(&mut self, mode: AddrMode) {
        self.a &= self.read_operand(mode).1;
        self.a = self.right_shift(self.a, self.flags.contains(Flags::CARRY));
        let bit_5 = (self.a >> 5) & 1;
        let bit_6 = (self.a >> 6) & 1;
        self.flags.set(Flags::CARRY, bit_6 == 1);
        self.flags.set(Flags::OVERFLOW, bit_5 ^ bit_6 == 1);
        self.set_zero_neg_flags(self.a);
    }

    fn bit(&mut self, mode: AddrMode) {
        let value = self.read_operand(mode).1;
        let result = self.a & value;
        self.flags.set(Flags::ZERO, result == 0);
        self.flags.set(Flags::OVERFLOW, value & (0b0100_0000) != 0);
        self.flags.set(Flags::NEGATIVE, value & (0b1000_0000) != 0);
    }

    fn cmp(&mut self, mode: AddrMode) {
        let value = self.read_operand(mode).1;
        self.set_compare_flags(self.a, value);
    }

    fn cpx(&mut self, mode: AddrMode) {
        let value = self.read_operand(mode).1;
        self.set_compare_flags(self.x, value);
    }

    fn cpy(&mut self, mode: AddrMode) {
        let value = self.read_operand(mode).1;
        self.set_compare_flags(self.y, value);
    }

    fn jmp(&mut self, mode: AddrMode) {
        self.pc = self.read_operand_address(mode);
    }

    fn jsr(&mut self, mode: AddrMode) {
        let address = self.read_operand_address(mode);
        self.stack_push_u16(self.pc - 1);
        self.bus.clock();
        self.pc = address;
    }

    fn rts(&mut self) {
        for _ in 0..3 {
            self.bus.clock();
        }
        self.pc = self.stack_pop_u16() + 1;
    }

    fn rti(&mut self) {
        self.plp();
        self.pc = self.stack_pop_u16();
    }

    fn brk(&mut self) {
        self.flags.insert(Flags::BREAK);
        self.pc += 1; // BRK has padding byte
        self.interrupt(IRQ_LOAD_VECTOR);
        self.flags.remove(Flags::BREAK);
    }

    fn bcc(&mut self, mode: AddrMode) {
        self.branch(mode, !self.flags.contains(Flags::CARRY));
    }

    fn bcs(&mut self, mode: AddrMode) {
        self.branch(mode, self.flags.contains(Flags::CARRY));
    }

    fn beq(&mut self, mode: AddrMode) {
        self.branch(mode, self.flags.contains(Flags::ZERO));
    }

    fn bmi(&mut self, mode: AddrMode) {
        self.branch(mode, self.flags.contains(Flags::NEGATIVE));
    }

    fn bne(&mut self, mode: AddrMode) {
        self.branch(mode, !self.flags.contains(Flags::ZERO));
    }

    fn bpl(&mut self, mode: AddrMode) {
        self.branch(mode, !self.flags.contains(Flags::NEGATIVE));
    }

    fn bvc(&mut self, mode: AddrMode) {
        self.branch(mode, !self.flags.contains(Flags::OVERFLOW));
    }

    fn bvs(&mut self, mode: AddrMode) {
        self.branch(mode, self.flags.contains(Flags::OVERFLOW));
    }

    // Does nothing
    fn nop(&mut self, mode: AddrMode) {
        if mode == AddrMode::Implied {
            self.bus.clock();
        } else {
            // Dummy read for nop instructions with address
            let address = self.read_operand_address(mode);
            self.bus.read(address);
        }
    }
}
