mod sound_modes;

use nom::{
    IResult, Parser,
    combinator::map,
    error::{ContextError, ParseError, context},
    number::complete::le_u8,
};
use openscq30_i18n_macros::Translate;
pub use sound_modes::*;
use strum::{EnumIter, EnumString, FromRepr, IntoStaticStr};

#[derive(
    Debug,
    Default,
    Eq,
    PartialEq,
    Clone,
    Copy,
    EnumIter,
    EnumString,
    IntoStaticStr,
    FromRepr,
    Translate,
)]
#[repr(u8)]
pub enum ImmersiveExperience {
    #[default]
    Disabled = 0,
    Enabled = 1,
}

impl ImmersiveExperience {
    pub fn take<'a, E: ParseError<&'a [u8]> + ContextError<&'a [u8]>>(
        input: &'a [u8],
    ) -> IResult<&'a [u8], Self, E> {
        context(
            "immersive experience",
            map(le_u8, |v| Self::from_repr(v).unwrap_or_default()),
        )
        .parse_complete(input)
    }
}

#[derive(
    Debug,
    Default,
    Eq,
    PartialEq,
    Clone,
    Copy,
    EnumIter,
    EnumString,
    IntoStaticStr,
    FromRepr,
    Translate,
)]
#[repr(u8)]
pub enum PressureSensitivity {
    Softest = 0,
    #[default]
    Medium = 1,
    Firmest = 2,
}

impl PressureSensitivity {
    pub fn take<'a, E: ParseError<&'a [u8]> + ContextError<&'a [u8]>>(
        input: &'a [u8],
    ) -> IResult<&'a [u8], Self, E> {
        context(
            "pressure sensitivity",
            map(le_u8, |v| Self::from_repr(v).unwrap_or_default()),
        )
        .parse_complete(input)
    }
}

/// The D1204 (Liberty 5 Pro Max) built-in "Curated Sound Effects" EQ presets.
///
/// Each is applied by replaying the exact =0x03:0x8a= body the official app
/// sends (captured from an Android btsnoop and confirmed audibly on the live
/// device). The body is a parametric definition: a profile id, a band count,
/// then per band =[filter_type, gain_dB f32, freq_Hz f32, Q f32]=.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
pub enum D1204EqualizerProfile {
    SoundcoreSignature,
    ClearVocals,
    PowerfulBass,
    CalmAndSoothing,
}

impl D1204EqualizerProfile {
    pub fn name(&self) -> &'static str {
        match self {
            Self::SoundcoreSignature => "soundcore Signature",
            Self::ClearVocals => "Clear Vocals",
            Self::PowerfulBass => "Powerful Bass",
            Self::CalmAndSoothing => "Calm and Soothing",
        }
    }

    /// The exact `0x03:0x8a` body the official app sends for this preset.
    pub fn write_body(&self) -> Vec<u8> {
        match self {
            Self::SoundcoreSignature => vec![
                0, 0, 0, 2, 106, 53, 57, 66, 3, 1, 0, 0, 64, 64, 0, 0, 240, 65, 154, 153, 153, 62,
                1, 0, 0, 128, 63, 0, 0, 72, 66, 0, 0, 128, 63, 2, 0, 0, 192, 63, 0, 96, 234, 69,
                51, 51, 51, 63,
            ],
            Self::ClearVocals => vec![
                1, 0, 0, 2, 106, 53, 57, 66, 4, 0, 102, 102, 102, 191, 0, 0, 187, 67, 51, 51, 51,
                63, 1, 154, 153, 25, 64, 0, 192, 23, 68, 154, 153, 153, 62, 2, 0, 0, 64, 64, 0,
                128, 219, 69, 51, 51, 51, 63, 1, 0, 0, 160, 64, 0, 0, 72, 69, 51, 51, 51, 63,
            ],
            Self::PowerfulBass => vec![
                2, 0, 0, 2, 106, 53, 57, 66, 4, 1, 0, 0, 192, 64, 0, 0, 160, 66, 0, 0, 0, 63, 1, 0,
                0, 0, 64, 0, 0, 72, 67, 0, 0, 32, 64, 1, 0, 0, 192, 191, 0, 128, 2, 68, 102, 102,
                166, 63, 1, 0, 0, 192, 63, 0, 192, 115, 69, 0, 0, 0, 63,
            ],
            Self::CalmAndSoothing => vec![
                3, 0, 0, 2, 106, 53, 57, 66, 4, 1, 0, 0, 64, 64, 0, 0, 150, 66, 51, 51, 51, 63, 1,
                0, 0, 128, 64, 0, 0, 69, 67, 0, 0, 32, 64, 1, 0, 0, 160, 192, 0, 0, 62, 69, 102,
                102, 166, 63, 2, 0, 0, 128, 63, 0, 0, 175, 69, 51, 51, 51, 63,
            ],
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        use strum::IntoEnumIterator;
        Self::iter().find(|profile| profile.name() == name)
    }
}

/// Selected D1204 EQ preset. `None` means unknown: the active preset is not
/// reported in the device's TLV state, so it is only known after openscq30
/// sets it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct D1204SelectedEqProfile(pub Option<D1204EqualizerProfile>);
