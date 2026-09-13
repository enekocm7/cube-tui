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
    fn from(value: InspectionAudio) -> Self {
        match value {
            InspectionAudio::EightSeconds => EIGHT_SECONDS_FILE,
            InspectionAudio::TwelveSeconds => TWELVE_SECONDS_FILE,
        }
    }
}

static MIXER: OnceLock<MixerDeviceSink> = OnceLock::new();

/// Plays the audio file specified in [`audio_type`]
///
/// The audio file should be under the assets/audio folder
///
/// # Arguments
///
/// * [`audio_type`] - The name of the audio file.
///
/// # Errors
///
/// Returns an error if the audio file cannot be opened or played.
///
/// # Examples
///
/// ```rust
/// play_audio(InspectionAudio::EightSeconds)
/// play_audio(InspectionAudio::TwelveSeconds)
/// ```
pub fn play_audio(audio_type: InspectionAudio) -> anyhow::Result<()> {
    let mixer = MIXER
        .get_or_init(|| {
            DeviceSinkBuilder::open_default_sink().expect("failed to open default sink")
        })
        .mixer();
    let audio_bytes: &[u8] = audio_type.into();
    rodio::play(mixer, Cursor::new(audio_bytes))?;
    Ok(())
}
