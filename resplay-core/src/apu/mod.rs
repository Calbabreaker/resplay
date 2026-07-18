use ringbuf::traits::Producer;

use channels::Channels;
use counters::FrameCounter;

mod channels;
mod counters;
mod envelope;
mod sequencer;

bitflags::bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Status: u8 {
        const PULSE_0 = 1;
        const PULSE_1 = 1 << 1;
        const TRIANGLE = 1 << 2;
        const NOISE = 1 << 3;
        const DMC = 1 << 4;
        const FRAME_IRQ = 1 << 6;
        const DMC_IRQ = 1 << 7;
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(default)]
pub struct ApuConfig {
    pub volume: f32,
    pub triangle_volume: f32,
    pub pulse_0_volume: f32,
    pub pulse_1_volume: f32,
    pub noise_volume: f32,
    pub dmc_volume: f32,
}

impl Default for ApuConfig {
    fn default() -> Self {
        Self {
            volume: 1.,
            triangle_volume: 1.,
            pulse_0_volume: 1.,
            pulse_1_volume: 1.,
            noise_volume: 1.,
            dmc_volume: 1.,
        }
    }
}

/// Emulated RP2A03 NTSC APU
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Apu {
    #[serde(skip)]
    pub config: ApuConfig,
    pub(crate) channels: Channels,
    frame_counter: FrameCounter,

    #[serde(skip)]
    pub(crate) sender: Option<SampleSender>,
    #[serde(skip)]
    pub speed_scale: f32,
}

impl Apu {
    pub(crate) fn write(&mut self, address: u16, value: u8) {
        std::debug_assert_matches!(address, 0x4000..=0x4017);
        match address {
            0x4000..=0x4013 => self.channels.write(address, value),
            0x4015 => self.channels.set_enabled(Status::from_bits_truncate(value)),
            0x4017 => {
                let state = self.frame_counter.write(value);
                self.channels.handle_frame_state(state);
            }
            _ => (),
        }
    }

    pub(crate) fn read_status(&mut self) -> u8 {
        let mut status = self.channels.get_status();
        status.set(Status::FRAME_IRQ, self.frame_counter.irq.read_status());
        status.bits()
    }

    /// Ran on every CPU cycle
    pub(crate) fn clock(&mut self, cpu_cycles: u64) {
        self.channels.clock(cpu_cycles);

        let state = self.frame_counter.clock();
        self.channels.handle_frame_state(state);

        if let Some(sender) = self.sender.as_mut()
            && self.speed_scale != 0.
        {
            sender.check_send(|| self.channels.sample(&self.config), self.speed_scale);
        }
    }

    pub(crate) fn irq_status(&self) -> bool {
        self.frame_counter.irq.status | self.channels.dmc.irq.status
    }

    pub(crate) fn reset(&mut self) {
        self.channels.set_enabled(Status::empty());
    }
}

pub(crate) struct SampleSender {
    sample_rate: f32,
    buffer_prod: ringbuf::HeapProd<f32>,
    high_pass: OnePoleFilter<true>,
    cycles_since_sample: f32,
}

impl SampleSender {
    pub fn new(sample_rate: f32, buffer_prod: ringbuf::HeapProd<f32>) -> Self {
        Self {
            sample_rate,
            buffer_prod,
            high_pass: OnePoleFilter::default(),
            cycles_since_sample: 0.,
        }
    }

    fn check_send(&mut self, get_sample: impl Fn() -> f32, speed_scale: f32) {
        let real_sample_rate = self.sample_rate / speed_scale;
        while self.cycles_since_sample > 0. {
            let mut sample = get_sample();
            // Include low freq high pass filter to get rid of DC bias
            sample = self.high_pass.process(sample, real_sample_rate, 20.);

            self.buffer_prod.try_push(sample).ok();
            self.cycles_since_sample -= crate::cpu::CLOCK_SPEED_HZ / real_sample_rate;
        }
        self.cycles_since_sample += 1.;
    }
}

#[derive(Default)]
struct OnePoleFilter<const HIGH_PASS: bool> {
    prev_out: f32,
    prev_in: f32,
}

impl<const HIGH_PASS: bool> OnePoleFilter<{ HIGH_PASS }> {
    fn process(&mut self, sample: f32, sample_rate: f32, cutoff_freq: f32) -> f32 {
        let rc = 1. / (std::f32::consts::TAU * cutoff_freq);
        let dt = 1. / sample_rate;
        // formulas from wikipedia
        let out = if HIGH_PASS {
            // y[i] := α × y[i−1] + α × (x[i] − x[i−1])
            let alpha = rc / (rc + dt);
            alpha * self.prev_out + alpha * (sample - self.prev_in)
        } else {
            // y[i] := α * x[i] + (1-α) * y[i-1]
            let alpha = dt / (rc + dt);
            alpha * sample + (1. - alpha) * self.prev_out
        };
        self.prev_out = out;
        self.prev_in = sample;
        out
    }
}
