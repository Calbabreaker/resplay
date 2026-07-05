use super::counters::TimerCounter;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Sequencer {
    /// 11 bit number for the sequencer to go to the next step
    pub timer: TimerCounter<u16>,
    /// 11 bit number for the timer start
    pub step: usize,
}

impl Sequencer {
    pub fn clock(&mut self, sequence_length: usize) {
        if self.timer.clock() {
            self.step += 1;
            if self.step >= sequence_length {
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
