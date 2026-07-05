use crate::{
    Apu, Controller, Ppu,
    cartridge::{Cartridge, FixedArray},
    ppu::PpuClockReport,
};

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct CpuBus {
    // 2kb of cpu ram
    pub ram: FixedArray<u8, 0x800>,
    pub apu: Apu,
    /// Number of cycles added when executing the previous instruction
    #[serde(skip)]
    pub(crate) cpu_cycles_since_inst: u32,
    pub cpu_cycles_total: u64,
    pub ppu: Ppu,
    open_bus: u8,
    pub controllers: [Controller; 2],
    require_nmi: bool,
}

impl CpuBus {
    /// Immutable read function for peeking into memory
    /// Reads into some address cause side effects
    pub fn peek_read(&self, address: u16) -> u8 {
        if let Some(value) = self.cartridge().and_then(|c| c.cpu_read(address)) {
            return value;
        }

        if let 0x0000..=0x1fff = address {
            // 2kb of ram is mirrored 3 times
            self.ram[address as usize % self.ram.len()]
        } else {
            self.open_bus
        }
    }

    pub fn read(&mut self, address: u16) -> u8 {
        // https://www.nesdev.org/wiki/CPU_memory_map
        self.clock();
        let output = match address {
            0x2000..=0x3fff => self.ppu.registers.read(address),
            // Top 3 high controller bits always have open bus
            0x4016 => self.controllers[0].read() | (0b1110_0000 & self.open_bus),
            // APU does not contribute to open bus
            0x4015 => return self.apu.read_status() | (0b0010_0000 & self.open_bus),
            0x4017 => self.controllers[1].read() | (0b1110_0000 & self.open_bus),
            _ => self.peek_read(address),
        };
        self.open_bus = output;
        output
    }

    pub fn write(&mut self, address: u16, value: u8) {
        if let Some(cartridge) = self.cartridge_mut() {
            cartridge.cpu_write(address, value);
        }

        let ram_len = self.ram.len();
        match address {
            // 2kb of ram is mirrored 3 times
            0x0000..=0x1fff => self.ram[address as usize % ram_len] = value,
            0x2000..=0x3fff => self.ppu.registers.write(address, value),
            0x4014 => self.oam_dma((value as u16) << 8),
            0x4016 => {
                self.controllers[0].write(value);
                self.controllers[1].write(value);
            }
            0x4000..=0x4017 => self.apu.write(address, value),
            _ => (),
        }
        self.clock();
    }

    pub fn read_u16(&mut self, address: u16) -> u16 {
        let lsb = self.read(address) as u16;
        let msb = self.read(address + 1) as u16;
        (msb << 8) | lsb
    }

    /// Same as read u16 but the high byte is wrapped to the beggining of the page
    pub fn read_u16_wrapped(&mut self, address: u16) -> u16 {
        let lsb = self.read(address) as u16;
        // Wrap the page by always getting the address high byte from the current page
        let address_for_msb = (address & 0xff00) | ((address + 1) & 0x00ff);
        let msb = self.read(address_for_msb) as u16;
        (msb << 8) | lsb
    }

    pub fn write_u16(&mut self, address: u16, value: u16) {
        let lsb = value as u8;
        let msb = (value >> 8) as u8;
        self.write(address, lsb);
        self.write(address + 1, msb);
    }

    // Clock all devices on the cpu bus relative to a cpu cycle
    pub fn clock(&mut self) {
        self.apu.clock(self.cpu_cycles_total);
        for _ in 0..3 {
            match self.ppu.clock() {
                PpuClockReport::None => (),
                PpuClockReport::Nmi => self.require_nmi = true,
            }
        }
        self.cpu_cycles_since_inst += 1;
        self.cpu_cycles_total += 1;
        if let Some(address) = self.apu.channels.dmc.require_dma_at {
            // TODO: stall cycles and some register conflict stuff
            self.apu.channels.dmc.on_dma_read(self.peek_read(address));
        }
    }

    pub fn irq_status(&self) -> bool {
        self.apu.irq_status() | self.cartridge().map(|c| c.irq_status()).unwrap_or(false)
    }

    pub fn require_nmi(&mut self) -> bool {
        let status = self.require_nmi;
        self.require_nmi = false;
        status
    }

    pub fn cartridge_mut(&mut self) -> Option<&mut Cartridge> {
        self.ppu.registers.bus.cartridge.as_mut()
    }

    pub fn cartridge(&self) -> Option<&Cartridge> {
        self.ppu.registers.bus.cartridge.as_ref()
    }

    fn oam_dma(&mut self, address_start: u16) {
        // 1 (or 2 if odd) idle cycles
        self.clock();
        if self.cpu_cycles_total % 2 == 1 {
            self.clock();
        }

        // 512 r/w cycles
        for i in 0..256 {
            let value = self.read(address_start + i);
            self.clock();
            self.ppu.registers.write_oam_data(value);
        }
    }
}

#[derive(Default, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct IrqStatus {
    pub status: bool,
    enabled: bool,
    just_on: bool,
}

impl IrqStatus {
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !self.enabled {
            self.status = false;
        }
    }

    pub fn on(&mut self) {
        if !self.status && self.enabled {
            self.status = true;
            self.just_on = true;
        } else {
            self.just_on = false;
        }
    }

    pub fn read_status(&mut self) -> bool {
        let status = self.status;
        if !self.just_on {
            self.status = false;
        }
        status
    }
}
