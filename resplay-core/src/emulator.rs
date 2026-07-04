use ringbuf::traits::Split;

use crate::{
    Apu, Cartridge, Controller, Cpu, Ppu,
    cartridge::NesParseError,
    cpu::{CLOCK_SPEED_HZ, CYCLES_PER_FRAME, CpuError},
    ppu::ScreenPixels,
};

/// High level struct for controlling the cpu
pub struct Emulator {
    pub cpu: Cpu,
    pub speed: f32,
    pub running: bool,
    clocks_remaining: f32,
    last_update_time: f32,
    last_frame_time: f32,
    frame_rate: f32,
    audio_sample_rate: f32,
}

impl Default for Emulator {
    fn default() -> Self {
        Self {
            last_update_time: 0.,
            running: true,
            cpu: Cpu::default(),
            last_frame_time: 0.,
            audio_sample_rate: 0.,
            frame_rate: 0.,
            clocks_remaining: 0.,
            speed: 1.,
        }
    }
}

impl Emulator {
    /// Keep stepping until a frame is generated
    pub fn next_frame(&mut self) -> Result<(), CpuError> {
        while !self.ppu().frame_complete() {
            self.cpu.execute_next()?;
        }
        Ok(())
    }

    /// Calculates the delta time that has passed since calling this function and clock the cpu
    /// required for that amount of time
    pub fn update(
        &mut self,
        now_secs: f32,
        mut on_frame_completed: impl FnMut(&ScreenPixels),
    ) -> Result<(), CpuError> {
        let delta = (now_secs - self.last_update_time).min(0.05) * self.speed;
        self.apu().sample_rate = self.audio_sample_rate / self.speed;
        self.last_update_time = now_secs;
        if !self.running {
            self.clocks_remaining = 0.;
            return Ok(());
        }

        self.clocks_remaining += delta * CLOCK_SPEED_HZ;
        while self.clocks_remaining > 0. {
            self.clocks_remaining -= self.cpu.execute_next()? as f32;
            if self.ppu().frame_complete() && self.clocks_remaining < CYCLES_PER_FRAME {
                self.frame_rate = 1. / (now_secs - self.last_frame_time);
                self.last_frame_time = now_secs;
                on_frame_completed(&self.ppu().screen_pixels);
            }
        }
        Ok(())
    }

    /// Setup the audio buffer
    /// Returns the ring buffer consumer that contains the samples generated from the APU
    pub fn setup_audio_buffer(
        &mut self,
        sample_rate: u32,
        buffer_length: std::time::Duration,
    ) -> ringbuf::HeapCons<f32> {
        self.audio_sample_rate = sample_rate as f32;
        self.apu().sample_rate = self.audio_sample_rate / self.speed;
        let size = self.audio_sample_rate * buffer_length.as_secs_f32();
        let rb = ringbuf::SharedRb::new(size as usize);
        let (prod, cons) = rb.split();
        self.apu().buffer_prod = Some(prod);
        cons
    }

    pub fn load_nes_file(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), NesParseError> {
        self.load_nes_rom(std::fs::File::open(path)?)
    }

    pub fn load_nes_rom(&mut self, bytes: impl std::io::Read) -> Result<(), NesParseError> {
        self.ppu().registers.bus.cartridge = Some(Cartridge::from_nes(bytes)?);
        self.cpu.reset();
        Ok(())
    }

    pub fn frame_rate(&self) -> f32 {
        self.frame_rate
    }

    pub fn ppu(&mut self) -> &mut Ppu {
        &mut self.cpu.bus.ppu
    }

    pub fn apu(&mut self) -> &mut Apu {
        &mut self.cpu.bus.apu
    }

    pub fn cartridge(&self) -> Option<&Cartridge> {
        self.cpu.bus.cartridge()
    }

    pub fn controller(&mut self, number: u8) -> &mut Controller {
        &mut self.cpu.bus.controllers[number as usize]
    }
}
