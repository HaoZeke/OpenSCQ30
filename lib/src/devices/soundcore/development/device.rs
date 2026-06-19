use std::{
    panic::Location,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use macaddr::MacAddr6;
use tokio::sync::watch;

use crate::{
    api::{
        connection::{ConnectionDescriptor, ConnectionStatus, RfcommBackend, RfcommConnection},
        device::{self, OpenSCQ30Device, OpenSCQ30DeviceRegistry},
        settings::{CategoryId, Setting, SettingId, Value},
    },
    connection::RfcommServiceSelectionStrategy,
    devices::{
        DeviceModel,
        soundcore::{
            self,
            common::packet::{
                self, Command, PacketIOController,
                outbound::{RequestState, ToPacket},
            },
        },
    },
};

pub struct SoundcoreDevelopmentDeviceRegistry {
    backend: Arc<dyn RfcommBackend + Send + Sync>,
}

impl SoundcoreDevelopmentDeviceRegistry {
    pub fn new(backend: Arc<dyn RfcommBackend + Send + Sync>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl OpenSCQ30DeviceRegistry for SoundcoreDevelopmentDeviceRegistry {
    async fn devices(&self) -> device::Result<Vec<ConnectionDescriptor>> {
        self.backend
            .devices()
            .await
            .map(|it| it.into_iter().collect::<Vec<_>>())
            .map_err(Into::into)
    }

    async fn connect(
        &self,
        mac_address: MacAddr6,
    ) -> device::Result<Arc<dyn OpenSCQ30Device + Send + Sync>> {
        let connection = self
            .backend
            .connect(
                mac_address,
                RfcommServiceSelectionStrategy::Dynamic(|service_uuids| {
                    service_uuids
                        .into_iter()
                        .find(soundcore::is_soundcore_vendor_rfcomm_uuid)
                        .unwrap_or(soundcore::RFCOMM_UUID)
                }),
            )
            .await?;
        let device = SoundcoreDevelopmentDevice::new(connection).await?;
        Ok(Arc::new(device))
    }
}

pub struct SoundcoreDevelopmentDevice {
    packet_io: PacketIOController,
    backend: Arc<dyn RfcommConnection + Send + Sync>,
    state_update_packet: Mutex<Option<packet::Inbound>>,
    changes_signal: watch::Sender<()>,
}

impl SoundcoreDevelopmentDevice {
    async fn new(connection: Arc<dyn RfcommConnection + Send + Sync>) -> device::Result<Self> {
        let (packet_io, _packet_receiver) =
            PacketIOController::new(connection.to_owned(), packet::ChecksumKind::Suffix).await?;
        let state_update_packet = packet_io
            .send_with_response(&RequestState.to_packet())
            .await
            .ok();
        Ok(Self {
            packet_io,
            backend: connection,
            state_update_packet: Mutex::new(state_update_packet),
            changes_signal: watch::channel(()).0,
        })
    }
}

#[async_trait]
impl OpenSCQ30Device for SoundcoreDevelopmentDevice {
    fn connection_status(&self) -> watch::Receiver<ConnectionStatus> {
        self.backend.connection_status()
    }

    fn model(&self) -> DeviceModel {
        DeviceModel::SoundcoreDevelopment
    }

    fn categories(&self) -> Vec<CategoryId> {
        vec![CategoryId::DeviceInformation]
    }

    fn settings_in_category(&self, category_id: &CategoryId) -> Vec<SettingId> {
        if *category_id == CategoryId::DeviceInformation {
            vec![SettingId::StateUpdatePacket, SettingId::SendPacket]
        } else {
            Vec::new()
        }
    }

    fn setting(&self, setting_id: &SettingId) -> Option<Setting> {
        match setting_id {
            SettingId::StateUpdatePacket => {
                let text = format!("{:?}", self.state_update_packet.lock().unwrap());
                Some(Setting::Information {
                    value: text.to_owned(),
                    translated_value: text,
                })
            }
            SettingId::SendPacket => Some(Setting::ImportString {
                confirmation_message: None,
            }),
            _ => None,
        }
    }

    fn watch_for_changes(&self) -> watch::Receiver<()> {
        self.changes_signal.subscribe()
    }

    async fn set_setting_values(
        &self,
        setting_values: Vec<(SettingId, Value)>,
    ) -> device::Result<()> {
        for (setting_id, value) in setting_values {
            if setting_id == SettingId::SendPacket {
                let mut data = value
                    .try_as_str()
                    .map_err(|err| device::Error::Other {
                        source: Box::new(err),
                        location: Location::caller(),
                    })?
                    .split(',')
                    .map(|item| {
                        let item = item.trim_ascii();
                        if let Some(hex_number) = item.strip_prefix("0x") {
                            u8::from_str_radix(hex_number, 16)
                        } else {
                            item.parse::<u8>()
                        }
                    })
                    .collect::<Result<Vec<u8>, _>>()
                    .map_err(|err| device::Error::Other {
                        source: Box::new(err),
                        location: Location::caller(),
                    })?;

                if data.len() < 2 {
                    return Err(device::Error::Other {
                        source: Box::new(DevelopmentDeviceError::MissingCommand),
                        location: Location::caller(),
                    });
                }

                let body = data.split_off(2);
                let command = Command(data.try_into().unwrap());

                let response = self
                    .packet_io
                    .send_with_response(&packet::Outbound::new(command, body))
                    .await?;
                *self.state_update_packet.lock().unwrap() = Some(response);
                let _ = self.changes_signal.send(());
            }
        }

        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
enum DevelopmentDeviceError {
    #[error("data length must be at least 2, since the first 2 bytes are used as the command")]
    MissingCommand,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use macaddr::MacAddr6;
    use tokio::sync::mpsc;

    use crate::{
        api::{
            device::{Error, OpenSCQ30DeviceRegistry},
            settings::{SettingId, Value},
        },
        connection_backend::mock::rfcomm::MockRfcommBackend,
        devices::soundcore::common::packet::{self, ChecksumKind, Command, outbound::RequestState},
    };

    use super::SoundcoreDevelopmentDeviceRegistry;

    #[tokio::test(start_paused = true)]
    async fn send_packet_timeout_returns_error() {
        let (_inbound_sender, inbound_receiver) = mpsc::channel(100);
        let (outbound_sender, _outbound_receiver) = mpsc::channel(100);
        let registry = SoundcoreDevelopmentDeviceRegistry::new(Arc::new(MockRfcommBackend::new(
            inbound_receiver,
            outbound_sender,
        )));
        let device = registry.connect(MacAddr6::nil()).await.unwrap();

        let error = device
            .set_setting_values(vec![(SettingId::SendPacket, Value::from("1,131,1"))])
            .await
            .expect_err("sendPacket should return the packet timeout error");

        assert!(matches!(
            error,
            Error::ActionTimedOut {
                action: "resending packet until ack received"
            }
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn send_packet_updates_displayed_development_response() {
        let (inbound_sender, inbound_receiver) = mpsc::channel(100);
        let (outbound_sender, mut outbound_receiver) = mpsc::channel(100);
        inbound_sender
            .send(
                packet::Inbound::new(RequestState::COMMAND, vec![0x00]).bytes(ChecksumKind::Suffix),
            )
            .await
            .unwrap();
        let registry = SoundcoreDevelopmentDeviceRegistry::new(Arc::new(MockRfcommBackend::new(
            inbound_receiver,
            outbound_sender,
        )));
        let device = registry.connect(MacAddr6::nil()).await.unwrap();
        outbound_receiver.recv().await.unwrap();

        let set_handle = tokio::spawn({
            let device = device.clone();
            async move {
                device
                    .set_setting_values(vec![(
                        SettingId::SendPacket,
                        Value::from("0x20,0x81,0x01"),
                    )])
                    .await
                    .unwrap();
            }
        });
        outbound_receiver.recv().await.unwrap();
        inbound_sender
            .send(
                packet::Inbound::new(Command([0x20, 0x81]), vec![0xab, 0xcd])
                    .bytes(ChecksumKind::Suffix),
            )
            .await
            .unwrap();

        set_handle.await.unwrap();

        let setting = device.setting(&SettingId::StateUpdatePacket).unwrap();
        let text = match setting {
            crate::api::settings::Setting::Information { value, .. } => value,
            _ => panic!("stateUpdatePacket should be information"),
        };
        assert!(text.contains("Command([32, 129])"));
        assert!(text.contains("body: [171, 205]"));
    }
}
