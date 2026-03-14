pub mod export;
pub mod mixer;

use std::num::NonZero;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use rodio::{DeviceSinkBuilder, MixerDeviceSink, Source};

use crate::project::{Pattern, Track};

use mixer::{Command, PatternSnapshot, PlaybackSettings, ScopeBuffer, TrackerSource};

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
    pub channel_scopes: Arc<Vec<ScopeBuffer>>,
    pub playback_row: Arc<AtomicUsize>,
    pub playback_order: Arc<AtomicUsize>,
    pub playback_ended: Arc<AtomicBool>,
    sender: mpsc::Sender<Command>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let playback_row = Arc::new(AtomicUsize::new(0));
        let playback_order = Arc::new(AtomicUsize::new(0));
        let playback_ended = Arc::new(AtomicBool::new(false));
        let peak_level = Arc::new(AtomicU32::new(0u32));
        let channel_scopes: Arc<Vec<ScopeBuffer>> =
            Arc::new((0..32).map(|_| ScopeBuffer::new()).collect());

        let source = TrackerSource::new(
            receiver,
            playback_row.clone(),
            playback_order.clone(),
            playback_ended.clone(),
            channel_scopes.clone(),
        );
        let monitored = PeakMonitor::new(source, peak_level.clone());

        let mut device_sink = DeviceSinkBuilder::from_default_device()
            .expect("Failed to open audio output")
            .with_buffer_size(rodio::cpal::BufferSize::Fixed(128))
            .open_sink_or_fallback()
            .expect("Failed to open audio output");
        device_sink.log_on_drop(false);
        device_sink.mixer().add(monitored);

        Self {
            _device_sink: device_sink,
            peak_level,
            channel_scopes,
            playback_row,
            playback_order,
            playback_ended,
            sender,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn start_playback(
        &self,
        row: usize,
        order_idx: usize,
        patterns: &[Pattern],
        order: &[usize],
        tracks: &[Track],
        master_volume: f32,
        muted_channels: &[bool],
    ) {
        let snapshots: Vec<Arc<PatternSnapshot>> = patterns
            .iter()
            .map(|p| Arc::new(PatternSnapshot::from_pattern(p)))
            .collect();
        let settings = Arc::new(PlaybackSettings {
            master_volume,
            tracks: tracks.to_vec(),
            muted_channels: muted_channels.to_vec(),
        });
        let _ = self.sender.send(Command::Play {
            start_row: row,
            start_order: order_idx,
            patterns: snapshots,
            order: order.to_vec(),
            settings,
            stop_at_end: true,
        });
    }

    pub fn stop_all(&self) {
        let _ = self.sender.send(Command::Stop);
    }

    pub fn update_settings(
        &self,
        tracks: &[Track],
        master_volume: f32,
        muted_channels: &[bool],
    ) {
        let settings = Arc::new(PlaybackSettings {
            master_volume,
            tracks: tracks.to_vec(),
            muted_channels: muted_channels.to_vec(),
        });
        let _ = self.sender.send(Command::UpdateSettings { settings });
    }

    pub fn update_patterns(&self, patterns: &[Pattern], order: &[usize]) {
        let snapshots: Vec<Arc<PatternSnapshot>> = patterns
            .iter()
            .map(|p| Arc::new(PatternSnapshot::from_pattern(p)))
            .collect();
        let _ = self.sender.send(Command::UpdatePatterns {
            patterns: snapshots,
            order: order.to_vec(),
        });
    }

    pub fn preview_notes(
        &self,
        freqs: &[f32],
        track_idx: usize,
        tracks: &[Track],
        master_volume: f32,
    ) {
        let track = &tracks[track_idx % tracks.len()];
        let _ = self.sender.send(Command::PreviewNotes {
            frequencies: freqs.to_vec(),
            volume: track.default_volume,
            panning: track.default_panning,
            vol_envelope: track.vol_envelope.clone(),
            sample_data: Arc::clone(&track.sample_data),
            master_volume,
            vol_fadeout: track.vol_fadeout,
            coarse_tune: track.coarse_tune,
            fine_tune: track.fine_tune,
            pitch_env_enabled: track.pitch_env_enabled,
            pitch_env_depth: track.pitch_env_depth,
            pitch_envelope: track.pitch_envelope.clone(),
            filter: Box::new(track.filter.clone()),
        });
    }
}
