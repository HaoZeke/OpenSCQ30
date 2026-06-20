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
        let dual_connections_devices = if state_update_packet.dual_connections_enabled
            || state_update_packet.supports_dual_connections_device_list
        {
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
        let is_d1204 = builder.device_model() == crate::devices::DeviceModel::SoundcoreD1204;

        builder.module_collection().add_state_update();
        if is_d1204 {
            builder.d1204_sound_modes();
            builder.d1204_equalizer();
        } else {
            builder.a3957_sound_modes();
            builder
                .equalizer_with_custom_hear_id_tws(equalizer::common_settings())
                .await;
        }
        if !is_d1204 {
            builder.button_configuration(&BUTTON_CONFIGURATION_SETTINGS);
            builder.ambient_sound_mode_cycle();
            builder.reset_button_configuration::<packets::inbound::A3957StateUpdatePacket>(
                RequestState.to_packet(),
            );
        }

        builder.limit_high_volume();

        if !is_d1204 {
            builder.dual_connections();
            builder.ldac();
        } else {
            builder.dual_connections_devices_read_only();
        }

        builder.auto_power_off(
            common::modules::auto_power_off::AutoPowerOffDuration::ten_twenty_thirty_sixty(),
        );
        if !is_d1204 {
            builder.touch_tone();
            builder.low_battery_prompt();
            builder.wearing_tone();
            builder.wearing_detection();
            builder.sound_leak_compensation();
            builder.gaming_mode();
        }

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

    use macaddr::MacAddr6;

    use crate::{
        DeviceModel,
        devices::soundcore::common::{
            device::{SoundcoreDeviceConfig, test_utils::TestSoundcoreDevice},
            packet::{self, outbound::ToPacket},
            structures::DualConnectionsDevice,
        },
        settings::{Setting, SettingId, Value},
    };

    fn minimal_d1204_state_body() -> Vec<u8> {
        let mut body = Vec::new();
        let mut push_field = |tag: u8, value: &[u8]| {
            body.push(tag);
            body.push(value.len().try_into().unwrap());
            body.extend_from_slice(value);
        };

        push_field(3, &[0, 98]);
        push_field(4, &[0, 96]);
        push_field(5, b"05.40");
        push_field(6, b"05.40");
        push_field(7, b"1204000000000000\0");
        push_field(8, &[0, 100]);
        body
    }

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
            .expect("D1204 should expose the writable noise-canceling mode");
        assert!(noise_canceling_mode.mode().is_writable());
        // tag36[0] == 2 (Normal) parses the noise-canceling sub-mode as Manual.
        assert_eq!(Value::from(noise_canceling_mode), Value::from("Manual"));
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
            (SettingId::NoiseCancelingMode, "Manual".into()),
            (SettingId::ManualNoiseCanceling, 1.into()),
            (SettingId::AutoPowerOff, "30m".into()),
            (SettingId::LimitHighVolume, false.into()),
            (SettingId::LimitHighVolumeDbLimit, 90.into()),
            (SettingId::LimitHighVolumeRefreshRate, "RealTime".into()),
        ]);
    }

    #[tokio::test(start_paused = true)]
    async fn test_d1204_hides_unmapped_a3957_controls() {
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
                        38, 2, 0, 0, 39, 3, 0, 90, 0, 42, 1, 1, 48, 1, 2, 49, 1, 0, 50, 2, 0, 1,
                        52, 1, 0, 53, 1, 255, 54, 2, 1, 1, 68, 1, 0,
                    ],
                ),
            )]),
            SoundcoreDeviceConfig::default(),
        )
        .await;

        for setting_id in [
            SettingId::Ldac,
            SettingId::DualConnections,
            SettingId::LeftSinglePress,
            SettingId::RightSinglePress,
            SettingId::LeftDoublePress,
            SettingId::RightDoublePress,
            SettingId::LeftTriplePress,
            SettingId::RightTriplePress,
            SettingId::LeftLongPress,
            SettingId::RightLongPress,
            SettingId::NormalModeInCycle,
            SettingId::TransparencyModeInCycle,
            SettingId::NoiseCancelingModeInCycle,
            SettingId::ResetButtonsToDefault,
            SettingId::TouchTone,
            SettingId::LowBatteryPrompt,
            SettingId::WearingTone,
            SettingId::WearingDetection,
            SettingId::SoundLeakCompensation,
            SettingId::GamingMode,
            SettingId::TransportationMode,
            SettingId::WindNoiseSuppression,
            SettingId::WindNoiseDetected,
            SettingId::CustomEqualizerProfile,
            SettingId::VolumeAdjustments,
            SettingId::ImportCustomEqualizerProfiles,
            SettingId::ExportCustomEqualizerProfiles,
            SettingId::ExportCustomEqualizerProfilesOutput,
        ] {
            assert!(
                device.inner().setting(&setting_id).is_none(),
                "{setting_id} should not be exposed for D1204 without packet evidence"
            );
        }

        device.assert_setting_values([
            (SettingId::AmbientSoundMode, "Normal".into()),
            (SettingId::TransparencyMode, "VocalMode".into()),
            (SettingId::NoiseCancelingMode, "Manual".into()),
            (SettingId::AutoPowerOff, "30m".into()),
            (SettingId::LimitHighVolume, false.into()),
            (SettingId::LimitHighVolumeDbLimit, 90.into()),
            (SettingId::LimitHighVolumeRefreshRate, "RealTime".into()),
        ]);
    }

    #[tokio::test(start_paused = true)]
    async fn test_d1204_sound_mode_write_surface_is_limited_to_verified_fields() {
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
                        48, 48, 48, 48, 0, 8, 2, 0, 100, 36, 3, 2, 0, 1, 37, 2, 5, 1,
                    ],
                ),
            )]),
            SoundcoreDeviceConfig::default(),
        )
        .await;

        for setting_id in [SettingId::AmbientSoundMode, SettingId::TransparencyMode] {
            let setting = device
                .inner()
                .setting(&setting_id)
                .expect("verified D1204 sound mode setting should be exposed");
            assert!(
                setting.mode().is_writable(),
                "{setting_id} should remain writable on D1204"
            );
        }

        // The D1204 noise-canceling mode and manual level are writable: the
        // official app's [06,81] body and the live device confirm both.
        let noise_canceling_mode = device
            .inner()
            .setting(&SettingId::NoiseCancelingMode)
            .expect("D1204 should expose the writable noise-canceling mode");
        assert!(noise_canceling_mode.mode().is_writable());
        // tag36[0] == 2 (Normal) parses the noise-canceling sub-mode as Manual.
        assert_eq!(Value::from(noise_canceling_mode), Value::from("Manual"));

        let manual = device
            .inner()
            .setting(&SettingId::ManualNoiseCanceling)
            .expect("D1204 should expose the writable manual noise-canceling level");
        assert!(manual.mode().is_writable());
        assert_eq!(Value::from(manual), Value::from(5));

        for setting_id in [
            SettingId::TransportationMode,
            SettingId::WindNoiseSuppression,
            SettingId::WindNoiseDetected,
        ] {
            assert!(
                device.inner().setting(&setting_id).is_none(),
                "{setting_id} should stay hidden on D1204 until packet writes are proven"
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn test_d1204_exposes_dual_connections_devices_read_only() {
        let device = TestSoundcoreDevice::new(
            super::device_registry,
            DeviceModel::SoundcoreD1204,
            HashMap::from([
                (
                    packet::Command([1, 1]),
                    packet::Inbound::new(packet::Command([1, 1]), minimal_d1204_state_body()),
                ),
                (
                    packet::Command([0x0b, 0x01]),
                    packet::inbound::DualConnectionsDevicePacket {
                        total_packets: 1,
                        current_packet_index: 1,
                        devices: vec![
                            DualConnectionsDevice {
                                is_connected: true,
                                mac_address: MacAddr6::new(0, 0, 0, 0, 0, 1),
                                name: "Laptop".to_owned(),
                            },
                            DualConnectionsDevice {
                                is_connected: false,
                                mac_address: MacAddr6::new(0, 0, 0, 0, 0, 2),
                                name: "Phone".to_owned(),
                            },
                        ],
                    }
                    .to_packet(),
                ),
            ]),
            SoundcoreDeviceConfig::default(),
        )
        .await;

        assert!(
            device
                .inner()
                .setting(&SettingId::DualConnections)
                .is_none(),
            "D1204 should not expose the writable dual-connections toggle without write-packet evidence"
        );

        let setting = device
            .inner()
            .setting(&SettingId::DualConnectionsDevices)
            .expect("D1204 should expose paired dual-connection devices");
        assert!(!setting.mode().is_writable());
        assert!(setting.mode().is_readable());

        let Setting::Information { value, .. } = setting else {
            panic!("D1204 dual-connection devices should be read-only information");
        };
        assert_eq!(value, "Laptop (connected), Phone (disconnected)");
    }

    #[tokio::test(start_paused = true)]
    async fn test_d1204_reads_adaptive_noise_canceling_from_mode_byte() {
        // tag36[0] == 3 is NoiseCanceling/Adaptive (verified against the live
        // device). tag37[0] carries the manual level even while adaptive.
        let mut body = minimal_d1204_state_body();
        body.extend_from_slice(&[36, 3, 3, 0, 0, 37, 2, 2, 1]);

        let device = TestSoundcoreDevice::new(
            super::device_registry,
            DeviceModel::SoundcoreD1204,
            HashMap::from([(
                packet::Command([1, 1]),
                packet::Inbound::new(packet::Command([1, 1]), body),
            )]),
            SoundcoreDeviceConfig::default(),
        )
        .await;

        device.assert_setting_values([
            (SettingId::AmbientSoundMode, "NoiseCanceling".into()),
            (SettingId::NoiseCancelingMode, "Adaptive".into()),
            (SettingId::ManualNoiseCanceling, 2.into()),
        ]);

        let noise_canceling_mode = device
            .inner()
            .setting(&SettingId::NoiseCancelingMode)
            .unwrap();
        assert!(noise_canceling_mode.mode().is_writable());
    }
}
