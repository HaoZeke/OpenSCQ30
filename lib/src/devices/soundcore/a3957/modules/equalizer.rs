use std::{borrow::Cow, marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use openscq30_lib_has::Has;
use strum::IntoEnumIterator;
use tokio::sync::watch;

use crate::{
    api::{
        device,
        settings::{self, CategoryId, Setting, SettingId, Value},
    },
    devices::soundcore::{
        a3957::structures::{D1204EqualizerProfile, D1204SelectedEqProfile},
        common::{
            modules::ModuleCollection,
            packet::{self, PacketIOController},
            settings_manager::{SettingHandler, SettingHandlerError, SettingHandlerResult},
            state_modifier::StateModifier,
        },
    },
};

impl<T> ModuleCollection<T>
where
    T: Has<D1204SelectedEqProfile> + Clone + Send + Sync + 'static,
{
    pub fn add_d1204_equalizer(&mut self, packet_io: Arc<PacketIOController>) {
        self.setting_manager
            .add_handler(CategoryId::Equalizer, D1204EqualizerSettingHandler);
        self.state_modifiers
            .push(Box::new(D1204EqualizerStateModifier::<T>::new(packet_io)));
    }
}

struct D1204EqualizerSettingHandler;

#[async_trait]
impl<T> SettingHandler<T> for D1204EqualizerSettingHandler
where
    T: Has<D1204SelectedEqProfile> + Send,
{
    fn settings(&self) -> Vec<SettingId> {
        vec![SettingId::PresetEqualizerProfile]
    }

    fn get(&self, state: &T, setting_id: &SettingId) -> Option<Setting> {
        if *setting_id != SettingId::PresetEqualizerProfile {
            return None;
        }
        let current: &D1204SelectedEqProfile = state.get();
        Some(Setting::OptionalSelect {
            setting: settings::Select {
                options: D1204EqualizerProfile::iter()
                    .map(|profile| Cow::Borrowed(profile.name()))
                    .collect(),
                localized_options: D1204EqualizerProfile::iter()
                    .map(|profile| profile.name().to_owned())
                    .collect(),
            },
            value: current.0.map(|profile| Cow::Borrowed(profile.name())),
        })
    }

    async fn set(
        &self,
        state: &mut T,
        setting_id: &SettingId,
        value: Value,
    ) -> SettingHandlerResult<()> {
        if *setting_id != SettingId::PresetEqualizerProfile {
            return Err(SettingHandlerError::MissingData);
        }
        let name = value.try_as_optional_str()?;
        let profile = match name {
            Some(name) => Some(
                D1204EqualizerProfile::from_name(name).ok_or(SettingHandlerError::MissingData)?,
            ),
            None => None,
        };
        let selected: &mut D1204SelectedEqProfile = state.get_mut();
        selected.0 = profile;
        Ok(())
    }
}

struct D1204EqualizerStateModifier<T> {
    packet_io: Arc<PacketIOController>,
    _state: PhantomData<T>,
}

impl<T> D1204EqualizerStateModifier<T> {
    fn new(packet_io: Arc<PacketIOController>) -> Self {
        Self {
            packet_io,
            _state: PhantomData,
        }
    }
}

#[async_trait]
impl<T> StateModifier<T> for D1204EqualizerStateModifier<T>
where
    T: Has<D1204SelectedEqProfile> + Send + Sync,
{
    async fn move_to_state(
        &self,
        state_sender: &watch::Sender<T>,
        target_state: &T,
    ) -> device::Result<()> {
        let target: D1204SelectedEqProfile = *target_state.get();
        if *state_sender.borrow().get() == target {
            return Ok(());
        }
        if let Some(profile) = target.0 {
            self.packet_io
                .send_with_response(&packet::Outbound::new(
                    packet::Command([0x03, 0x8a]),
                    profile.write_body(),
                ))
                .await?;
        }
        state_sender.send_modify(|state| *state.get_mut() = target);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn d1204_eq_preset_bodies_match_official_app() {
        // byte0 is the preset index; bytes 4..8 are the profile id "j59B".
        assert_eq!(
            D1204EqualizerProfile::SoundcoreSignature.write_body()[0..9],
            [0, 0, 0, 2, 106, 53, 57, 66, 3]
        );
        assert_eq!(D1204EqualizerProfile::ClearVocals.write_body()[0], 1);
        assert_eq!(D1204EqualizerProfile::PowerfulBass.write_body()[0], 2);
        assert_eq!(D1204EqualizerProfile::CalmAndSoothing.write_body()[0], 3);
        // Round-trips through the user-facing name.
        for profile in D1204EqualizerProfile::iter() {
            assert_eq!(
                D1204EqualizerProfile::from_name(profile.name()),
                Some(profile)
            );
        }
    }
}
