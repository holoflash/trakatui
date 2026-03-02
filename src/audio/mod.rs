pub mod export;
pub mod mixer;

use std::num::NonZero;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use rodio::{DeviceSinkBuilder, MixerDeviceSink, Source};

use crate::project::{ChannelSettings, Pattern};

use mixer::{Command, PatternSnapshot, PlaybackSettings, TrackerSource};

pub struct PeakMonitor<S> {
    source: S,
    peak: Arc<AtomicU32>,
}

impl<S> PeakMonitor<S> {
    pub const fn new(source: S, peak: Arc<AtomicU32>) -> Self {
        Self { source, peak }
    }
}

impl<S: Source<Item = f32>> Iterator for PeakMonitor<S> {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.source.next()?;
        let abs = sample.abs();
        let mut current = self.peak.load(Ordering::Relaxed);
        loop {
            let current_f = f32::from_bits(current);
            if abs <= current_f {
                break;
            }
            match self.peak.compare_exchange_weak(
                current,
                abs.to_bits(),
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(c) => current = c,
            }
        }
        Some(sample)
    }
}

impl<S: Source<Item = f32>> Source for PeakMonitor<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.source.current_span_len()
    }

    fn channels(&self) -> NonZero<u16> {
        self.source.channels()
    }

    fn sample_rate(&self) -> NonZero<u32> {
        self.source.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }
}

pub struct AudioEngine {
    _device_sink: MixerDeviceSink,
    pub peak_level: Arc<AtomicU32>,
    pub playback_row: Arc<AtomicUsize>,
    sender: mpsc::Sender<Command>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let playback_row = Arc::new(AtomicUsize::new(0));
        let peak_level = Arc::new(AtomicU32::new(0u32));

        let source = TrackerSource::new(receiver, playback_row.clone());
        let monitored = PeakMonitor::new(source, peak_level.clone());

        let mut device_sink =
            DeviceSinkBuilder::open_default_sink().expect("Failed to open audio output");
        device_sink.log_on_drop(false);
        device_sink.mixer().add(monitored);

        Self {
            _device_sink: device_sink,
            peak_level,
            playback_row,
            sender,
        }
    }

    pub fn start_playback(
        &self,
        row: usize,
        pattern: &Pattern,
        channel_settings: &[ChannelSettings],
        bpm: u16,
        master_volume: f32,
    ) {
        let snapshot = Arc::new(PatternSnapshot::from_pattern(pattern));
        let settings = Arc::new(PlaybackSettings {
            bpm,
            master_volume,
            channel_settings: channel_settings.to_vec(),
        });
        let _ = self.sender.send(Command::Play {
            start_row: row,
            pattern: snapshot,
            settings,
        });
    }

    pub fn stop_all(&self) {
        let _ = self.sender.send(Command::Stop);
    }

    pub fn update_settings(
        &self,
        channel_settings: &[ChannelSettings],
        bpm: u16,
        master_volume: f32,
    ) {
        let settings = Arc::new(PlaybackSettings {
            bpm,
            master_volume,
            channel_settings: channel_settings.to_vec(),
        });
        let _ = self.sender.send(Command::UpdateSettings { settings });
    }

    pub fn update_pattern(&self, pattern: &Pattern) {
        let snapshot = Arc::new(PatternSnapshot::from_pattern(pattern));
        let _ = self
            .sender
            .send(Command::UpdatePattern { pattern: snapshot });
    }

    pub fn preview_note(
        &self,
        freq: f32,
        channel: usize,
        channel_settings: &[ChannelSettings],
        master_volume: f32,
    ) {
        let cs = &channel_settings[channel % channel_settings.len()];
        let _ = self.sender.send(Command::PreviewNote {
            frequency: freq,
            waveform: cs.waveform,
            volume: 1.0,
            envelope: cs.envelope,
            sample_data: cs.sample_data.clone(),
            master_volume,
        });
    }
}
