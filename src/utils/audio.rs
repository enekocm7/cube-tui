use rodio::{DeviceSinkBuilder, MixerDeviceSink};
use std::io::Cursor;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy)]
/// Audio files types to play during an inspection
pub enum InspectionAudio {
    /// 8-second audio file
    EightSeconds,
    /// 12-second audio file
    TwelveSeconds,
}

const EIGHT_SECONDS_FILE: &[u8] = include_bytes!("../../assets/audio/8-seconds.mp3");
const TWELVE_SECONDS_FILE: &[u8] = include_bytes!("../../assets/audio/12-seconds.mp3");

impl From<InspectionAudio> for &[u8] {
    /// Returns the embedded MP3 bytes for the selected inspection cue.
    fn from(value: InspectionAudio) -> Self {
        match value {
            InspectionAudio::EightSeconds => EIGHT_SECONDS_FILE,
            InspectionAudio::TwelveSeconds => TWELVE_SECONDS_FILE,
        }
    }
}

static MIXER: OnceLock<MixerDeviceSink> = OnceLock::new();

/// Plays the selected embedded inspection cue through the default audio device.
///
/// The output mixer is initialized on the first call and reused for subsequent
/// cues.
///
/// # Errors
///
/// Returns an error if Rodio cannot decode or enqueue the embedded MP3 data.
///
/// # Panics
///
/// Panics if the system's default audio output cannot be opened during mixer
/// initialization.
pub fn play_audio(audio_type: InspectionAudio) -> anyhow::Result<()> {
    let mixer = MIXER
        .get_or_init(|| {
            DeviceSinkBuilder::open_default_sink().expect("failed to open default sink")
        })
        .mixer();
    let audio_bytes: &[u8] = audio_type.into();
    let player = rodio::play(mixer, Cursor::new(audio_bytes))?;
    player.detach();
    Ok(())
}
