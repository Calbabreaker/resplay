use crate::{
    Cpu,
    cpu::{AddrMode, Opcode},
};

pub struct Disassembler<'a> {
    cpu: &'a Cpu,
    pub current_address: u16,
}

impl<'a> Disassembler<'a> {
    pub fn new(cpu: &'a Cpu) -> Self {
        Self {
            current_address: cpu.pc,
            cpu,
        }
    }

    pub fn disassemble_lines(&mut self, amount: usize) -> String {
        (0..amount)
            .map(|_| {
                let mut out = String::new();
                self.disassemble_next(&mut out).unwrap();
                out
            })
            .collect::<Vec<String>>()
            .join("\n")
    }

    pub fn disassemble_next(&mut self, mut f: impl std::fmt::Write) -> std::fmt::Result {
        write!(f, "${:04x}: ", self.current_address)?;

        let opcode_byte = self.cpu.bus.peek_read(self.current_address);
        self.current_address = self.current_address.wrapping_add(1);

        let opcode = Opcode::from_byte(opcode_byte);
        let name = format!("{:?}", opcode.instruction);
        write!(f, "{} ", name.to_lowercase())?;

        match opcode.addr_mode {
            AddrMode::Accumulator => write!(f, "A")?,
            AddrMode::Implied => (),
            AddrMode::Immediate => write!(f, "#{}", self.next_byte())?,
            AddrMode::ZeroPage => write!(f, "{}", self.next_byte())?,
            AddrMode::ZeroPageX => write!(f, "{},X", self.next_byte())?,
            AddrMode::ZeroPageY => write!(f, "{},Y", self.next_byte())?,
            AddrMode::Absolute => write!(f, "{}", self.next_word())?,
            AddrMode::AbsoluteX | AddrMode::AbsoluteXForceDummy => {
                write!(f, "{},X", self.next_word())?
            }
            AddrMode::AbsoluteY | AddrMode::AbsoluteYForceDummy => {
                write!(f, "{},Y", self.next_word())?
            }
            AddrMode::Indirect => write!(f, "[{}]", self.next_word())?,
            AddrMode::IndirectX => write!(f, "[{},X]", self.next_byte())?,
            AddrMode::IndirectY | AddrMode::IndirectYForceDummy => {
                write!(f, "[{}],Y", self.next_byte())?
            }
            AddrMode::Relative => {
                let offset = self.next_byte().0 as i8;
                let address = HexDisplay((self.current_address as i16 + offset as i16) as u16);
                let sign = if offset >= 0 { '+' } else { '-' };
                write!(f, "*{sign}{} ({address})", offset.saturating_abs())?
            }
        };

        Ok(())
    }

    fn next_byte(&mut self) -> HexDisplay<u8> {
        let address = self.current_address;
        self.current_address = self.current_address.wrapping_add(1);
        HexDisplay(self.cpu.bus.peek_read(address))
    }

    fn next_word(&mut self) -> HexDisplay<u16> {
        HexDisplay(self.next_byte().0 as u16 | ((self.next_byte().0 as u16) << 8))
    }
}

struct HexDisplay<T>(pub T);

impl std::fmt::Display for HexDisplay<u8> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${0:02x}", self.0)
    }
}

impl std::fmt::Display for HexDisplay<u16> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${0:04x}", self.0)
    }
}
