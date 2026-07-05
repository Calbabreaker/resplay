use crate::cpu::IrqStatus;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameCounterState {
    Quarter,
    Half,
    None,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct FrameCounter {
    cycles_counter: i32,
    five_step_mode: bool,
    pub irq: IrqStatus,
}

impl FrameCounter {
    /// Write to 0x4017
    pub fn write(&mut self, value: u8) -> FrameCounterState {
        self.five_step_mode = value & 0b1000_0000 != 0;
        self.irq.set_enabled(value & 0b0100_0000 == 0);
        // Delay counter reset by 3 or 4 cycles whether or not on a APU cycle
        self.cycles_counter = if self.cycles_counter % 2 == 0 { -3 } else { -4 };
        if self.five_step_mode {
            FrameCounterState::Half
        } else {
            FrameCounterState::None
        }
    }

    /// Ran on every cpu cycle
    pub fn clock(&mut self) -> FrameCounterState {
        let mut state = FrameCounterState::None;
        // Sequence from https://www.nesdev.org/wiki/APU_Frame_Counter
        // Note that this is in CPU cycles and includes one cycle delay
        match self.cycles_counter {
            7457 => state = FrameCounterState::Quarter,
            14913 => state = FrameCounterState::Half,
            22371 => state = FrameCounterState::Quarter,

            29828 if !self.five_step_mode => self.irq.on(),
            29829 if !self.five_step_mode => {
                self.irq.on();
                state = FrameCounterState::Half
            }
            29830 if !self.five_step_mode => {
                // This is meant to be cycle 0 so we skip the first cycle for the next loop
                self.cycles_counter = 1;
                self.irq.on();
            }

            37281 if self.five_step_mode => {
                self.cycles_counter = 0;
                state = FrameCounterState::Half
            }
            _ => (),
        };
        self.cycles_counter += 1;
        state
    }
}

/// Values from https://www.nesdev.org/wiki/APU_Length_Counter
const LENGTH_COUNTER_VALUES: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, //
    12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
];

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct LengthCounter {
    pub halt: bool,
    counter: u8,
    enabled: bool,
}

impl LengthCounter {
    pub fn set_counter(&mut self, value: u8) {
        if self.enabled {
            self.counter = LENGTH_COUNTER_VALUES[(value >> 3) as usize];
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if !enabled {
            self.counter = 0;
        }
        self.enabled = enabled;
    }

    /// Clocked every frame counter half frame
    pub fn clock(&mut self) {
        if !self.halt && self.counter != 0 {
            self.counter -= 1;
        }
    }

    pub fn playing(&self) -> bool {
        self.counter > 0
    }
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct TimerCounter<T> {
    pub start: T,
    pub counter: T,
}

impl<T: Eq + std::ops::SubAssign + Copy + From<u8>> TimerCounter<T> {
    /// Decrements the timer counter returning true when zero then reload the counter with start
    pub fn clock(&mut self) -> bool {
        if self.counter == T::from(0) {
            self.counter = self.start;
            true
        } else {
            self.counter -= T::from(1);
            false
        }
    }
}
