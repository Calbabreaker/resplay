#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inst {
    Pha,
    Php,
    Pla,
    Plp,
    Asl,
    Lsr,
    Rol,
    Ror,
    Slo,
    Rla,
    Sre,
    Rra,
    Adc,
    Sbc,
    Inc,
    Dec,
    Inx,
    Iny,
    Dex,
    Dey,
    Isc,
    Dcp,
    Lda,
    Ldx,
    Ldy,
    Lax,
    Las,
    Sta,
    Stx,
    Sty,
    Sha,
    Shs,
    Shy,
    Shx,
    Sax,
    Tax,
    Tay,
    Tsx,
    Txa,
    Txs,
    Tya,
    Ane,
    Axs,
    Clc,
    Clv,
    Cld,
    Cli,
    Sec,
    Sed,
    Sei,
    And,
    Bit,
    Eor,
    Ora,
    Anc,
    Asr,
    Arr,
    Cmp,
    Cpx,
    Cpy,
    Jmp,
    Jsr,
    Rts,
    Brk,
    Rti,
    Bcc,
    Bcs,
    Beq,
    Bmi,
    Bne,
    Bpl,
    Bvc,
    Bvs,
    Nop,
    Hlt,
}

/// Addressing modes for instructions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddrMode {
    /// Contains no operand
    Implied,
    /// Use the accumulator
    Accumulator,
    /// Operand contains the value
    Immediate,
    /// Operand contains the address in the first page (256 bytes)
    ZeroPage,
    /// Same as ZeroPage + x register
    ZeroPageX,
    /// Same as ZeroPage + y register
    ZeroPageY,
    /// Same as ZeroPage but now the whole address range (16 bits)
    Absolute,
    /// Same as Absolute + x register
    AbsoluteX,
    /// Same as AbsoluteX but always clock when it would otherwise depend on page cross
    AbsoluteXForceDummy,
    /// Same as Absolute + y register
    AbsoluteY,
    AbsoluteYForceDummy,
    /// Operand contains the address to the address
    Indirect,
    /// Operand contains the address (with x added) to the address
    IndirectX,
    /// Operand contains the address to the address (with y added)
    IndirectY,
    IndirectYForceDummy,
    Relative,
}

#[derive(Clone, Copy, Debug)]
pub struct Opcode {
    pub instruction: Inst,
    pub addr_mode: AddrMode,
}

impl Opcode {
    pub fn new(instruction: Inst, addr_mode: AddrMode) -> Self {
        Self {
            instruction,
            addr_mode,
        }
    }

    /// Convert opcode byte into opcode name with addressing mode including unofficial ones
    pub fn from_byte(byte: u8) -> Self {
        use AddrMode::*;
        use Inst::*;
        match byte {
            // -- Stack --
            0x48 => Opcode::new(Pha, Implied),
            0x08 => Opcode::new(Php, Implied),
            0x68 => Opcode::new(Pla, Implied),
            0x28 => Opcode::new(Plp, Implied),

            // -- Shift and rotate --
            0x0a => Opcode::new(Asl, Accumulator),
            0x06 => Opcode::new(Asl, ZeroPage),
            0x16 => Opcode::new(Asl, ZeroPageX),
            0x0e => Opcode::new(Asl, Absolute),
            0x1e => Opcode::new(Asl, AbsoluteXForceDummy),

            0x4a => Opcode::new(Lsr, Accumulator),
            0x46 => Opcode::new(Lsr, ZeroPage),
            0x56 => Opcode::new(Lsr, ZeroPageX),
            0x4e => Opcode::new(Lsr, Absolute),
            0x5e => Opcode::new(Lsr, AbsoluteXForceDummy),

            0x2a => Opcode::new(Rol, Accumulator),
            0x26 => Opcode::new(Rol, ZeroPage),
            0x36 => Opcode::new(Rol, ZeroPageX),
            0x2e => Opcode::new(Rol, Absolute),
            0x3e => Opcode::new(Rol, AbsoluteXForceDummy),

            0x6a => Opcode::new(Ror, Accumulator),
            0x66 => Opcode::new(Ror, ZeroPage),
            0x76 => Opcode::new(Ror, ZeroPageX),
            0x6e => Opcode::new(Ror, Absolute),
            0x7e => Opcode::new(Ror, AbsoluteXForceDummy),

            0x07 => Opcode::new(Slo, ZeroPage),
            0x17 => Opcode::new(Slo, ZeroPageX),
            0x0f => Opcode::new(Slo, Absolute),
            0x1f => Opcode::new(Slo, AbsoluteXForceDummy),
            0x1b => Opcode::new(Slo, AbsoluteYForceDummy),
            0x03 => Opcode::new(Slo, IndirectX),
            0x13 => Opcode::new(Slo, IndirectYForceDummy),

            0x27 => Opcode::new(Rla, ZeroPage),
            0x37 => Opcode::new(Rla, ZeroPageX),
            0x2f => Opcode::new(Rla, Absolute),
            0x3f => Opcode::new(Rla, AbsoluteXForceDummy),
            0x3b => Opcode::new(Rla, AbsoluteYForceDummy),
            0x23 => Opcode::new(Rla, IndirectX),
            0x33 => Opcode::new(Rla, IndirectYForceDummy),

            0x47 => Opcode::new(Sre, ZeroPage),
            0x57 => Opcode::new(Sre, ZeroPageX),
            0x4f => Opcode::new(Sre, Absolute),
            0x5f => Opcode::new(Sre, AbsoluteXForceDummy),
            0x5b => Opcode::new(Sre, AbsoluteYForceDummy),
            0x43 => Opcode::new(Sre, IndirectX),
            0x53 => Opcode::new(Sre, IndirectYForceDummy),

            0x67 => Opcode::new(Rra, ZeroPage),
            0x77 => Opcode::new(Rra, ZeroPageX),
            0x6f => Opcode::new(Rra, Absolute),
            0x7f => Opcode::new(Rra, AbsoluteXForceDummy),
            0x7b => Opcode::new(Rra, AbsoluteYForceDummy),
            0x63 => Opcode::new(Rra, IndirectX),
            0x73 => Opcode::new(Rra, IndirectYForceDummy),

            // -- Arithmetic --
            0x69 => Opcode::new(Adc, Immediate),
            0x65 => Opcode::new(Adc, ZeroPage),
            0x75 => Opcode::new(Adc, ZeroPageX),
            0x6d => Opcode::new(Adc, Absolute),
            0x7d => Opcode::new(Adc, AbsoluteX),
            0x79 => Opcode::new(Adc, AbsoluteY),
            0x61 => Opcode::new(Adc, IndirectX),
            0x71 => Opcode::new(Adc, IndirectY),

            0xe9 | 0xeb => Opcode::new(Sbc, Immediate),
            0xe5 => Opcode::new(Sbc, ZeroPage),
            0xf5 => Opcode::new(Sbc, ZeroPageX),
            0xed => Opcode::new(Sbc, Absolute),
            0xfd => Opcode::new(Sbc, AbsoluteX),
            0xf9 => Opcode::new(Sbc, AbsoluteY),
            0xe1 => Opcode::new(Sbc, IndirectX),
            0xf1 => Opcode::new(Sbc, IndirectY),

            // -- Increment and decrement --
            0xe6 => Opcode::new(Inc, ZeroPage),
            0xf6 => Opcode::new(Inc, ZeroPageX),
            0xee => Opcode::new(Inc, Absolute),
            0xfe => Opcode::new(Inc, AbsoluteXForceDummy),

            0xc6 => Opcode::new(Dec, ZeroPage),
            0xd6 => Opcode::new(Dec, ZeroPageX),
            0xce => Opcode::new(Dec, Absolute),
            0xde => Opcode::new(Dec, AbsoluteXForceDummy),

            0xe8 => Opcode::new(Inx, Implied),
            0xc8 => Opcode::new(Iny, Implied),
            0xca => Opcode::new(Dex, Implied),
            0x88 => Opcode::new(Dey, Implied),

            0xe7 => Opcode::new(Isc, ZeroPage),
            0xf7 => Opcode::new(Isc, ZeroPageX),
            0xef => Opcode::new(Isc, Absolute),
            0xff => Opcode::new(Isc, AbsoluteX),
            0xfb => Opcode::new(Isc, AbsoluteY),
            0xe3 => Opcode::new(Isc, IndirectX),
            0xf3 => Opcode::new(Isc, IndirectY),

            0xc7 => Opcode::new(Dcp, ZeroPage),
            0xd7 => Opcode::new(Dcp, ZeroPageX),
            0xcf => Opcode::new(Dcp, Absolute),
            0xdf => Opcode::new(Dcp, AbsoluteX),
            0xdb => Opcode::new(Dcp, AbsoluteY),
            0xc3 => Opcode::new(Dcp, IndirectX),
            0xd3 => Opcode::new(Dcp, IndirectY),

            // -- Register loads --
            0xa9 => Opcode::new(Lda, Immediate),
            0xa5 => Opcode::new(Lda, ZeroPage),
            0xb5 => Opcode::new(Lda, ZeroPageX),
            0xad => Opcode::new(Lda, Absolute),
            0xbd => Opcode::new(Lda, AbsoluteX),
            0xb9 => Opcode::new(Lda, AbsoluteY),
            0xa1 => Opcode::new(Lda, IndirectX),
            0xb1 => Opcode::new(Lda, IndirectY),

            0xa2 => Opcode::new(Ldx, Immediate),
            0xa6 => Opcode::new(Ldx, ZeroPage),
            0xb6 => Opcode::new(Ldx, ZeroPageY),
            0xae => Opcode::new(Ldx, Absolute),
            0xbe => Opcode::new(Ldx, AbsoluteY),

            0xa0 => Opcode::new(Ldy, Immediate),
            0xa4 => Opcode::new(Ldy, ZeroPage),
            0xb4 => Opcode::new(Ldy, ZeroPageX),
            0xac => Opcode::new(Ldy, Absolute),
            0xbc => Opcode::new(Ldy, AbsoluteX),

            0xab => Opcode::new(Lax, Immediate),
            0xa7 => Opcode::new(Lax, ZeroPage),
            0xb7 => Opcode::new(Lax, ZeroPageY),
            0xaf => Opcode::new(Lax, Absolute),
            0xbf => Opcode::new(Lax, AbsoluteY),
            0xa3 => Opcode::new(Lax, IndirectX),
            0xb3 => Opcode::new(Lax, IndirectY),

            0xbb => Opcode::new(Las, AbsoluteY),

            // -- Register stores --
            0x85 => Opcode::new(Sta, ZeroPage),
            0x95 => Opcode::new(Sta, ZeroPageX),
            0x8d => Opcode::new(Sta, Absolute),
            0x9d => Opcode::new(Sta, AbsoluteXForceDummy),
            0x99 => Opcode::new(Sta, AbsoluteYForceDummy),
            0x81 => Opcode::new(Sta, IndirectX),
            0x91 => Opcode::new(Sta, IndirectYForceDummy),

            0x8e => Opcode::new(Stx, Absolute),
            0x86 => Opcode::new(Stx, ZeroPage),
            0x96 => Opcode::new(Stx, ZeroPageY),

            0x8c => Opcode::new(Sty, Absolute),
            0x84 => Opcode::new(Sty, ZeroPage),
            0x94 => Opcode::new(Sty, ZeroPageX),

            0x87 => Opcode::new(Sax, ZeroPage),
            0x97 => Opcode::new(Sax, ZeroPageY),
            0x8f => Opcode::new(Sax, Absolute),
            0x83 => Opcode::new(Sax, IndirectX),

            0x93 => Opcode::new(Sha, IndirectYForceDummy),
            0x9f => Opcode::new(Sha, AbsoluteYForceDummy),
            0x9b => Opcode::new(Shs, AbsoluteYForceDummy),
            0x9c => Opcode::new(Shy, AbsoluteX),
            0x9e => Opcode::new(Shx, AbsoluteYForceDummy),

            // -- Register transfers --
            0xaa => Opcode::new(Tax, Implied),
            0xa8 => Opcode::new(Tay, Implied),
            0xba => Opcode::new(Tsx, Implied),
            0x8a => Opcode::new(Txa, Implied),
            0x9a => Opcode::new(Txs, Implied),
            0x98 => Opcode::new(Tya, Implied),
            0x8b => Opcode::new(Ane, Immediate),
            0xcb => Opcode::new(Axs, Immediate),

            // -- Flag clear and set --
            0x18 => Opcode::new(Clc, Implied),
            0xd8 => Opcode::new(Cld, Implied),
            0x58 => Opcode::new(Cli, Implied),
            0xb8 => Opcode::new(Clv, Implied),
            0x38 => Opcode::new(Sec, Implied),
            0xf8 => Opcode::new(Sed, Implied),
            0x78 => Opcode::new(Sei, Implied),

            // -- Logic --
            0x29 => Opcode::new(And, Immediate),
            0x25 => Opcode::new(And, ZeroPage),
            0x35 => Opcode::new(And, ZeroPageX),
            0x2d => Opcode::new(And, Absolute),
            0x3d => Opcode::new(And, AbsoluteX),
            0x39 => Opcode::new(And, AbsoluteY),
            0x21 => Opcode::new(And, IndirectX),
            0x31 => Opcode::new(And, IndirectY),

            0x2c => Opcode::new(Bit, Absolute),
            0x24 => Opcode::new(Bit, ZeroPage),

            0x49 => Opcode::new(Eor, Immediate),
            0x45 => Opcode::new(Eor, ZeroPage),
            0x55 => Opcode::new(Eor, ZeroPageX),
            0x4d => Opcode::new(Eor, Absolute),
            0x5d => Opcode::new(Eor, AbsoluteX),
            0x59 => Opcode::new(Eor, AbsoluteY),
            0x41 => Opcode::new(Eor, IndirectX),
            0x51 => Opcode::new(Eor, IndirectY),

            0x09 => Opcode::new(Ora, Immediate),
            0x05 => Opcode::new(Ora, ZeroPage),
            0x15 => Opcode::new(Ora, ZeroPageX),
            0x0d => Opcode::new(Ora, Absolute),
            0x1d => Opcode::new(Ora, AbsoluteX),
            0x19 => Opcode::new(Ora, AbsoluteY),
            0x01 => Opcode::new(Ora, IndirectX),
            0x11 => Opcode::new(Ora, IndirectY),

            0x0b => Opcode::new(Anc, Immediate),
            0x2b => Opcode::new(Anc, Immediate),
            0x4b => Opcode::new(Asr, Immediate),
            0x6b => Opcode::new(Arr, Immediate),

            0xc9 => Opcode::new(Cmp, Immediate),
            0xc5 => Opcode::new(Cmp, ZeroPage),
            0xd5 => Opcode::new(Cmp, ZeroPageX),
            0xcd => Opcode::new(Cmp, Absolute),
            0xdd => Opcode::new(Cmp, AbsoluteX),
            0xd9 => Opcode::new(Cmp, AbsoluteY),
            0xc1 => Opcode::new(Cmp, IndirectX),
            0xd1 => Opcode::new(Cmp, IndirectY),

            0xe0 => Opcode::new(Cpx, Immediate),
            0xe4 => Opcode::new(Cpx, ZeroPage),
            0xec => Opcode::new(Cpx, Absolute),

            0xc0 => Opcode::new(Cpy, Immediate),
            0xc4 => Opcode::new(Cpy, ZeroPage),
            0xcc => Opcode::new(Cpy, Absolute),

            // -- Control flow --
            0x4c => Opcode::new(Jmp, Absolute),
            0x6c => Opcode::new(Jmp, Indirect),
            0x20 => Opcode::new(Jsr, Absolute),
            0x60 => Opcode::new(Rts, Implied),
            0x00 => Opcode::new(Brk, Implied),
            0x40 => Opcode::new(Rti, Implied),

            0x90 => Opcode::new(Bcc, Relative),
            0xb0 => Opcode::new(Bcs, Relative),
            0xf0 => Opcode::new(Beq, Relative),
            0x30 => Opcode::new(Bmi, Relative),
            0xd0 => Opcode::new(Bne, Relative),
            0x10 => Opcode::new(Bpl, Relative),
            0x50 => Opcode::new(Bvc, Relative),
            0x70 => Opcode::new(Bvs, Relative),

            // Nop madness but like why are there so many nop opcodes
            0x1a | 0x3a | 0x5a | 0x7a | 0xda | 0xea | 0xfa => Opcode::new(Nop, Implied),
            0x80 | 0x82 | 0x89 | 0xc2 | 0xe2 => Opcode::new(Nop, Immediate),
            0x04 | 0x44 | 0x64 => Opcode::new(Nop, ZeroPage),
            0x14 | 0x34 | 0x54 | 0x74 | 0xd4 | 0xf4 => Opcode::new(Nop, ZeroPageX),
            0x0c => Opcode::new(Nop, Absolute),
            0x1c | 0x3c | 0x5c | 0x7c | 0xdc | 0xfc => Opcode::new(Nop, AbsoluteX),

            0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xb2 | 0xd2 | 0xf2 => {
                Opcode::new(Hlt, Implied)
            }
        }
    }
}
