use crate::apu::{
    counters::{LengthCounter, TimerCounter},
    envelope::Envelope,
};

/// Period values from https://www.nesdev.org/wiki/APU_Noise
/// Note this is halfed since it is in APU cycles
const NOISE_PERIODS: [u16; 16] = [
    2, 4, 8, 16, 32, 48, 64, 80, 101, 127, 190, 254, 381, 508, 1017, 2034,
];

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct NoiseChannel {
    pub timer: TimerCounter<u16>,
    pub length_counter: LengthCounter,
    pub envelope: Envelope,
    pub mode_flag: bool,
    pub shift_register: u16,
}

impl NoiseChannel {
    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0x400c => {
                self.envelope.write(value);
                self.length_counter.halt = value & 0b0010_0000 != 0;
            }
            0x400d => (),
            0x400e => {
                self.mode_flag = value & 0b1000_0000 != 0;
                self.timer.start = NOISE_PERIODS[(value & 0x0f) as usize];
            }
            0x400f => {
                self.envelope.start();
                self.length_counter.set_counter(value);
            }
            _ => (),
        }
    }

    pub fn clock(&mut self) {
        if self.timer.clock() {
            if self.shift_register == 0 {
                self.shift_register = 1;
            }

            let bit_0 = self.shift_register & 1;
            // Bit 6 if mode flag set otherwise bit 1
            let bit_1 = (self.shift_register >> (if self.mode_flag { 6 } else { 1 })) & 1;
            self.shift_register >>= 1;
            self.shift_register |= (bit_0 ^ bit_1) << 14;
        }
    }

    pub fn sample(&self) -> u8 {
        if self.length_counter.playing() && self.shift_register & 1 == 0 {
            self.envelope.volume()
        } else {
            0
        }
    }
}
