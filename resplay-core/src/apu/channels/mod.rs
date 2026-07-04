use crate::apu::ApuConfig;

use super::{Status, counters::FrameCounterState};

mod dmc;
mod noise;
mod pulse;
mod triangle;

#[derive(Default)]
pub struct Channels {
    pulse_0: pulse::PulseChannel<0>,
    pulse_1: pulse::PulseChannel<1>,
    triangle: triangle::TriangleChannel,
    noise: noise::NoiseChannel,
    pub(crate) dmc: dmc::DmcChannel,
}

impl Channels {
    pub fn write(&mut self, address: u16, value: u8) {
        std::debug_assert_matches!(address, 0x4000..=0x4013);
        self.pulse_0.write(address, value, 0);
        self.pulse_1.write(address, value, 1);
        self.triangle.write(address, value);
        self.noise.write(address, value);
        self.dmc.write(address, value);
    }

    pub fn clock(&mut self, cpu_cycles: u64) {
        self.triangle.clock();
        if cpu_cycles.is_multiple_of(2) {
            self.pulse_0.sequencer.clock();
            self.pulse_1.sequencer.clock();
            self.noise.clock();
            self.dmc.clock();
        }
    }

    pub fn handle_frame_state(&mut self, state: FrameCounterState) {
        if state == FrameCounterState::Half {
            // Half frame clocks
            self.pulse_0.length_counter.clock();
            self.pulse_0.sweep.clock(&mut self.pulse_0.sequencer);
            self.pulse_1.length_counter.clock();
            self.pulse_1.sweep.clock(&mut self.pulse_1.sequencer);
            self.triangle.length_counter.clock();
            self.noise.length_counter.clock();
        }

        if matches!(state, FrameCounterState::Half | FrameCounterState::Quarter) {
            // Quarter frame clocks
            self.pulse_0.envelope.clock();
            self.pulse_1.envelope.clock();
            self.noise.envelope.clock();
            self.triangle.clock_linear_counter();
        }
    }

    pub fn set_enabled(&mut self, status: Status) {
        self.pulse_0
            .length_counter
            .set_enabled(status.contains(Status::PULSE_0));
        self.pulse_1
            .length_counter
            .set_enabled(status.contains(Status::PULSE_1));
        self.triangle
            .length_counter
            .set_enabled(status.contains(Status::TRIANGLE));
        self.noise
            .length_counter
            .set_enabled(status.contains(Status::NOISE));
        self.dmc.set_enabled(status.contains(Status::DMC));
    }

    pub fn get_status(&self) -> Status {
        let mut status = Status::empty();
        status.set(Status::PULSE_0, self.pulse_0.length_counter.playing());
        status.set(Status::PULSE_1, self.pulse_1.length_counter.playing());
        status.set(Status::TRIANGLE, self.triangle.length_counter.playing());
        status.set(Status::NOISE, self.noise.length_counter.playing());
        status.set(Status::DMC, self.dmc.playing());
        status.set(Status::DMC_IRQ, self.dmc.irq.status);
        status
    }

    pub fn sample(&self, config: &ApuConfig) -> f32 {
        let pulse_0 = self.pulse_0.sample() as f32 * config.pulse_0_volume;
        let pulse_1 = self.pulse_1.sample() as f32 * config.pulse_1_volume;
        let noise = self.noise.sample() as f32 * config.noise_volume;
        let triangle = self.triangle.sample() as f32 * config.triangle_volume;
        let dmc = self.dmc.sample() as f32 * config.dmc_volume;

        // Math from https://www.nesdev.org/wiki/APU_Mixer
        let pulse = pulse_0 + pulse_1;
        let tnd = triangle / 8227. + noise / 12241. + dmc / 22638.;
        let pulse_out = (95.88 * pulse) / (8128. + 100. * pulse);
        let tnd_out = (159.79 * tnd) / (1. + 100. * tnd);
        (tnd_out + pulse_out) * config.volume
    }
}
