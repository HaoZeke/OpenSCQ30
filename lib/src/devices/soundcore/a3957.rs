use std::collections::HashMap;

use crate::devices::soundcore::common::{
    self,
    macros::soundcore_device,
    modules::{
        button_configuration::{
            ButtonConfigurationSettings, ButtonDisableMode, ButtonSettings, COMMON_ACTIONS,
        },
        equalizer,
    },
    packet::{
        inbound::TryToPacket,
        outbound::{RequestState, ToPacket},
    },
    structures::button_configuration::{
        ActionKind, Button, ButtonParseSettings, ButtonPressKind, EnabledFlagKind,
    },
};

mod modules;
mod packets;
mod state;
mod structures;

soundcore_device!(
    state::A3957State,
    async |packet_io| {
        let state_update_packet: packets::inbound::A3957StateUpdatePacket = packet_io
            .send_with_response(&RequestState.to_packet())
            .await?
            .try_to_packet()?;
        let dual_connections_devices = if state_update_packet.dual_connections_enabled {
            common::modules::dual_connections::take_dual_connection_devices(&packet_io).await?
        } else {
            Vec::new()
        };
        Ok(state::A3957State::new(
            state_update_packet,
            dual_connections_devices,
        ))
    },
    async |builder| {
        builder.module_collection().add_state_update();
        builder.a3957_sound_modes();
        builder
            .equalizer_with_custom_hear_id_tws(equalizer::common_settings())
            .await;
        builder.button_configuration(&BUTTON_CONFIGURATION_SETTINGS);
        builder.ambient_sound_mode_cycle();
        builder.reset_button_configuration::<packets::inbound::A3957StateUpdatePacket>(
            RequestState.to_packet(),
        );

        builder.limit_high_volume();

        builder.dual_connections();

        builder.ldac();
        builder.auto_power_off(
            common::modules::auto_power_off::AutoPowerOffDuration::ten_twenty_thirty_sixty(),
        );
        builder.touch_tone();
        builder.low_battery_prompt();
        builder.wearing_tone();
        builder.wearing_detection();
        builder.sound_leak_compensation();
        builder.gaming_mode();

        builder.tws_status();
        builder.dual_battery_custom(common::modules::dual_battery::DualBatteryConfiguration {
            max_level: 10,
            level_offset: 1,
        });
        builder.case_battery_level_custom(
            common::modules::case_battery_level::CaseBatteryLevelConfiguration {
                max_level: 10,
                level_offset: 1,
            },
        );
        builder.serial_number_and_dual_firmware_version();
    },
    {
        HashMap::from([(
            RequestState::COMMAND,
            packets::inbound::A3957StateUpdatePacket::default().to_packet(),
        )])
    },
);

pub const BUTTON_CONFIGURATION_SETTINGS: ButtonConfigurationSettings<8, 4> =
    ButtonConfigurationSettings {
        supports_set_all_packet: false,
        ignore_enabled_flag: false,
        order: [
            Button::LeftSinglePress,
            Button::RightSinglePress,
            Button::LeftDoublePress,
            Button::RightDoublePress,
            Button::LeftTriplePress,
            Button::RightTriplePress,
            Button::LeftLongPress,
            Button::RightLongPress,
        ],
        settings: [
            ButtonSettings {
                parse_settings: ButtonParseSettings {
                    enabled_flag_kind: EnabledFlagKind::None,
                    action_kind: ActionKind::TwsLowBits,
                },
                button_id: 2,
                press_kind: ButtonPressKind::Single,
                available_actions: COMMON_ACTIONS,
                disable_mode: ButtonDisableMode::IndividualDisable,
            },
            ButtonSettings {
                parse_settings: ButtonParseSettings {
                    enabled_flag_kind: EnabledFlagKind::None,
                    action_kind: ActionKind::TwsLowBits,
                },
                button_id: 0,
                press_kind: ButtonPressKind::Double,
                available_actions: COMMON_ACTIONS,
                disable_mode: ButtonDisableMode::IndividualDisable,
            },
            ButtonSettings {
                parse_settings: ButtonParseSettings {
                    enabled_flag_kind: EnabledFlagKind::None,
                    action_kind: ActionKind::TwsLowBits,
                },
                button_id: 5,
                press_kind: ButtonPressKind::Triple,
                available_actions: COMMON_ACTIONS,
                disable_mode: ButtonDisableMode::IndividualDisable,
            },
            ButtonSettings {
                parse_settings: ButtonParseSettings {
                    enabled_flag_kind: EnabledFlagKind::None,
                    action_kind: ActionKind::TwsLowBits,
                },
                button_id: 1,
                press_kind: ButtonPressKind::Long,
                available_actions: COMMON_ACTIONS,
                disable_mode: ButtonDisableMode::IndividualDisable,
            },
        ],
    };

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        DeviceModel,
        devices::soundcore::common::{
            device::{SoundcoreDeviceConfig, test_utils::TestSoundcoreDevice},
            packet,
        },
        settings::{Setting, SettingId, Value},
    };

    #[tokio::test(start_paused = true)]
    async fn test_with_liberty5_packet_from_issue_226() {
        let device = TestSoundcoreDevice::new(
            super::device_registry,
            DeviceModel::SoundcoreA3957,
            HashMap::from([(
                packet::Command([1, 1]),
                packet::Inbound::new(
                    packet::Command([1, 1]),
                    vec![
                        0x00, 0x01, 0x08, 0x08, 0x00, 0x00, 0x30, 0x33, 0x2e, 0x39, 0x30, 0x30,
                        0x33, 0x2e, 0x39, 0x30, 0x33, 0x39, 0x35, 0x37, 0x46, 0x34, 0x39, 0x44,
                        0x38, 0x41, 0x43, 0x32, 0x30, 0x33, 0x33, 0x34, 0x30, 0x30, 0x2e, 0x30,
                        0x30, 0x02, 0x00, 0x00, 0x78, 0x78, 0x78, 0x78, 0x78, 0x78, 0x78, 0x78,
                        0x78, 0x78, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                        0x00, 0x01, 0x91, 0x82, 0x73, 0x77, 0x8b, 0x93, 0x8f, 0x96, 0x00, 0x00,
                        0x91, 0x82, 0x73, 0x77, 0x8b, 0x93, 0x8f, 0x96, 0x00, 0x00, 0x00, 0x00,
                        0x00, 0x00, 0x00, 0x91, 0x82, 0x73, 0x77, 0x8b, 0x93, 0x8f, 0x96, 0x00,
                        0x00, 0x91, 0x82, 0x73, 0x77, 0x8b, 0x93, 0x8f, 0x96, 0x00, 0x00, 0x00,
                        0x00, 0x0a, 0x66, 0x66, 0x32, 0x33, 0xff, 0xff, 0x44, 0x44, 0x33, 0x00,
                        0x50, 0x00, 0x00, 0x00, 0x00, 0x00, 0x33, 0x01, 0x01, 0x00, 0x01, 0x01,
                        0x02, 0x00, 0x5a, 0x00, 0x00, 0x01, 0x00, 0x00, 0xff, 0x01, 0x00, 0x00,
                        0x01, 0x01, 0x00, 0x01, 0xff, 0xff, 0xff, 0x03, 0xff, 0xff,
                    ],
                ),
            )]),
            SoundcoreDeviceConfig::default(),
        )
        .await;

        device.assert_setting_values([
            (SettingId::FirmwareVersionLeft, "03.90".into()),
            (SettingId::FirmwareVersionRight, "03.90".into()),
            (SettingId::SerialNumber, "3957F49D8AC20334".into()),
            (
                SettingId::PresetEqualizerProfile,
                Some("SoundcoreSignature").into(),
            ),
            (SettingId::BatteryLevelLeft, "9/10".into()),
            (SettingId::BatteryLevelRight, "9/10".into()),
            (SettingId::CaseBatteryLevel, "3/10".into()),
            (SettingId::AmbientSoundMode, "NoiseCanceling".into()),
            (SettingId::NoiseCancelingMode, "Manual".into()),
            (SettingId::ManualNoiseCanceling, 5.into()),
            (SettingId::TransportationMode, "Plane".into()),
            (SettingId::WindNoiseSuppression, false.into()),
            (SettingId::LeftSinglePress, Some("PlayPause").into()),
            (SettingId::LeftDoublePress, Some("PreviousSong").into()),
            (SettingId::LeftTriplePress, Value::OptionalString(None)),
            (SettingId::LeftLongPress, Some("AmbientSoundMode").into()),
            (SettingId::RightSinglePress, Some("PlayPause").into()),
            (SettingId::RightDoublePress, Some("NextSong").into()),
            (SettingId::RightTriplePress, Value::OptionalString(None)),
            (SettingId::RightLongPress, Some("AmbientSoundMode").into()),
            (SettingId::GamingMode, false.into()),
            (SettingId::WearingDetection, true.into()),
            (SettingId::SoundLeakCompensation, false.into()),
            (SettingId::TouchTone, true.into()),
            (SettingId::WearingTone, true.into()),
            (SettingId::LowBatteryPrompt, true.into()),
            (SettingId::LimitHighVolume, false.into()),
            (SettingId::LimitHighVolumeRefreshRate, "RealTime".into()),
            (SettingId::LimitHighVolumeDbLimit, 90.into()),
            (SettingId::DualConnections, true.into()),
            // (SettingId::DolbyAudio, Value::OptionalString(None)),
            // (SettingId::InCallSoundAwareness, true.into()),
            // (SettingId::EarbudPressureSensitivity, "Medium".into()),
        ]);
    }

    #[tokio::test(start_paused = true)]
    async fn test_with_liberty5_pro_max_d1204_packet() {
        let device = TestSoundcoreDevice::new(
            super::device_registry,
            DeviceModel::SoundcoreD1204,
            HashMap::from([(
                packet::Command([1, 1]),
                packet::Inbound::new(
                    packet::Command([1, 1]),
                    vec![
                        1, 1, 1, 2, 1, 1, 3, 2, 0, 98, 4, 2, 0, 96, 5, 5, 48, 53, 46, 52, 48, 6, 5,
                        48, 53, 46, 52, 48, 7, 17, 49, 50, 48, 52, 48, 48, 48, 48, 48, 48, 48, 48,
                        48, 48, 48, 48, 0, 8, 2, 0, 100, 9, 5, 48, 49, 46, 51, 56, 10, 1, 49, 11,
                        2, 0, 0, 12, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 13, 2, 15, 15, 15, 2, 3, 6, 17, 2,
                        15, 15, 19, 2, 4, 4, 21, 2, 0, 0, 23, 2, 1, 1, 14, 2, 15, 15, 16, 2, 6, 6,
                        18, 2, 15, 15, 20, 2, 4, 4, 22, 2, 0, 0, 24, 2, 1, 1, 25, 2, 1, 2, 42, 1,
                        1, 26, 1, 255, 27, 3, 8, 9, 0, 28, 1, 1, 35, 92, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 36, 3, 2, 0, 0, 37, 2, 5, 1, 38, 2, 0, 0, 39, 3, 0,
                        90, 0, 41, 6, 0, 127, 29, 7, 104, 63, 44, 3, 0, 0, 0, 46, 1, 99, 48, 1, 2,
                        49, 1, 0, 50, 2, 0, 1, 51, 6, 0, 3, 106, 53, 57, 66, 52, 1, 0, 53, 1, 255,
                        54, 2, 1, 1, 68, 1, 0,
                    ],
                ),
            )]),
            SoundcoreDeviceConfig::default(),
        )
        .await;

        device.assert_setting_values([
            (SettingId::FirmwareVersionLeft, "05.40".into()),
            (SettingId::FirmwareVersionRight, "05.40".into()),
            (SettingId::SerialNumber, "1204000000000000".into()),
            (SettingId::BatteryLevelLeft, "10/10".into()),
            (SettingId::BatteryLevelRight, "10/10".into()),
            (SettingId::CaseBatteryLevel, "10/10".into()),
        ]);

        let noise_canceling_mode = device
            .inner()
            .setting(&SettingId::NoiseCancelingMode)
            .unwrap();
        let Setting::Select { setting, value } = noise_canceling_mode else {
            panic!("noiseCancelingMode should be a select setting")
        };
        assert_eq!(value, "Adaptive");
        assert!(setting.options.iter().any(|option| option == "Adaptive"));
    }

    #[tokio::test(start_paused = true)]
    async fn test_d1204_tlv_fields_drive_phone_profile_settings() {
        let device = TestSoundcoreDevice::new(
            super::device_registry,
            DeviceModel::SoundcoreD1204,
            HashMap::from([(
                packet::Command([1, 1]),
                packet::Inbound::new(
                    packet::Command([1, 1]),
                    vec![
                        1, 1, 1, 2, 1, 1, 3, 2, 0, 98, 4, 2, 0, 96, 5, 5, 48, 53, 46, 52, 48, 6, 5,
                        48, 53, 46, 52, 48, 7, 17, 49, 50, 48, 52, 48, 48, 48, 48, 48, 48, 48, 48,
                        48, 48, 48, 48, 0, 8, 2, 0, 100, 25, 2, 1, 2, 36, 3, 2, 0, 1, 37, 2, 1, 1,
                        38, 2, 0, 0, 39, 3, 0, 90, 0,
                    ],
                ),
            )]),
            SoundcoreDeviceConfig::default(),
        )
        .await;

        device.assert_setting_values([
            (SettingId::AmbientSoundMode, "Normal".into()),
            (SettingId::TransparencyMode, "VocalMode".into()),
            (SettingId::NoiseCancelingMode, "Adaptive".into()),
            (SettingId::ManualNoiseCanceling, 1.into()),
            (SettingId::AutoPowerOff, "30m".into()),
            (SettingId::LimitHighVolume, false.into()),
            (SettingId::LimitHighVolumeDbLimit, 90.into()),
            (SettingId::LimitHighVolumeRefreshRate, "RealTime".into()),
        ]);
    }
}
