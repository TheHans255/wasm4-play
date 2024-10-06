use std::collections::HashMap;

use crate::wasm4::{
    self, trace, TONE_MODE1, TONE_MODE2, TONE_MODE3, TONE_MODE4, TONE_NOISE, TONE_NOTE_MODE,
    TONE_PAN_LEFT, TONE_PAN_RIGHT, TONE_PULSE1, TONE_PULSE2, TONE_TRIANGLE,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DutyCycleMode {
    OneEighth,
    OneFourth,
    OneHalf,
    ThreeFourths,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChannelMode {
    Pulse(DutyCycleMode),
    Triangle,
    Noise,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Frequency {
    Zero,
    Hertz(u16),
    MIDINote(u8, u8),
}

impl From<Frequency> for u32 {
    fn from(value: Frequency) -> Self {
        match value {
            Frequency::Zero => 0,
            Frequency::Hertz(hz) => hz as u32,
            Frequency::MIDINote(note, bend) => (bend as u32) << 8 | (note as u32),
        }
    }
}

impl PartialOrd for Frequency {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Frequency::Hertz(f1), Frequency::Hertz(f2)) => u16::partial_cmp(f1, f2),
            (Frequency::MIDINote(n1, b1), Frequency::MIDINote(n2, b2)) => u16::partial_cmp(
                &((*n1 as u16) << 8 & *b1 as u16),
                &((*n2 as u16) << 8 & *b2 as u16),
            ),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PanMode {
    Center,
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MusicNote {
    pub channel_mode: ChannelMode,
    pub frequency_start: Frequency,
    pub frequency_end: Frequency,
    pub duration_sustain: u8,
    pub duration_release: u8,
    pub duration_decay: u8,
    pub duration_attack: u8,
    pub volume_sustain: u8,
    pub volume_attack: u8,
    pub pan_mode: PanMode,
}

impl MusicNote {
    pub fn new(
        channel_mode: ChannelMode,
        frequency_start: Frequency,
        frequency_end: Frequency,
        duration_sustain: u8,
        duration_release: u8,
        duration_decay: u8,
        duration_attack: u8,
        volume_sustain: u8,
        volume_attack: u8,
        pan_mode: PanMode,
    ) -> Self {
        Self {
            channel_mode,
            frequency_start,
            frequency_end,
            duration_sustain,
            duration_release,
            duration_decay,
            duration_attack,
            volume_sustain,
            volume_attack,
            pan_mode,
        }
    }

    pub fn play(&self, pulse_channel_id: u8) {
        let f: u32 = (u32::from(self.frequency_end) << 16) | u32::from(self.frequency_start);
        let d: u32 = ((self.duration_attack as u32) << 24)
            | ((self.duration_decay as u32) << 16)
            | ((self.duration_release as u32) << 8)
            | (self.duration_sustain as u32);
        let v: u32 = ((self.volume_attack as u32) << 8) | (self.volume_sustain as u32);
        let flags: u32 = match self.channel_mode {
            ChannelMode::Noise => TONE_NOISE,
            ChannelMode::Triangle => TONE_TRIANGLE,
            ChannelMode::Pulse(duty_cycle) => {
                let pulse_flag = match pulse_channel_id {
                    0 => TONE_PULSE1,
                    _ => TONE_PULSE2,
                };
                let mode_flag = match duty_cycle {
                    DutyCycleMode::OneEighth => TONE_MODE1,
                    DutyCycleMode::OneFourth => TONE_MODE2,
                    DutyCycleMode::OneHalf => TONE_MODE3,
                    DutyCycleMode::ThreeFourths => TONE_MODE4,
                };
                mode_flag | pulse_flag
            }
        } | match self.pan_mode {
            PanMode::Center => 0,
            PanMode::Left => TONE_PAN_LEFT,
            PanMode::Right => TONE_PAN_RIGHT,
        } | match self.frequency_start {
            Frequency::MIDINote(_, _) => TONE_NOTE_MODE,
            _ => 0,
        };
        trace(format!("tone({}, {}, {}, {})", f, d, v, flags));

        wasm4::tone(f, d, v, flags);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Rest {
    pub duration: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrackItem {
    Note(MusicNote),
    Rest(Rest),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Track<'a> {
    pub items: &'a [TrackItem],
    pub priority: u32,
}

#[derive(Debug)]
struct TrackProgress<'a> {
    track: Track<'a>,
    current_item_index: Option<usize>,
    current_item_time_remaining: u32,
}

impl<'a> TrackProgress<'a> {
    fn current_item(&self) -> Option<&'a TrackItem> {
        match self.current_item_index {
            None => None,
            Some(i) => {
                if i > self.track.items.len() {
                    None
                } else {
                    Some(&(self.track.items[i]))
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct SoundPlayer<'a> {
    tracks: HashMap<u32, TrackProgress<'a>>,
    id_seed: u32,
    channel_pulse_1_id: u32,
    channel_pulse_2_id: u32,
    channel_triangle_id: u32,
    channel_noise_id: u32,
}

impl<'a> SoundPlayer<'a> {
    pub fn new() -> Self {
        Self {
            tracks: HashMap::new(),
            id_seed: 1,
            channel_noise_id: 0,
            channel_triangle_id: 0,
            channel_pulse_1_id: 0,
            channel_pulse_2_id: 0,
        }
    }

    pub fn update(&mut self) {
        let mut noise_tracks_to_play: Vec<u32> = Vec::new();
        let mut triangle_tracks_to_play: Vec<u32> = Vec::new();
        let mut pulse_tracks_to_play: Vec<u32> = Vec::new();
        let mut tracks_to_remove: Vec<u32> = Vec::new();
        // Go through all the tracks and identify the ones that should be starting their next note
        // or should be expiring, and advance their clocks
        for (id_ref, progress) in self.tracks.iter_mut() {
            let id = *id_ref;
            if progress.current_item_time_remaining > 0 {
                progress.current_item_time_remaining -= 1;
            } else {
                if progress.current_item_index.is_none() {
                    progress.current_item_index = Some(0);
                } else {
                    progress.current_item_index = Some(progress.current_item_index.unwrap() + 1);
                }
                let i = progress.current_item_index.unwrap();
                if progress.current_item_index.unwrap() >= progress.track.items.len() {
                    tracks_to_remove.push(id);
                } else {
                    let item = &progress.track.items[i];
                    match item {
                        TrackItem::Note(music_note) => {
                            match music_note.channel_mode {
                                ChannelMode::Pulse(_) => &mut pulse_tracks_to_play,
                                ChannelMode::Triangle => &mut triangle_tracks_to_play,
                                ChannelMode::Noise => &mut noise_tracks_to_play,
                            }
                            .push(id);
                            progress.current_item_time_remaining = music_note.duration_attack
                                as u32
                                + music_note.duration_decay as u32
                                + music_note.duration_release as u32
                                + music_note.duration_sustain as u32;
                        }
                        TrackItem::Rest(rest) => {
                            progress.current_item_time_remaining = rest.duration as u32;
                        }
                    }
                }
            }
        }
        // Remove all the tracks that have ended
        tracks_to_remove.iter().for_each(|id| {
            self.tracks.remove(id);
        });
        // Determine whether any new tracks should play now
        // TODO: Implement track priority. For now, overwrite existing tracks with imputiny
        for track_id in pulse_tracks_to_play {
            let track = self.tracks.get(&track_id).unwrap();
            let item = track.current_item().unwrap();
            if let TrackItem::Note(music_note) = item {
                self.channel_pulse_1_id = track_id;
                music_note.play(0);
            }
        }
        for track_id in noise_tracks_to_play {
            let track = self.tracks.get(&track_id).unwrap();
            let item = track.current_item().unwrap();
            if let TrackItem::Note(music_note) = item {
                self.channel_noise_id = track_id;
                music_note.play(0);
            }
        }
        for track_id in triangle_tracks_to_play {
            let track = self.tracks.get(&track_id).unwrap();
            let item = track.current_item().unwrap();
            if let TrackItem::Note(music_note) = item {
                self.channel_triangle_id = track_id;
                music_note.play(0);
            }
        }
    }

    pub fn play<'b: 'a>(&mut self, track: &'b Track<'a>) -> u32 {
        self.play_after_delay(track, 0)
    }

    pub fn play_after_delay<'b: 'a>(&mut self, track: &'b Track<'a>, delay: u32) -> u32 {
        let play_id = self.id_seed;
        self.id_seed += 1;
        self.tracks.insert(
            play_id,
            TrackProgress {
                track: track.clone(),
                current_item_index: None,
                current_item_time_remaining: delay,
            },
        );
        play_id
    }

    pub fn stop(&mut self, play_id: u32) {
        todo!("Find the track in the list and stop it")
    }
}
