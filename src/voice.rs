// Capri's sounds, synthesized directly in memory into PCM WAV buffers
// and played asynchronously via PipeWire / PulseAudio / ALSA.
// Custom sounds can also be placed in ~/.config/capri/sounds/ to override any cue.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

pub const CUES: &[&str] = &[
    "teleport-out",
    "teleport-in",
    "done",
    "alert",
    "error",
    "hello",
    "blip",
    "tick",
    "star",
    "land",
    "rise",
    "bolt",
    "charge",
    "possess",
];

const BLIP_NOTES: &[f64] = &[62.0, 65.0, 67.0, 69.0, 72.0, 74.0, 77.0, 79.0];

fn midi_to_freq(n: f64) -> f64 {
    440.0 * 2.0_f64.powf((n - 69.0) / 12.0)
}

fn write_wav_header(out: &mut Vec<u8>, num_samples: usize, sample_rate: u32) {
    let data_size = (num_samples * 2) as u32;
    let file_size = 36 + data_size;

    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&file_size.to_le_bytes());
    out.extend_from_slice(b"WAVE");

    // "fmt " chunk
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // subchunk size = 16
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

    // "data" chunk
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_size.to_le_bytes());
}

fn synth_samples_to_wav(samples: &[f32], sample_rate: u32, volume: f32) -> Vec<u8> {
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    write_wav_header(&mut out, samples.len(), sample_rate);
    for &s in samples {
        let clamped = (s * volume).clamp(-1.0, 1.0);
        let sample_i16 = (clamped * 32767.0) as i16;
        out.extend_from_slice(&sample_i16.to_le_bytes());
    }
    out
}

struct Synth {
    sample_rate: u32,
    buffer: Vec<f32>,
}

impl Synth {
    fn new(duration_sec: f32) -> Self {
        let sample_rate = 44100;
        let count = (duration_sec * sample_rate as f32) as usize;
        Self {
            sample_rate,
            buffer: vec![0.0; count],
        }
    }

    fn add_osc(
        &mut self,
        wave: &str,
        start_time: f32,
        duration: f32,
        freq_start: f64,
        freq_end: f64,
        peak: f32,
    ) {
        let sr = self.sample_rate as f32;
        let start_idx = (start_time * sr) as usize;
        let count = (duration * sr) as usize;
        let mut phase = 0.0_f64;

        for i in 0..count {
            let idx = start_idx + i;
            if idx >= self.buffer.len() {
                break;
            }
            let progress = i as f64 / count as f64;
            let freq = freq_start + (freq_end - freq_start) * progress;
            phase += freq / sr as f64;
            if phase > 1.0 {
                phase -= phase.floor();
            }

            let raw = match wave {
                "saw" => (2.0 * phase - 1.0) as f32,
                "square" => {
                    if phase < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                "triangle" => (4.0 * (phase - 0.5).abs() - 1.0) as f32,
                _ => (2.0 * std::f64::consts::PI * phase).sin() as f32,
            };

            // Attack & decay envelope
            let t = i as f32 / count as f32;
            let env = if t < 0.08 {
                t / 0.08
            } else {
                (1.0 - t).powf(1.8)
            };

            self.buffer[idx] += raw * peak * env;
        }
    }

    fn add_noise(
        &mut self,
        start_time: f32,
        duration: f32,
        _from_freq: f32,
        _to_freq: f32,
        peak: f32,
    ) {
        let sr = self.sample_rate as f32;
        let start_idx = (start_time * sr) as usize;
        let count = (duration * sr) as usize;
        let mut prev = 0.0_f32;

        for i in 0..count {
            let idx = start_idx + i;
            if idx >= self.buffer.len() {
                break;
            }
            let white = rand::random::<f32>() * 2.0 - 1.0;
            // Simple low/bandpass approximation
            let filtered = 0.3 * white + 0.7 * prev;
            prev = filtered;

            let t = i as f32 / count as f32;
            let env = if t < 0.05 {
                t / 0.05
            } else {
                (1.0 - t).powf(1.5)
            };

            self.buffer[idx] += filtered * peak * env;
        }
    }

    fn build(self, volume: f32) -> Vec<u8> {
        synth_samples_to_wav(&self.buffer, self.sample_rate, volume)
    }
}

pub struct Voice {
    pub muted: bool,
    pub volume: f32,
    custom_sounds: HashMap<String, PathBuf>,
    cache: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl Voice {
    pub fn new() -> Self {
        let mut custom = HashMap::new();
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let sounds_dir = PathBuf::from(home).join(".config").join("capri").join("sounds");

        if let Ok(entries) = fs::read_dir(sounds_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    let lower = stem.to_lowercase();
                    if CUES.contains(&lower.as_str()) {
                        custom.insert(lower, p);
                    }
                }
            }
        }

        Self {
            muted: false,
            volume: 0.7,
            custom_sounds: custom,
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn play(&self, cue: &str) {
        if self.muted {
            return;
        }

        if let Some(custom_path) = self.custom_sounds.get(cue) {
            let p = custom_path.clone();
            thread::spawn(move || {
                let _ = Command::new("pw-play")
                    .arg(&p)
                    .spawn()
                    .or_else(|_| Command::new("paplay").arg(&p).spawn())
                    .or_else(|_| Command::new("aplay").arg(&p).spawn());
            });
            return;
        }

        let wav_data = {
            let mut cache = self.cache.lock().unwrap();
            if let Some(data) = cache.get(cue) {
                data.clone()
            } else {
                let generated = self.synth_cue(cue);
                cache.insert(cue.to_string(), generated.clone());
                generated
            }
        };

        thread::spawn(move || {
            let temp_path = PathBuf::from("/tmp").join(format!("capri_cue_{}.wav", std::process::id()));
            if let Ok(mut f) = fs::File::create(&temp_path) {
                let _ = f.write_all(&wav_data);
                let _ = Command::new("pw-play")
                    .arg(&temp_path)
                    .status()
                    .or_else(|_| Command::new("paplay").arg(&temp_path).status())
                    .or_else(|_| Command::new("aplay").arg(&temp_path).status());
                let _ = fs::remove_file(temp_path);
            }
        });
    }

    fn synth_cue(&self, cue: &str) -> Vec<u8> {
        let vol = self.volume;
        match cue {
            "teleport-out" => {
                let mut s = Synth::new(0.3);
                s.add_osc("saw", 0.0, 0.22, 180.0, 2400.0, 0.25);
                s.add_noise(0.0, 0.2, 400.0, 7000.0, 0.2);
                s.build(vol)
            }
            "teleport-in" => {
                let mut s = Synth::new(0.35);
                s.add_osc("saw", 0.0, 0.22, 1600.0, 140.0, 0.25);
                s.add_noise(0.0, 0.2, 6000.0, 500.0, 0.25);
                s.add_osc("triangle", 0.16, 0.18, 70.0, 45.0, 0.35);
                s.build(vol)
            }
            "done" => {
                let mut s = Synth::new(0.65);
                let notes = [64.0, 68.0, 71.0, 75.0, 76.0];
                for (i, &n) in notes.iter().enumerate() {
                    let at = i as f32 * 0.07;
                    s.add_osc("triangle", at, 0.28, midi_to_freq(n + 12.0), midi_to_freq(n + 12.0), 0.2);
                    s.add_osc("square", at, 0.12, midi_to_freq(n), midi_to_freq(n), 0.12);
                }
                s.build(vol)
            }
            "alert" => {
                let mut s = Synth::new(0.5);
                for r in 0..2 {
                    let at = r as f32 * 0.24;
                    s.add_osc("square", at, 0.09, 988.0, 988.0, 0.2);
                    s.add_osc("square", at + 0.11, 0.11, 740.0, 740.0, 0.2);
                }
                s.build(vol)
            }
            "error" => {
                let mut s = Synth::new(0.4);
                s.add_osc("saw", 0.0, 0.32, 110.0, 55.0, 0.3);
                s.add_noise(0.0, 0.25, 900.0, 200.0, 0.25);
                s.build(vol)
            }
            "hello" => {
                let mut s = Synth::new(0.35);
                let notes = [69.0, 76.0, 81.0];
                for (i, &n) in notes.iter().enumerate() {
                    let at = i as f32 * 0.06;
                    s.add_osc("square", at, 0.07, midi_to_freq(n), midi_to_freq(n), 0.2);
                }
                s.add_noise(0.18, 0.08, 2000.0, 8000.0, 0.15);
                s.build(vol)
            }
            "blip" => {
                let mut s = Synth::new(0.08);
                let note = BLIP_NOTES[rand::random::<usize>() % BLIP_NOTES.len()];
                let f = midi_to_freq(note);
                s.add_osc("square", 0.0, 0.035, f, f, 0.18);
                s.add_osc("triangle", 0.004, 0.03, f * 1.5, f * 1.5, 0.15);
                s.build(vol)
            }
            "tick" => {
                let mut s = Synth::new(0.05);
                s.add_noise(0.0, 0.03, 3000.0, 9000.0, 0.15);
                s.build(vol)
            }
            "star" => {
                let mut s = Synth::new(1.3);
                s.add_osc("sine", 0.0, 0.9, midi_to_freq(93.0), midi_to_freq(93.0), 0.2);
                s.add_osc("sine", 0.14, 1.1, midi_to_freq(100.0), midi_to_freq(100.0), 0.15);
                s.add_noise(0.05, 0.6, 9000.0, 12000.0, 0.08);
                s.build(vol)
            }
            "land" => {
                let mut s = Synth::new(0.2);
                s.add_osc("triangle", 0.0, 0.14, 150.0, 48.0, 0.35);
                s.add_noise(0.0, 0.06, 1800.0, 400.0, 0.18);
                s.build(vol)
            }
            "rise" => {
                let mut s = Synth::new(0.25);
                s.add_osc("saw", 0.0, 0.2, 160.0, 1400.0, 0.25);
                s.add_noise(0.0, 0.18, 500.0, 6000.0, 0.2);
                s.build(vol)
            }
            "bolt" => {
                let mut s = Synth::new(0.35);
                s.add_osc("saw", 0.0, 0.3, 62.0, 58.0, 0.25);
                s.add_noise(0.0, 0.2, 2500.0, 8000.0, 0.28);
                s.build(vol)
            }
            "charge" => {
                let mut s = Synth::new(0.7);
                s.add_osc("saw", 0.0, 0.65, 55.0, 240.0, 0.3);
                s.add_noise(0.1, 0.5, 3000.0, 7000.0, 0.2);
                s.build(vol)
            }
            "possess" => {
                let mut s = Synth::new(0.5);
                let notes = [79.0, 76.0, 72.0];
                for (i, &n) in notes.iter().enumerate() {
                    let at = i as f32 * 0.11;
                    s.add_osc("square", at, 0.07, midi_to_freq(n), midi_to_freq(n), 0.2);
                    s.add_osc("triangle", at + 0.008, 0.07, midi_to_freq(n - 12.0), midi_to_freq(n - 12.0), 0.18);
                }
                s.add_noise(0.0, 0.3, 1200.0, 5000.0, 0.16);
                s.build(vol)
            }
            _ => Synth::new(0.1).build(vol),
        }
    }
}
