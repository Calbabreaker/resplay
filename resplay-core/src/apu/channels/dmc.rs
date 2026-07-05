use crate::{apu::counters::TimerCounter, cpu::IrqStatus};

/// Rates from https://www.nesdev.org/wiki/APU_DMC
/// Note this is halfed since it is in APU cycles
pub const DMC_RATES: [u16; 16] = [
    214, 190, 170, 160, 143, 127, 113, 107, 95, 80, 71, 64, 53, 42, 36, 27,
];

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct DmcChannel {
    timer: TimerCounter<u16>,
    output_level: u8,
    sample_address: u16,
    sample_length: u16,
    bytes_remaining: u16,
    current_address: u16,
    shift_register: u8,
    bits_remaining: u8,
    pub irq: IrqStatus,
    looping: bool,
    enabled: bool,
    #[serde(skip)]
    pub require_dma_at: Option<u16>,
}

impl DmcChannel {
    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0x4010 => {
                self.timer.start = DMC_RATES[(value & 0x0f) as usize];
                self.looping = value & 0b0100_0000 != 0;
                self.irq.set_enabled(value & 0b1000_0000 != 0);
            }
            0x4011 => self.output_level = value & 0b0111_1111,
            // 11AAAAAA_AA000000
            0x4012 => self.sample_address = 0xc000 | ((value as u16) << 6),
            // 0000LLLL_LLLL0001
            0x4013 => self.sample_length = ((value as u16) << 4) | 1,
            _ => (),
        }
    }

    pub fn clock(&mut self) {
        if self.timer.clock() && self.bits_remaining != 0 {
            if self.shift_register & 1 == 1 {
                if self.output_level <= 125 {
                    self.output_level += 2;
                }
            } else {
                if self.output_level >= 2 {
                    self.output_level -= 2;
                }
            }
            self.shift_register >>= 1;
            self.bits_remaining -= 1;
        }

        if self.bits_remaining == 0 && self.bytes_remaining != 0 && self.enabled {
            self.require_dma_at = Some(self.current_address);
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.irq.status = false;
        if self.enabled {
            if self.bytes_remaining == 0 {
                self.restart();
            }
        } else {
            self.bytes_remaining = 0;
        }
    }

    pub fn sample(&self) -> u8 {
        self.output_level
    }

    pub fn playing(&self) -> bool {
        self.bytes_remaining > 0
    }

    pub fn on_dma_read(&mut self, value: u8) {
        self.require_dma_at = None;
        self.shift_register = value;
        self.bits_remaining = 8;

        if self.current_address == 0xffff {
            self.current_address = 0x8000;
        }
        self.current_address += 1;
        self.bytes_remaining -= 1;
        if self.bytes_remaining == 0 {
            if self.looping {
                self.restart();
            } else {
                self.irq.on();
            }
        }
    }

    fn restart(&mut self) {
        self.bytes_remaining = self.sample_length;
        self.current_address = self.sample_address;
    }
}
