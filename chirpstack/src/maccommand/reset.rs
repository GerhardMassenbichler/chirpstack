use anyhow::Result;
use tracing::info;

use crate::storage::{device, device_profile};

const SERV_LORAWAN_VERSION: lrwn::Version = lrwn::Version::LoRaWAN1_1;

pub fn handle(
    dev: &mut device::Device,
    dp: &device_profile::DeviceProfile,
    block: &lrwn::MACCommandSet,
) -> Result<Option<lrwn::MACCommandSet>> {
    let dev_eui = dev.dev_eui;
    let ds = dev.get_device_session_mut()?;

    let block_mac = (**block)
        .first()
        .ok_or_else(|| anyhow!("MACCommandSet is empty"))?;
    let block_pl = if let lrwn::MACCommand::ResetInd(pl) = block_mac {
        pl
    } else {
        return Err(anyhow!("ResetInd expected"));
    };

    info!(dev_eui = %dev_eui, dev_lorawan_version = %block_pl.dev_lorawan_version, serv_lorawan_version = %SERV_LORAWAN_VERSION, "ResetInd received");

    dp.reset_session_to_boot_params(ds);

    Ok(Some(lrwn::MACCommandSet::new(vec![
        lrwn::MACCommand::ResetConf(lrwn::ResetConfPayload {
            serv_lorawan_version: if SERV_LORAWAN_VERSION.to_u8()
                > block_pl.dev_lorawan_version.to_u8()
            {
                block_pl.dev_lorawan_version
            } else {
                SERV_LORAWAN_VERSION
            },
        }),
    ])))
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::storage::fields;
    use chirpstack_api::internal;
    use std::collections::HashMap;

    #[test]
    fn test_handle() {
        let mut dev = device::Device {
            device_session: Some(
                internal::DeviceSession {
                    tx_power_index: 3,
                    min_supported_tx_power_index: 1,
                    max_supported_tx_power_index: 5,
                    extra_uplink_channels: [(3, Default::default())].iter().cloned().collect(),
                    rx1_delay: 3,
                    rx1_dr_offset: 1,
                    rx2_dr: 5,
                    rx2_frequency: 868900000,
                    enabled_uplink_channel_indices: vec![0, 1],
                    class_b_ping_slot_dr: 3,
                    class_b_ping_slot_freq: 868100000,
                    nb_trans: 3,
                    ..Default::default()
                }
                .into(),
            ),
            ..Default::default()
        };
        let dp = device_profile::DeviceProfile {
            supports_otaa: false,
            abp_params: Some(fields::AbpParams {
                rx1_delay: 1,
                rx1_dr_offset: 0,
                rx2_dr: 0,
                rx2_freq: 868300000,
            }),
            class_b_params: Some(fields::ClassBParams {
                ping_slot_dr: 2,
                ping_slot_freq: 868100000,
                ping_slot_periodicity: 6,
                timeout: 0,
            }),
            ..Default::default()
        };

        let resp = handle(
            &mut dev,
            &dp,
            &lrwn::MACCommandSet::new(vec![lrwn::MACCommand::ResetInd(lrwn::ResetIndPayload {
                dev_lorawan_version: lrwn::Version::LoRaWAN1_1,
            })]),
        )
        .unwrap();

        assert_eq!(
            Some(lrwn::MACCommandSet::new(vec![lrwn::MACCommand::ResetConf(
                lrwn::ResetConfPayload {
                    serv_lorawan_version: lrwn::Version::LoRaWAN1_1
                }
            )])),
            resp
        );

        assert_eq!(
            &internal::DeviceSession {
                rx1_delay: 1,
                rx1_dr_offset: 0,
                rx2_dr: 0,
                rx2_frequency: 868300000,
                tx_power_index: 0,
                dr: 0,
                min_supported_tx_power_index: 0,
                max_supported_tx_power_index: 0,
                nb_trans: 1,
                enabled_uplink_channel_indices: Vec::new(),
                class_b_ping_slot_nb: 2,
                class_b_ping_slot_dr: 2,
                class_b_ping_slot_freq: 868100000,
                extra_uplink_channels: HashMap::new(),
                ..Default::default()
            },
            dev.get_device_session().unwrap()
        );
    }
}
