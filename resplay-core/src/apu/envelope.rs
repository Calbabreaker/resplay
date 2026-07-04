use super::counters::TimerCounter;

const DECAY_START: u8 = 15;

#[derive(Default)]
pub struct Envelope {
    timer: TimerCounter<u8>,
    constant_volume: bool,
    should_loop: bool,
    start: bool,
    decay_level: u8,
}

impl Envelope {
    /// Writes to 0x4000, 0x4004, 0x400c
    pub fn write(&mut self, value: u8) {
        self.timer.start = value & 0x0f;
        self.constant_volume = value & 0b0001_0000 != 0;
        self.should_loop = value & 0b0010_0000 != 0;
    }

    /// Clocked by every quarter frame frame counter
    pub fn clock(&mut self) {
        if self.start {
            self.decay_level = DECAY_START;
            self.timer.counter = self.timer.start;
            self.start = false;
        } else if self.timer.clock() {
            if self.decay_level != 0 {
                self.decay_level -= 1;
            } else if self.should_loop {
                self.decay_level = DECAY_START;
            }
        }
    }

    pub fn start(&mut self) {
        self.start = true;
    }

    pub fn volume(&self) -> u8 {
        if self.constant_volume {
            self.timer.start
        } else {
            self.decay_level
        }
    }
}
