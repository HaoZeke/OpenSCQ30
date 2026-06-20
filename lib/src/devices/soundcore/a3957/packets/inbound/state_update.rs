use std::{collections::HashMap, iter};

use async_trait::async_trait;
use nom::{
    IResult, Parser,
    bytes::complete::take,
    combinator::map,
    error::{ContextError, ErrorKind, ParseError, context},
};
use tokio::sync::watch;

use crate::{
    api::device,
    devices::soundcore::{
        a3957,
        common::{
            self,
            modules::ModuleCollection,
            packet::{
                self, Command,
                inbound::{FromPacketBody, TryToPacket},
                outbound::ToPacket,
                parsing::take_bool,
            },
            packet_manager::PacketHandler,
            state::Update,
            structures::{Ldac, button_configuration::ButtonStatusCollection},
        },
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A3957StateUpdatePacket {
    pub tws_status: common::structures::TwsStatus,
    pub dual_battery: common::structures::DualBattery,
    pub dual_firmware_version: common::structures::DualFirmwareVersion,
    pub serial_number: common::structures::SerialNumber,
    // 5 bytes ("00.00")
    pub case_battery: common::structures::CaseBatteryLevel,
    pub equalizer_configuration: common::structures::CommonEqualizerConfiguration<2, 10>,
    pub age_range: common::structures::AgeRange,
    pub gender: common::structures::Gender,
    pub hear_id: common::structures::CustomHearId<2, 10>,
    pub button_configuration: ButtonStatusCollection<8>,
    pub ambient_sound_mode_cycle: common::structures::AmbientSoundModeCycle,
    pub sound_modes: a3957::structures::SoundModes,
    pub wearing_tone: common::structures::WearingTone,
    pub low_battery_prompt: common::structures::LowBatteryPrompt,
    pub ldac: Ldac,
    pub dual_connections_enabled: bool,
    pub supports_dual_connections_device_list: bool,
    pub auto_power_off: common::structures::AutoPowerOff,
    pub limit_high_volume: common::structures::LimitHighVolume,
    pub immersive_experience: a3957::structures::ImmersiveExperience,
    pub sound_leak_compensation: common::structures::SoundLeakCompensation,
    pub wearing_detection: common::structures::WearingDetection,
    pub touch_tone: common::structures::TouchTone,
    pub gaming_mode: common::structures::GamingMode,
    pub pressure_sensitivity: a3957::structures::PressureSensitivity,
}

impl Default for A3957StateUpdatePacket {
    fn default() -> Self {
        Self {
            tws_status: Default::default(),
            dual_battery: Default::default(),
            dual_firmware_version: Default::default(),
            serial_number: Default::default(),
            case_battery: Default::default(),
            equalizer_configuration: Default::default(),
            age_range: Default::default(),
            hear_id: Default::default(),
            button_configuration: a3957::BUTTON_CONFIGURATION_SETTINGS.default_status_collection(),
            ambient_sound_mode_cycle: Default::default(),
            sound_modes: Default::default(),
            wearing_tone: Default::default(),
            low_battery_prompt: Default::default(),
            ldac: Default::default(),
            dual_connections_enabled: Default::default(),
            supports_dual_connections_device_list: Default::default(),
            auto_power_off: Default::default(),
            limit_high_volume: Default::default(),
            immersive_experience: Default::default(),
            sound_leak_compensation: Default::default(),
            wearing_detection: Default::default(),
            touch_tone: Default::default(),
            gaming_mode: Default::default(),
            pressure_sensitivity: Default::default(),
            gender: Default::default(),
        }
    }
}

impl FromPacketBody for A3957StateUpdatePacket {
    type DirectionMarker = packet::InboundMarker;

    fn take<'a, E: ParseError<&'a [u8]> + ContextError<&'a [u8]>>(
        input: &'a [u8],
    ) -> IResult<&'a [u8], Self, E> {
        Self::take_flat(input).or_else(|_: nom::Err<E>| Self::take_d1204_tlv(input))
    }
}

impl A3957StateUpdatePacket {
    fn take_flat<'a, E: ParseError<&'a [u8]> + ContextError<&'a [u8]>>(
        input: &'a [u8],
    ) -> IResult<&'a [u8], Self, E> {
        context(
            "a3957 state update packet",
            map(
                (
                    (
                        common::structures::TwsStatus::take,
                        common::structures::DualBattery::take,
                        common::structures::DualFirmwareVersion::take,
                        common::structures::SerialNumber::take,
                        take(5usize), // "00.00"
                        common::structures::CaseBatteryLevel::take,
                        common::structures::EqualizerConfiguration::take,
                        common::structures::AgeRange::take,
                        common::structures::CustomHearId::take_with_music_genre_at_end,
                        take(1usize), // unknown (value 10)
                        ButtonStatusCollection::take(
                            a3957::BUTTON_CONFIGURATION_SETTINGS.parse_settings(),
                        ),
                        common::structures::AmbientSoundModeCycle::take,
                        a3957::structures::SoundModes::take, // 7 bytes: 119-125
                        take(1usize),                        // unknown: 126 (always 0x33)
                        common::structures::WearingTone::take, // 127
                        common::structures::LowBatteryPrompt::take, // 128
                        Ldac::take,                          // 129: Ldac
                        take_bool,                           // 130: dual connections enabled
                        common::structures::AutoPowerOff::take, // 131-132
                        common::structures::LimitHighVolume::take, // 133-135
                        a3957::structures::ImmersiveExperience::take, // 136
                    ),
                    (
                        take(6usize),                                    // unknown: 137-142
                        common::structures::SoundLeakCompensation::take, // 143
                        common::structures::WearingDetection::take,      // 144
                        common::structures::TouchTone::take,             // 145
                        common::structures::GamingMode::take,            // 146
                        a3957::structures::PressureSensitivity::take,    // 147
                        take(6usize),                                    // unknown: 148-153
                    ),
                ),
                |(
                    (
                        tws_status,
                        dual_battery,
                        dual_firmware_version,
                        serial_number,
                        _unknown0,
                        case_battery,
                        equalizer_configuration,
                        age_range,
                        hear_id,
                        _unknown1,
                        button_configuration,
                        ambient_sound_mode_cycle,
                        sound_modes,
                        _unknown2,
                        wearing_tone,
                        low_battery_prompt,
                        ldac,
                        dual_connections_enabled,
                        auto_power_off,
                        limit_high_volume,
                        immersive_experience,
                    ),
                    (
                        _unknown4,
                        sound_leak_compensation,
                        wearing_detection,
                        touch_tone,
                        gaming_mode,
                        pressure_sensitivity,
                        _unknown5,
                    ),
                )| {
                    Self {
                        tws_status,
                        dual_battery,
                        dual_firmware_version,
                        serial_number,
                        equalizer_configuration,
                        age_range,
                        hear_id,
                        case_battery,
                        button_configuration,
                        ambient_sound_mode_cycle,
                        sound_modes,
                        wearing_tone,
                        low_battery_prompt,
                        ldac,
                        dual_connections_enabled,
                        supports_dual_connections_device_list: false,
                        auto_power_off,
                        limit_high_volume,
                        immersive_experience,
                        sound_leak_compensation,
                        wearing_detection,
                        touch_tone,
                        gaming_mode,
                        pressure_sensitivity,
                        // not parsed from packet
                        gender: Default::default(),
                    }
                },
            ),
        )
        .parse_complete(input)
    }

    fn take_d1204_tlv<'a, E: ParseError<&'a [u8]> + ContextError<&'a [u8]>>(
        input: &'a [u8],
    ) -> IResult<&'a [u8], Self, E> {
        let (_, fields) = d1204_tlv_fields::<E>(input)?;
        let left_battery = d1204_battery_level(d1204_field::<E>(&fields, 3, input)?);
        let right_battery = d1204_battery_level(d1204_field::<E>(&fields, 4, input)?);
        let left_firmware =
            d1204_firmware_version::<E>(d1204_field::<E>(&fields, 5, input)?, input)?;
        let right_firmware =
            d1204_firmware_version::<E>(d1204_field::<E>(&fields, 6, input)?, input)?;
        let serial_number = d1204_serial_number::<E>(d1204_field::<E>(&fields, 7, input)?, input)?;
        let case_battery = d1204_battery_level(d1204_field::<E>(&fields, 8, input)?);

        Ok((
            &[],
            Self {
                tws_status: common::structures::TwsStatus {
                    host_device: d1204_host_device(fields.get(&1).copied()),
                    is_connected: d1204_bool(fields.get(&2).copied()),
                },
                dual_battery: common::structures::DualBattery {
                    left: common::structures::SingleBattery {
                        level: left_battery,
                        is_charging: Default::default(),
                    },
                    right: common::structures::SingleBattery {
                        level: right_battery,
                        is_charging: Default::default(),
                    },
                },
                dual_firmware_version: common::structures::DualFirmwareVersion::Both {
                    left: left_firmware,
                    right: right_firmware,
                },
                serial_number,
                case_battery: common::structures::CaseBatteryLevel(case_battery),
                equalizer_configuration: Default::default(),
                age_range: Default::default(),
                gender: Default::default(),
                hear_id: Default::default(),
                button_configuration: a3957::BUTTON_CONFIGURATION_SETTINGS
                    .default_status_collection(),
                ambient_sound_mode_cycle: Default::default(),
                sound_modes: d1204_sound_modes(&fields),
                wearing_tone: Default::default(),
                low_battery_prompt: Default::default(),
                ldac: Default::default(),
                dual_connections_enabled: false,
                supports_dual_connections_device_list: true,
                auto_power_off: d1204_auto_power_off(&fields),
                limit_high_volume: d1204_limit_high_volume(&fields),
                immersive_experience: Default::default(),
                sound_leak_compensation: Default::default(),
                wearing_detection: Default::default(),
                touch_tone: Default::default(),
                gaming_mode: Default::default(),
                pressure_sensitivity: Default::default(),
            },
        ))
    }
}

fn d1204_tlv_fields<'a, E: ParseError<&'a [u8]>>(
    input: &'a [u8],
) -> IResult<&'a [u8], HashMap<u8, &'a [u8]>, E> {
    let mut rest = input;
    let mut fields = HashMap::new();
    while !rest.is_empty() {
        if rest.len() < 2 {
            return Err(nom::Err::Error(E::from_error_kind(input, ErrorKind::Eof)));
        }
        let tag = rest[0];
        let len = usize::from(rest[1]);
        rest = &rest[2..];
        if rest.len() < len {
            return Err(nom::Err::Error(E::from_error_kind(input, ErrorKind::Eof)));
        }
        let value = &rest[..len];
        fields.insert(tag, value);
        rest = &rest[len..];
    }
    Ok((rest, fields))
}

fn d1204_field<'a, E: ParseError<&'a [u8]>>(
    fields: &HashMap<u8, &'a [u8]>,
    tag: u8,
    input: &'a [u8],
) -> Result<&'a [u8], nom::Err<E>> {
    fields
        .get(&tag)
        .copied()
        .ok_or_else(|| nom::Err::Error(E::from_error_kind(input, ErrorKind::Tag)))
}

fn d1204_battery_level(input: &[u8]) -> common::structures::BatteryLevel {
    let percent = input.last().copied().unwrap_or_default();
    common::structures::BatteryLevel(percent.saturating_sub(1).saturating_div(10).min(9))
}

fn d1204_bool(input: Option<&[u8]>) -> bool {
    input
        .and_then(|bytes| bytes.last())
        .copied()
        .unwrap_or_default()
        != 0
}

fn d1204_host_device(input: Option<&[u8]>) -> common::structures::HostDevice {
    match input.and_then(|bytes| bytes.last()).copied() {
        Some(1) => common::structures::HostDevice::Right,
        _ => common::structures::HostDevice::Left,
    }
}

fn d1204_firmware_version<'a, E: ParseError<&'a [u8]>>(
    input: &[u8],
    error_input: &'a [u8],
) -> Result<common::structures::FirmwareVersion, nom::Err<E>> {
    if input.len() != 5 || input[2] != b'.' {
        return Err(nom::Err::Error(E::from_error_kind(
            error_input,
            ErrorKind::Tag,
        )));
    }
    let major = d1204_two_digit_number::<E>(&input[0..2], error_input)?;
    let minor = d1204_two_digit_number::<E>(&input[3..5], error_input)?;
    Ok(common::structures::FirmwareVersion::new(major, minor))
}

fn d1204_two_digit_number<'a, E: ParseError<&'a [u8]>>(
    input: &[u8],
    error_input: &'a [u8],
) -> Result<u8, nom::Err<E>> {
    if input.len() != 2 || !input.iter().all(u8::is_ascii_digit) {
        return Err(nom::Err::Error(E::from_error_kind(
            error_input,
            ErrorKind::Digit,
        )));
    }
    Ok((input[0] - b'0') * 10 + input[1] - b'0')
}

fn d1204_serial_number<'a, E: ParseError<&'a [u8]>>(
    input: &[u8],
    error_input: &'a [u8],
) -> Result<common::structures::SerialNumber, nom::Err<E>> {
    let serial_bytes = input.strip_suffix(&[0]).unwrap_or(input);
    let serial = std::str::from_utf8(serial_bytes)
        .map_err(|_| nom::Err::Error(E::from_error_kind(error_input, ErrorKind::Char)))?;
    if serial.is_empty() || !serial.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(nom::Err::Error(E::from_error_kind(
            error_input,
            ErrorKind::AlphaNumeric,
        )));
    }
    Ok(common::structures::SerialNumber::from(serial))
}

fn d1204_sound_modes(fields: &HashMap<u8, &[u8]>) -> a3957::structures::SoundModes {
    let mut sound_modes = a3957::structures::SoundModes::default();
    if let Some(bytes) = fields.get(&36) {
        // D1204 tag36[0] is the combined mode selector:
        //   0 = NoiseCanceling/Manual, 1 = Transparency, 2 = Normal,
        //   3 = NoiseCanceling/Adaptive
        if let Some(&mode_byte) = bytes.first() {
            sound_modes.ambient_sound_mode = match mode_byte {
                1 => common::structures::AmbientSoundMode::Transparency,
                2 => common::structures::AmbientSoundMode::Normal,
                _ => common::structures::AmbientSoundMode::NoiseCanceling,
            };
            sound_modes.noise_canceling_mode = if mode_byte == 3 {
                a3957::structures::NoiseCancelingMode::Adaptive
            } else {
                a3957::structures::NoiseCancelingMode::Manual
            };
        }
        if let Some(transparency_mode) = bytes
            .get(2)
            .and_then(|id| common::structures::TransparencyMode::from_id(*id))
        {
            sound_modes.transparency_mode = transparency_mode;
        }
    }
    if let Some(transparency_mode) = fields
        .get(&38)
        .and_then(|bytes| bytes.first())
        .and_then(|id| common::structures::TransparencyMode::from_id(*id))
    {
        if !fields.get(&36).is_some_and(|bytes| bytes.len() >= 3) {
            sound_modes.transparency_mode = transparency_mode;
        }
    }
    // tag37[0] is the manual noise-canceling level. tag37[1] is not the mode
    // (it stays 1 regardless), so the mode is taken from tag36[0] above.
    if let Some(manual_level) = fields.get(&37).and_then(|bytes| bytes.first()).copied() {
        sound_modes.manual_noise_canceling =
            a3957::structures::ManualNoiseCanceling::new(manual_level);
    }
    sound_modes
}

fn d1204_auto_power_off(fields: &HashMap<u8, &[u8]>) -> common::structures::AutoPowerOff {
    fields
        .get(&25)
        .and_then(|bytes| {
            Some(common::structures::AutoPowerOff {
                is_enabled: bytes.first().copied()? != 0,
                duration: common::structures::AutoPowerOffDurationIndex(*bytes.get(1)?),
            })
        })
        .unwrap_or_default()
}

fn d1204_limit_high_volume(fields: &HashMap<u8, &[u8]>) -> common::structures::LimitHighVolume {
    fields
        .get(&39)
        .and_then(|bytes| {
            Some(common::structures::LimitHighVolume {
                enabled: bytes.first().copied()? != 0,
                db_limit: *bytes.get(1)?,
                refresh_rate: common::structures::DecibelReadingRefreshRate::from_repr(
                    *bytes.get(2)?,
                )
                .unwrap_or_default(),
            })
        })
        .unwrap_or_default()
}

impl ToPacket for A3957StateUpdatePacket {
    type DirectionMarker = packet::InboundMarker;

    fn command(&self) -> Command {
        packet::inbound::STATE_COMMAND
    }

    fn body(&self) -> Vec<u8> {
        self.tws_status
            .bytes()
            .into_iter()
            .chain(self.dual_battery.bytes())
            .chain(self.dual_firmware_version.bytes())
            .chain(self.serial_number.bytes())
            .chain([0; 5]) // unknown: 32-36
            .chain(iter::once(self.case_battery.0.0))
            .chain(self.equalizer_configuration.bytes())
            .chain(iter::once(self.age_range.0))
            .chain(self.hear_id.bytes_with_music_genre_at_end())
            .chain(iter::once(0)) // unknown: 109
            .chain(
                self.button_configuration
                    .bytes(a3957::BUTTON_CONFIGURATION_SETTINGS.parse_settings()),
            )
            .chain(self.ambient_sound_mode_cycle.bytes())
            .chain(self.sound_modes.bytes())
            .chain(iter::once(0x33)) // unknown: 126
            .chain(self.wearing_tone.bytes()) // 127
            .chain(self.low_battery_prompt.bytes()) // 128
            .chain(self.ldac.bytes()) // 129: Ldac
            .chain([self.dual_connections_enabled.into()]) // 130
            .chain(self.auto_power_off.bytes()) // 131-132
            .chain(self.limit_high_volume.bytes()) // 133-135
            .chain(iter::once(self.immersive_experience as u8)) // 136
            .chain([0; 6]) // unknown: 137-142
            .chain(self.sound_leak_compensation.bytes()) // 143
            .chain(self.wearing_detection.bytes()) // 144
            .chain([self.touch_tone.0.into()]) // 145
            .chain(self.gaming_mode.bytes()) // 146
            .chain(iter::once(self.pressure_sensitivity as u8)) // 147
            .chain([0xff; 6]) // unknown: 148-153
            .collect()
    }
}

struct StateUpdatePacketHandler;

#[async_trait]
impl PacketHandler<a3957::state::A3957State> for StateUpdatePacketHandler {
    async fn handle_packet(
        &self,
        state: &watch::Sender<a3957::state::A3957State>,
        packet: &packet::Inbound,
    ) -> device::Result<()> {
        let packet: A3957StateUpdatePacket = packet.try_to_packet()?;
        state.send_modify(|state| state.update(packet));
        Ok(())
    }
}

impl ModuleCollection<a3957::state::A3957State> {
    pub fn add_state_update(&mut self) {
        self.packet_handlers.set_handler(
            packet::inbound::STATE_COMMAND,
            Box::new(StateUpdatePacketHandler {}),
        );
    }
}
