use crate::apu::{counters::LengthCounter, sequencer::Sequencer};

const TRIANGLE_WAVEFORM: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, //
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
];

pub struct TriangleChannel {
    pub length_counter: LengthCounter,
    pub linear_counter: u8,
    linear_counter_reload: bool,
    linear_counter_reload_value: u8,
    pub sequencer: Sequencer,
}

impl Default for TriangleChannel {
    fn default() -> Self {
        Self {
            linear_counter: 0,
            linear_counter_reload: true,
            linear_counter_reload_value: 0,
            length_counter: LengthCounter::default(),
            sequencer: Sequencer::new(&TRIANGLE_WAVEFORM),
        }
    }
}

impl TriangleChannel {
    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0x4008 => {
                self.length_counter.halt = value & 0b1000_0000 != 0;
                self.linear_counter_reload_value = value & 0b0111_1111;
            }
            0x4009 => (),
            0x400a => self.sequencer.set_timer_low(value),
            0x400b => {
                self.sequencer.set_timer_high(value);
                self.length_counter.set_counter(value);
                self.linear_counter_reload = true;
            }
            _ => (),
        }
    }

    pub fn clock_linear_counter(&mut self) {
        if self.linear_counter_reload {
            self.linear_counter = self.linear_counter_reload_value;
        } else if self.linear_counter != 0 {
            self.linear_counter -= 1;
        }
        if !self.length_counter.halt {
            self.linear_counter_reload = false;
        }
    }

    pub fn clock(&mut self) {
        // Clock if not muted
        if self.length_counter.playing()
            && self.linear_counter != 0
            // Prevent high frequencies from some games using this to silence the channel
            && self.sequencer.timer.start > 1
        {
            self.sequencer.clock();
        }
    }

    pub fn sample(&self) -> u8 {
        self.sequencer.sample()
    }
}
