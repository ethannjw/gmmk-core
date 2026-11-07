//! Vendor HID feature reports for GMMK 2 96% US (Glorious CORE GmmkSeries, behavioral reference).

pub const VID: u16 = 0x320f;
pub const PID: u16 = 0x504b;
pub const PID_BOOTLOADER_US: u16 = 0xb00b;
pub const SERIAL_KEY: &str = "0x320F0x504B";
pub const REPORT_ID: u8 = 7;
pub const REPORT_SIZE: usize = 264;
pub const ACTIVATION_REPORT_SIZE: usize = 64;
pub const KEY_MATRIX_LEN: usize = 120;
pub const LED_COUNT: usize = 140;
pub const SIDE_LED_INDICES: std::ops::Range<usize> = 120..LED_COUNT;

pub const USAGE_PAGE_CONFIG: u16 = 0xff01;
pub const USAGE_CONFIG: u16 = 0x01;

pub const POLLING_RATES_HZ: [u32; 4] = [1000, 500, 250, 125];

/// UI effect id → firmware effect index (CORE `arrLEDType`).
pub const LED_EFFECT_IDS: [u8; 19] = [
    8, 0, 1, 3, 4, 5, 2, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
];

pub const LED_EFFECT_LABELS: [&str; 19] = [
    "Off",
    "Glorious Mode",
    "Wave #1",
    "Wave #2",
    "Spiraling Wave",
    "Acid Mode",
    "Breathing",
    "Normally On",
    "Ripple Graff",
    "Pass Without Trace",
    "Fast Run Without Trace",
    "Matrix 2",
    "Matrix 3",
    "Rainbow",
    "Heartbeat Sensor",
    "Digital Times",
    "Kamehemeha",
    "Pingpong",
    "Surmount",
];

fn alloc_report() -> [u8; REPORT_SIZE] {
    let mut buf = [0u8; REPORT_SIZE];
    buf[0] = REPORT_ID;
    buf
}

pub fn build_change_profile(profile_index: u8, layer_index: u8) -> [u8; ACTIVATION_REPORT_SIZE] {
    let mut buf = [0; ACTIVATION_REPORT_SIZE];
    buf[0] = REPORT_ID;
    buf[1] = 1;
    buf[2] = profile_index.saturating_add(1);
    buf[3] = layer_index.saturating_add(1);
    buf
}

pub fn build_polling_rate(rate_hz: u32, ep2_enable: u8, led_no_change: u8) -> Result<[u8; REPORT_SIZE], String> {
    let idx = POLLING_RATES_HZ
        .iter()
        .position(|&r| r == rate_hz)
        .ok_or_else(|| format!("unsupported polling rate {rate_hz}"))?;
    let mut buf = alloc_report();
    buf[1] = 8;
    buf[8] = idx as u8;
    buf[9] = ep2_enable;
    buf[10] = led_no_change;
    Ok(buf)
}

pub fn build_led_type(
    profile_index: u8,
    layer_index: u8,
    effect_ui_value: u8,
    brightness_0_100: u8,
    side_light_on: bool,
    input_latency_ms: u8,
) -> [u8; REPORT_SIZE] {
    let effect_index = LED_EFFECT_IDS
        .iter()
        .position(|&v| v == effect_ui_value)
        .unwrap_or(0) as u8;
    let mut buf = alloc_report();
    buf[1] = 7;
    buf[2] = profile_index.saturating_add(1);
    buf[3] = layer_index.saturating_add(1);
    buf[4] = effect_index;
    buf[5] = 0;
    buf[6] = 1;
    buf[7] = if side_light_on { 0 } else { 1 };
    buf[8] = ((brightness_0_100 as u16 * 20) / 100) as u8;
    buf[10] = input_latency_ms;
    buf
}

pub fn build_led_effect(
    effect_ui_value: u8,
    speed_0_100: u8,
    brightness_0_100: u8,
    rgb: [u8; 3],
    multicolor: bool,
) -> [u8; REPORT_SIZE] {
    let effect_index = LED_EFFECT_IDS
        .iter()
        .position(|&v| v == effect_ui_value)
        .unwrap_or(0) as u8;
    let mut buf = alloc_report();
    buf[1] = 2;
    buf[2] = effect_index;
    buf[8] = 100u8.saturating_sub(speed_0_100);
    buf[9] = ((brightness_0_100 as u16 * 20) / 100) as u8;
    if multicolor {
        buf[10] = 1;
        buf[12] = 1;
    }
    buf[15] = rgb[0];
    buf[16] = rgb[1];
    buf[17] = rgb[2];
    buf
}

pub fn build_key_matrix(profile_index: u8, layer_index: u8, data: &[u8]) -> Result<[u8; REPORT_SIZE], String> {
    if data.len() != KEY_MATRIX_LEN * 2 {
        return Err(format!(
            "key matrix must be {} bytes, got {}",
            KEY_MATRIX_LEN * 2,
            data.len()
        ));
    }
    let mut buf = alloc_report();
    buf[1] = 3;
    buf[2] = profile_index.saturating_add(1);
    buf[3] = layer_index.saturating_add(1);
    buf[8..8 + data.len()].copy_from_slice(data);
    Ok(buf)
}

pub fn build_led_layout_chunk(
    profile_index: u8,
    layer_index: u8,
    chunk_index: u8,
    chunk: &[u8],
    brightness_scaled: u8,
) -> Result<[u8; REPORT_SIZE], String> {
    if chunk.len() > 200 {
        return Err("per-key chunk max 200 bytes".into());
    }
    let mut buf = alloc_report();
    buf[1] = 6;
    buf[2] = profile_index.saturating_add(1);
    buf[3] = layer_index.saturating_add(1);
    buf[4] = 83;
    buf[5] = chunk_index;
    buf[6] = brightness_scaled;
    buf[8..8 + chunk.len()].copy_from_slice(chunk);
    Ok(buf)
}

pub fn validate_report(buf: &[u8]) -> Result<(), String> {
    if buf.len() < 2 {
        return Err("report must contain an id and command".into());
    }
    let expected_length = match buf[1] {
        1 => ACTIVATION_REPORT_SIZE,
        2 | 3 | 6 | 7 | 8 => REPORT_SIZE,
        command => return Err(format!("unsupported report command {command}")),
    };
    if buf.len() != expected_length {
        return Err(format!("command {} report length must be {expected_length}", buf[1]));
    }
    if buf[0] != REPORT_ID {
        return Err("report id must be 7".into());
    }
    if matches!(buf[1], 1 | 3 | 6 | 7)
        && (!(1..=3).contains(&buf[2]) || !(1..=3).contains(&buf[3]))
    {
        return Err("encoded profile and layer must be 1, 2, or 3".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activation_matches_the_windows_64_byte_report() {
        let report = build_change_profile(2, 1);
        assert_eq!(report.len(), 64);
        assert_eq!(&report[..4], &[7, 1, 3, 2]);
        assert!(report[4..].iter().all(|byte| *byte == 0));
        assert!(validate_report(&report).is_ok());
    }

    #[test]
    fn report_validation_checks_command_specific_lengths_and_known_commands() {
        let mut activation = vec![0; 64];
        activation[..4].copy_from_slice(&[7, 1, 1, 1]);
        assert!(validate_report(&activation).is_ok());
        activation.resize(264, 0);
        assert!(validate_report(&activation).is_err());
        for command in [2, 3, 6, 7, 8] {
            let mut report = vec![0; 64];
            report[..4].copy_from_slice(&[7, command, 1, 1]);
            assert!(validate_report(&report).is_err(), "short data command {command}");
            report.resize(264, 0);
            assert!(validate_report(&report).is_ok(), "valid data command {command}");
            report[0] = 0;
            assert!(validate_report(&report).is_err(), "wrong report id for command {command}");
        }
        for command in [0, 4, 5, 9, 138, 255] {
            let mut report = [0; 264];
            report[..4].copy_from_slice(&[7, command, 1, 1]);
            assert!(validate_report(&report).is_err(), "unimplemented command {command}");
        }
        for length in [0, 1, 2, 63, 65, 263, 265] {
            assert!(validate_report(&vec![0; length]).is_err());
        }
    }

    #[test]
    fn invalid_profile_or_layer_headers_are_rejected_at_the_send_boundary() {
        for (profile, layer) in [(3, 0), (255, 0), (0, 3), (0, 255)] {
            assert!(validate_report(&build_change_profile(profile, layer)).is_err());
            assert!(validate_report(&build_key_matrix(profile, layer, &[0; 240]).unwrap()).is_err());
            assert!(validate_report(&build_led_type(profile, layer, 0, 100, true, 16)).is_err());
            assert!(validate_report(&build_led_layout_chunk(profile, layer, 0, &[0; 200], 20).unwrap()).is_err());
        }
        for profile in 0..3 {
            for layer in 0..3 {
                assert!(validate_report(&build_change_profile(profile, layer)).is_ok());
            }
        }
    }

    #[test]
    fn preset_speed_matches_windows_core_2_1_23() {
        for speed in 0..=100 {
            assert_eq!(build_led_effect(0, speed, 100, [10, 20, 30], false)[8], 100 - speed);
        }
    }

    #[test]
    fn preset_reports_match_shared_windows_reference_vectors() {
        #[derive(serde::Deserialize)]
        struct EffectVector {
            identifier: String,
            local_id: u8,
            wire_id: u8,
        }
        #[derive(serde::Deserialize)]
        struct Sample {
            name: String,
            profile: u8,
            layer: u8,
            speed: u8,
            brightness: u8,
            rgb: [u8; 3],
            latency: u8,
            multicolor: bool,
            properties_prefix: Vec<u8>,
            effect_prefix: Vec<u8>,
        }
        #[derive(serde::Deserialize)]
        struct ReferenceVectors {
            report_size: usize,
            effects: Vec<EffectVector>,
            samples: Vec<Sample>,
        }
        let vectors: ReferenceVectors = serde_json::from_str(include_str!(
            "../../assets/windows-preset-lighting-vectors.json"
        )).unwrap();
        assert_eq!(vectors.report_size, REPORT_SIZE);
        assert_eq!(vectors.effects.len(), 19);
        assert_eq!(vectors.samples.len(), 8);
        for effect in &vectors.effects {
            for sample in &vectors.samples {
                let mut expected_properties = [0; REPORT_SIZE];
                expected_properties[..sample.properties_prefix.len()].copy_from_slice(&sample.properties_prefix);
                expected_properties[4] = effect.wire_id;
                let mut expected_effect = [0; REPORT_SIZE];
                expected_effect[..sample.effect_prefix.len()].copy_from_slice(&sample.effect_prefix);
                expected_effect[2] = effect.wire_id;
                assert_eq!(build_led_type(
                    sample.profile, sample.layer, effect.local_id, sample.brightness, true, sample.latency,
                ), expected_properties, "{} / {} properties", effect.identifier, sample.name);
                assert_eq!(build_led_effect(
                    effect.local_id, sample.speed, sample.brightness, sample.rgb, sample.multicolor,
                ), expected_effect, "{} / {} effect", effect.identifier, sample.name);
                assert!(validate_report(&expected_properties).is_ok());
                assert!(validate_report(&expected_effect).is_ok());
            }
        }
    }

    #[test]
    fn multicolor_sets_both_windows_effect_flags() {
        let report = build_led_effect(0, 50, 100, [10, 20, 30], true);
        assert_eq!(report[10], 1);
        assert_eq!(report[12], 1);
        let report = build_led_effect(13, 50, 100, [10, 20, 30], false);
        assert_eq!(report[10], 0);
        assert_eq!(report[12], 0);
    }

    #[test]
    fn imported_effect_ids_map_to_firmware_indices() {
        for (index, effect_id) in LED_EFFECT_IDS.iter().enumerate() {
            assert_eq!(build_led_effect(*effect_id, 50, 100, [1, 2, 3], false)[2], index as u8);
            assert_eq!(build_led_type(0, 0, *effect_id, 100, true, 16)[4], index as u8);
        }
        assert_eq!(LED_EFFECT_IDS[0], 8);
        assert_eq!(LED_EFFECT_IDS[1], 0);
    }

    #[test]
    fn key_packet_preserves_payload_and_reserved_bytes() {
        let matrix: Vec<u8> = (0..240).collect();
        let report = build_key_matrix(2, 1, &matrix).unwrap();
        assert_eq!(&report[..8], &[7, 3, 3, 2, 0, 0, 0, 0]);
        assert_eq!(&report[8..248], matrix.as_slice());
        assert!(report[248..].iter().all(|byte| *byte == 0));
        assert!(validate_report(&report).is_ok());
    }

    #[test]
    fn invalid_payload_lengths_are_rejected() {
        assert!(build_key_matrix(0, 0, &[0; 239]).is_err());
        assert!(build_key_matrix(0, 0, &[0; 241]).is_err());
        assert!(build_led_layout_chunk(0, 0, 0, &[0; 201], 20).is_err());
        assert!(build_polling_rate(8000, 1, 0).is_err());
    }
}
