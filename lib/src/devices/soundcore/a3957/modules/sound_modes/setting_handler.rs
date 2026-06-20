use async_trait::async_trait;
use openscq30_lib_has::Has;
use strum::IntoEnumIterator;

use crate::{
    api::settings::{self, Setting, SettingId, Value},
    devices::soundcore::{
        a3957::structures::{ManualNoiseCanceling, NoiseCancelingMode, SoundModes},
        common::settings_manager::{SettingHandler, SettingHandlerError, SettingHandlerResult},
    },
    i18n::fl,
};

use super::SoundModeSetting;

#[derive(Default)]
pub struct SoundModesSettingHandler;
pub struct D1204SoundModesSettingHandler;

#[async_trait]
impl<T> SettingHandler<T> for SoundModesSettingHandler
where
    T: Has<SoundModes> + Send,
{
    fn settings(&self) -> Vec<SettingId> {
        SoundModeSetting::iter().map(Into::into).collect()
    }

    fn get(&self, state: &T, setting_id: &SettingId) -> Option<Setting> {
        let sound_modes: &SoundModes = state.get();
        let sound_mode_setting: SoundModeSetting = (*setting_id).try_into().ok()?;
        match sound_mode_setting {
            SoundModeSetting::AmbientSoundMode => Some(Setting::select_from_enum_all_variants(
                sound_modes.ambient_sound_mode,
            )),
            SoundModeSetting::TransparencyMode => Some(Setting::select_from_enum_all_variants(
                sound_modes.transparency_mode,
            )),
            SoundModeSetting::NoiseCancelingMode => Some(Setting::select_from_enum_all_variants(
                sound_modes.noise_canceling_mode,
            )),
            SoundModeSetting::ManualNoiseCanceling => Some(Setting::I32Range {
                setting: settings::Range {
                    range: 1..=5,
                    step: 1,
                },
                value: sound_modes.manual_noise_canceling.inner().into(),
            }),
            SoundModeSetting::WindNoiseSuppression => Some(Setting::Toggle {
                value: sound_modes.wind_noise.is_suppression_enabled,
            }),
            SoundModeSetting::WindNoiseDetected => Some(Setting::Information {
                value: sound_modes.wind_noise.is_detected.to_string(),
                translated_value: if sound_modes.wind_noise.is_detected {
                    fl!("yes")
                } else {
                    fl!("no")
                },
            }),
            SoundModeSetting::TransportationMode => Some(Setting::select_from_enum_all_variants(
                sound_modes.transportation_mode,
            )),
        }
    }

    async fn set(
        &self,
        state: &mut T,
        setting_id: &SettingId,
        value: Value,
    ) -> SettingHandlerResult<()> {
        let sound_mode_setting: SoundModeSetting = (*setting_id)
            .try_into()
            .expect("already filtered to valid values only by SettingsManager");
        match sound_mode_setting {
            SoundModeSetting::AmbientSoundMode => {
                let sound_modes: &mut SoundModes = state.get_mut();
                sound_modes.ambient_sound_mode = value.try_as_enum_variant()?;
            }
            SoundModeSetting::TransparencyMode => {
                let sound_modes: &mut SoundModes = state.get_mut();
                sound_modes.transparency_mode = value.try_as_enum_variant()?;
            }
            SoundModeSetting::NoiseCancelingMode => {
                let sound_modes: &mut SoundModes = state.get_mut();
                sound_modes.noise_canceling_mode = value.try_as_enum_variant()?;
            }
            SoundModeSetting::ManualNoiseCanceling => {
                let sound_modes: &mut SoundModes = state.get_mut();
                sound_modes.manual_noise_canceling =
                    ManualNoiseCanceling::new(value.try_as_i32()? as u8);
            }
            SoundModeSetting::WindNoiseSuppression => {
                let sound_modes: &mut SoundModes = state.get_mut();
                sound_modes.wind_noise.is_suppression_enabled = value.try_as_bool()?;
            }
            SoundModeSetting::WindNoiseDetected => return Err(SettingHandlerError::ReadOnly),
            SoundModeSetting::TransportationMode => {
                let sound_modes: &mut SoundModes = state.get_mut();
                sound_modes.transportation_mode = value.try_as_enum_variant()?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl<T> SettingHandler<T> for D1204SoundModesSettingHandler
where
    T: Has<SoundModes> + Send,
{
    fn settings(&self) -> Vec<SettingId> {
        vec![
            SettingId::AmbientSoundMode,
            SettingId::TransparencyMode,
            SettingId::NoiseCancelingMode,
            SettingId::ManualNoiseCanceling,
        ]
    }

    fn get(&self, state: &T, setting_id: &SettingId) -> Option<Setting> {
        let sound_modes: &SoundModes = state.get();
        match setting_id {
            SettingId::AmbientSoundMode => Some(Setting::select_from_enum_all_variants(
                sound_modes.ambient_sound_mode,
            )),
            SettingId::TransparencyMode => Some(Setting::select_from_enum_all_variants(
                sound_modes.transparency_mode,
            )),
            // The D1204 supports adaptive and manual noise canceling, but not the
            // A3957 transportation mode, so only offer the two it accepts.
            SettingId::NoiseCancelingMode => Some(Setting::select_from_enum(
                &[NoiseCancelingMode::Manual, NoiseCancelingMode::Adaptive],
                sound_modes.noise_canceling_mode,
            )),
            SettingId::ManualNoiseCanceling => Some(Setting::I32Range {
                setting: settings::Range {
                    range: 1..=5,
                    step: 1,
                },
                value: sound_modes.manual_noise_canceling.inner().into(),
            }),
            _ => None,
        }
    }

    async fn set(
        &self,
        state: &mut T,
        setting_id: &SettingId,
        value: Value,
    ) -> SettingHandlerResult<()> {
        match setting_id {
            SettingId::AmbientSoundMode => {
                let sound_modes: &mut SoundModes = state.get_mut();
                sound_modes.ambient_sound_mode = value.try_as_enum_variant()?;
            }
            SettingId::TransparencyMode => {
                let sound_modes: &mut SoundModes = state.get_mut();
                sound_modes.transparency_mode = value.try_as_enum_variant()?;
            }
            SettingId::NoiseCancelingMode => {
                let sound_modes: &mut SoundModes = state.get_mut();
                sound_modes.noise_canceling_mode = value.try_as_enum_variant()?;
            }
            SettingId::ManualNoiseCanceling => {
                let sound_modes: &mut SoundModes = state.get_mut();
                sound_modes.manual_noise_canceling =
                    ManualNoiseCanceling::new(value.try_as_i32()? as u8);
            }
            _ => return Err(SettingHandlerError::MissingData),
        }
        Ok(())
    }
}
