use crate::apu::{
    counters::{LengthCounter, TimerCounter},
    envelope::Envelope,
    sequencer::Sequencer,
};

/// Waveform for the pulse wave for each duty cycle
/// https://www.nesdev.org/wiki/APU_Pulse#Sequencer_behavior
/// This will be multiplied by the envelope's decay level
const PULSE_WAVEFORM: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0],
    [0, 1, 1, 0, 0, 0, 0, 0],
    [0, 1, 1, 1, 1, 0, 0, 0],
    [1, 0, 0, 1, 1, 1, 1, 1],
];

/// Generator for pulse/square wave
pub struct PulseChannel<const NUMBER: u16> {
    pub sequencer: Sequencer,
    pub envelope: Envelope,
    pub length_counter: LengthCounter,
    pub sweep: Sweep<NUMBER>,
}

impl<const NUMBER: u16> Default for PulseChannel<NUMBER> {
    fn default() -> Self {
        Self {
            sequencer: Sequencer::new(&PULSE_WAVEFORM[0]),
            envelope: Envelope::default(),
            length_counter: LengthCounter::default(),
            sweep: Sweep::default(),
        }
    }
}

impl<const NUMBER: u16> PulseChannel<NUMBER> {
    pub fn write(&mut self, address: u16, value: u8, number: u16) {
        match address - number * 4 {
            // 0x4000 or 0x4004 if number == 1 etc
            0x4000 => {
                self.sequencer.sequence = &PULSE_WAVEFORM[(value >> 6) as usize];
                self.envelope.write(value);
                self.length_counter.halt = value & 0b0010_0000 != 0;
            }
            0x4001 => self.sweep.write(value),
            0x4002 => self.sequencer.set_timer_low(value),
            0x4003 => {
                self.envelope.start();
                self.sequencer.set_timer_high(value);
                self.length_counter.set_counter(value);
                self.sequencer.step = 0;
            }
            _ => (),
        }
    }

    pub fn sample(&self) -> u8 {
        if self.length_counter.playing() && !self.sweep.muted(&self.sequencer) {
            self.sequencer.sample() * self.envelope.volume()
        } else {
            0
        }
    }
}

#[derive(Default)]
pub struct Sweep<const NUMBER: u16> {
    enabled: bool,
    timer: TimerCounter<u8>,
    negate: bool,
    shift_count: u8,
}

impl<const NUMBER: u16> Sweep<NUMBER> {
    fn write(&mut self, value: u8) {
        self.enabled = value & 0b1000_0000 != 0;
        self.timer.start = (value & 0b0111_0000) >> 4;
        self.timer.counter = self.timer.start + 1;
        self.negate = value & 0b0000_1000 != 0;
        self.shift_count = value & 0b0000_0111;
    }

    /// Clocked on every half frame
    /// Calculates target period of period
    pub fn clock(&mut self, sequencer: &mut Sequencer) {
        if self.timer.clock() && self.enabled && self.shift_count != 0 && !self.muted(sequencer) {
            sequencer.timer.start = self.target_period(sequencer);
        }
    }

    fn muted(&self, sequencer: &Sequencer) -> bool {
        sequencer.timer.start < 8 || self.target_period(sequencer) > 0x7ff
    }

    fn target_period(&self, sequencer: &Sequencer) -> u16 {
        let period = sequencer.timer.start;
        let change = period >> self.shift_count;
        // Rotate right an 11 bit number
        if self.negate {
            period
                .saturating_sub(change)
                .saturating_sub(if NUMBER == 0 { 1 } else { 0 })
        } else {
            period + change
        }
    }
}
