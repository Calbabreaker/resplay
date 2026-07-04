use super::counters::TimerCounter;

pub struct Sequencer {
    /// 11 bit number for the sequencer to go to the next step
    pub timer: TimerCounter<u16>,
    /// 11 bit number for the timer start
    pub step: usize,
    pub sequence: &'static [u8],
}

impl Sequencer {
    pub fn new(sequence: &'static [u8]) -> Self {
        Self {
            sequence,
            step: 0,
            timer: TimerCounter::default(),
        }
    }

    pub fn sample(&self) -> u8 {
        self.sequence[self.step]
    }

    pub fn clock(&mut self) {
        if self.timer.clock() {
            self.step += 1;
            if self.step >= self.sequence.len() {
                self.step = 0;
            }
        }
    }

    pub fn set_timer_low(&mut self, value: u8) {
        self.timer.start = (value as u16) | self.timer.start & (0xff00);
    }

    pub fn set_timer_high(&mut self, value: u8) {
        self.timer.start = ((value as u16 & 0b0000_0111) << 8) | self.timer.start & (0x00ff);
    }
}
