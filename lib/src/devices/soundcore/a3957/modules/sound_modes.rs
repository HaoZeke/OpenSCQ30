mod setting_handler;

use std::{marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use openscq30_lib_has::Has;
use setting_handler::{D1204SoundModesSettingHandler, SoundModesSettingHandler};
use strum::{EnumIter, EnumString, IntoStaticStr};
use tokio::sync::watch;

use crate::{
    api::{
        device,
        settings::{CategoryId, SettingId},
    },
    devices::soundcore::{
        a3957::structures::{NoiseCancelingMode, SoundModes},
        common::{
            modules::ModuleCollection,
            packet::{self, PacketIOController},
            state_modifier::StateModifier,
            structures::AmbientSoundMode,
        },
    },
    macros::enum_subset,
};

enum_subset! {
    SettingId,
    #[derive(EnumString, EnumIter, IntoStaticStr)]
    enum SoundModeSetting {
        AmbientSoundMode,
        TransparencyMode,
        NoiseCancelingMode,
        ManualNoiseCanceling,
        TransportationMode,
        WindNoiseSuppression,
        WindNoiseDetected,
    }
}

impl<T> ModuleCollection<T>
where
    T: Has<SoundModes> + Clone + Send + Sync + 'static,
{
    pub fn add_a3957_sound_modes(&mut self, packet_io: Arc<PacketIOController>) {
        self.setting_manager
            .add_handler(CategoryId::SoundModes, SoundModesSettingHandler);
        self.add_partial_sound_modes_v2_with_migration(packet_io);
    }

    pub fn add_d1204_sound_modes(&mut self, packet_io: Arc<PacketIOController>) {
        self.setting_manager
            .add_handler(CategoryId::SoundModes, D1204SoundModesSettingHandler);
        self.state_modifiers
            .push(Box::new(D1204SoundModesStateModifier::<T>::new(packet_io)));
    }
}

/// Encodes the D1204 (Liberty 5 Pro Max) `[06,81]` sound-mode write body.
///
/// The D1204 layout differs from the A3957 flat layout (verified against the
/// official app's btsnoop and live device echoes):
///   byte0 mode: 0 = NoiseCanceling/Manual, 1 = Transparency, 2 = Normal,
///               3 = NoiseCanceling/Adaptive
///   byte2 = transparency sub-mode id (0 = fully transparent, 1 = vocal)
///   byte3 = manual noise-canceling level (1..=5)
///   byte4 = 0x11 constant stamped by the app on every sound-mode write
pub fn d1204_sound_modes_body(sound_modes: &SoundModes) -> Vec<u8> {
    let mode = match sound_modes.ambient_sound_mode {
        AmbientSoundMode::Transparency => 1,
        AmbientSoundMode::Normal => 2,
        AmbientSoundMode::NoiseCanceling => match sound_modes.noise_canceling_mode {
            NoiseCancelingMode::Adaptive => 3,
            _ => 0,
        },
    };
    vec![
        mode,
        0,
        sound_modes.transparency_mode.id(),
        sound_modes.manual_noise_canceling.inner(),
        0x11,
        0,
        0,
    ]
}

struct D1204SoundModesStateModifier<T> {
    packet_io: Arc<PacketIOController>,
    _state: PhantomData<T>,
}

impl<T> D1204SoundModesStateModifier<T> {
    fn new(packet_io: Arc<PacketIOController>) -> Self {
        Self {
            packet_io,
            _state: PhantomData,
        }
    }
}

#[async_trait]
impl<T> StateModifier<T> for D1204SoundModesStateModifier<T>
where
    T: Has<SoundModes> + Send + Sync,
{
    async fn move_to_state(
        &self,
        state_sender: &watch::Sender<T>,
        target_state: &T,
    ) -> device::Result<()> {
        let target: SoundModes = *target_state.get();
        if *state_sender.borrow().get() == target {
            return Ok(());
        }
        self.packet_io
            .send_with_response(&packet::Outbound::new(
                packet::Command([0x06, 0x81]),
                d1204_sound_modes_body(&target),
            ))
            .await?;
        state_sender.send_modify(|state| *state.get_mut() = target);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devices::soundcore::common::structures::TransparencyMode;

    #[test]
    fn d1204_body_matches_official_app() {
        let mut sound_modes = SoundModes {
            ambient_sound_mode: AmbientSoundMode::NoiseCanceling,
            noise_canceling_mode: NoiseCancelingMode::Manual,
            transparency_mode: TransparencyMode::FullyTransparent,
            ..Default::default()
        };
        sound_modes.manual_noise_canceling =
            crate::devices::soundcore::a3957::structures::ManualNoiseCanceling::new(5);
        // Exact bytes captured from the Soundcore app for "NoiseCanceling,
        // manual level 5": [06,81] body = [0, 0, 0, 5, 17, 0, 0].
        assert_eq!(
            d1204_sound_modes_body(&sound_modes),
            vec![0, 0, 0, 5, 0x11, 0, 0]
        );

        sound_modes.noise_canceling_mode = NoiseCancelingMode::Adaptive;
        assert_eq!(d1204_sound_modes_body(&sound_modes)[0], 3);

        sound_modes.ambient_sound_mode = AmbientSoundMode::Normal;
        assert_eq!(d1204_sound_modes_body(&sound_modes)[0], 2);

        sound_modes.ambient_sound_mode = AmbientSoundMode::Transparency;
        sound_modes.transparency_mode = TransparencyMode::VocalMode;
        let body = d1204_sound_modes_body(&sound_modes);
        assert_eq!(body[0], 1);
        assert_eq!(body[2], 1);
    }
}
