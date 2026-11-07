use crate::protocol::validate_report;

const FEATURE_REPORT_SIZE: usize = 256;
const CONFIG_DESCRIPTOR: &[u8] = &[
    0x06, 0x01, 0xff, 0x09, 0x01, 0xa1, 0x01, 0x85, 0x07, 0x15, 0x00, 0x26,
    0xff, 0x00, 0x09, 0x20, 0x75, 0x08, 0x96, 0xff, 0x00, 0xb1, 0x02, 0xc0,
];

pub(super) fn validate_descriptor(descriptor: &[u8]) -> Result<(), String> {
    if descriptor != CONFIG_DESCRIPTOR {
        return Err("Unsupported configuration HID descriptor; expected GMMK 2 report 7 with 255 payload bytes. No settings sent.".into());
    }
    Ok(())
}

pub(super) fn feature_report(staging: &[u8]) -> Result<Vec<u8>, String> {
    validate_report(staging)?;
    if staging.get(FEATURE_REPORT_SIZE..).is_some_and(|tail| tail.iter().any(|byte| *byte != 0)) {
        return Err("Settings exceed the 256-byte HID feature report; refusing to discard nonzero data".into());
    }
    let mut wire = vec![0; FEATURE_REPORT_SIZE];
    let length = staging.len().min(FEATURE_REPORT_SIZE);
    wire[..length].copy_from_slice(&staging[..length]);
    Ok(wire)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{
        build_change_profile, build_key_matrix, build_led_effect, build_led_layout_chunk,
        build_led_type, build_polling_rate,
    };

    #[test]
    fn every_supported_command_fits_the_physical_feature_report() {
        let matrix = (0..240).collect::<Vec<u8>>();
        let reports = [
            build_led_type(2, 2, 0, 100, true, 16).to_vec(),
            build_key_matrix(2, 2, &matrix).unwrap().to_vec(),
            build_change_profile(2, 2).to_vec(),
            build_led_effect(0, 50, 100, [11, 22, 33], false).to_vec(),
            build_led_layout_chunk(2, 2, 3, &[123; 200], 20).unwrap().to_vec(),
            build_polling_rate(1000, 1, 0).unwrap().to_vec(),
        ];
        for staging in reports {
            let wire = feature_report(&staging).unwrap();
            assert_eq!(wire.len(), 256, "command {}", staging[1]);
            let meaningful = staging.len().min(256);
            assert_eq!(&wire[..meaningful], &staging[..meaningful]);
            assert!(wire[meaningful..].iter().all(|byte| *byte == 0));
        }
    }

    #[test]
    fn nonzero_staging_tail_is_rejected_instead_of_truncated() {
        for index in 256..264 {
            let mut staging = build_led_type(0, 0, 0, 100, true, 16);
            staging[index] = 1;
            assert!(feature_report(&staging).is_err(), "tail byte {index}");
        }
    }

    #[test]
    fn invalid_staging_report_is_rejected_before_framing() {
        for report in [vec![], vec![7; 256], vec![0; 264]] {
            assert!(feature_report(&report).is_err());
        }
    }

    #[test]
    fn only_the_observed_configuration_descriptor_is_accepted() {
        assert!(validate_descriptor(CONFIG_DESCRIPTOR).is_ok());
        for length in 0..CONFIG_DESCRIPTOR.len() {
            assert!(validate_descriptor(&CONFIG_DESCRIPTOR[..length]).is_err());
        }
        for index in 0..CONFIG_DESCRIPTOR.len() {
            let mut changed = CONFIG_DESCRIPTOR.to_vec();
            changed[index] ^= 1;
            assert!(validate_descriptor(&changed).is_err());
        }
    }
}
