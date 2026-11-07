use crate::protocol::{KEY_MATRIX_LEN, LED_COUNT, LED_EFFECT_IDS, PID, POLLING_RATES_HZ, SERIAL_KEY, VID};
use serde::{Deserialize, Serialize};

pub const FORMAT_ID: &str = "gmmk-config-profile";
pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRef {
    #[serde(alias = "serialKey")]
    pub serial_key: String,
    pub vid: u16,
    pub pid: u16,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerKeyColor {
    pub matrix_index: usize,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub visible: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightingState {
    #[serde(alias = "effectUiValue")]
    pub effect_ui_value: u8,
    pub speed: u8,
    pub brightness: u8,
    pub rgb: [u8; 3],
    #[serde(alias = "sideLightOn")]
    pub side_light_on: bool,
    #[serde(alias = "inputLatencyMs")]
    pub input_latency_ms: u8,
    #[serde(alias = "perKeyMode")]
    pub per_key_mode: u8,
    #[serde(default, alias = "perKeyColors")]
    pub per_key_colors: Option<Vec<PerKeyColor>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerState {
    pub layer: u8,
    #[serde(alias = "keyMatrix")]
    pub key_matrix: Vec<u8>,
    #[serde(alias = "pollingHz")]
    pub polling_hz: u32,
    pub lighting: LightingState,
}

impl LayerState {
    pub fn validate(&self) -> Result<(), String> {
        if self.layer >= 3 {
            return Err("layer must be 0, 1, or 2".into());
        }
        if self.key_matrix.len() != KEY_MATRIX_LEN * 2 {
            return Err(format!("layer {} matrix must be {} bytes", self.layer, KEY_MATRIX_LEN * 2));
        }
        if !POLLING_RATES_HZ.contains(&self.polling_hz) {
            return Err(format!("unsupported polling rate {}", self.polling_hz));
        }
        self.lighting.validate()
    }
}

impl LightingState {
    pub fn validate(&self) -> Result<(), String> {
        if !LED_EFFECT_IDS.contains(&self.effect_ui_value) {
            return Err(format!("unsupported lighting effect {}", self.effect_ui_value));
        }
        if self.speed > 100 || self.brightness > 100 {
            return Err("lighting speed and brightness must be between 0 and 100".into());
        }
        if let Some(colors) = &self.per_key_colors {
            let mut seen = [false; LED_COUNT];
            for color in colors {
                if color.matrix_index >= LED_COUNT {
                    return Err(format!("LED index {} is outside the device lighting map", color.matrix_index));
                }
                if seen[color.matrix_index] {
                    return Err(format!("duplicate LED index {}", color.matrix_index));
                }
                seen[color.matrix_index] = true;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardProfile {
    pub id: u8,
    pub name: String,
    pub layers: Vec<LayerState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileFile {
    pub format: String,
    pub version: u32,
    pub device: DeviceRef,
    #[serde(alias = "activeProfile")]
    pub active_profile: u8,
    pub profiles: Vec<OnboardProfile>,
}

impl ProfileFile {
    pub fn validate(&self) -> Result<(), String> {
        if self.format != FORMAT_ID {
            return Err(format!("unknown format {}", self.format));
        }
        if self.version != FORMAT_VERSION {
            return Err(format!("unsupported version {}", self.version));
        }
        if self.device.serial_key != SERIAL_KEY {
            return Err(format!(
                "profile is for {}, not {}",
                self.device.serial_key, SERIAL_KEY
            ));
        }
        if self.device.vid != VID || self.device.pid != PID {
            return Err("profile USB identity does not match GMMK 2 96% ANSI".into());
        }
        if self.active_profile >= 3 {
            return Err("active profile must be 0, 1, or 2".into());
        }
        if self.profiles.len() != 3 {
            return Err("expected 3 onboard profiles".into());
        }
        for (profile_index, profile) in self.profiles.iter().enumerate() {
            if profile.id as usize != profile_index {
                return Err("profiles must have unique IDs 0, 1, 2 in order".into());
            }
            if profile.layers.len() != 3 {
                return Err(format!("profile {} must have 3 layers", profile.id));
            }
            for (layer_index, layer) in profile.layers.iter().enumerate() {
                if layer.layer as usize != layer_index {
                    return Err("layers must have unique IDs 0, 1, 2 in order".into());
                }
                layer.validate()?;
            }
        }
        Ok(())
    }
}

pub fn parse_profile_json(text: &str) -> Result<ProfileFile, String> {
    let file: ProfileFile = serde_json::from_str(text).map_err(|e| e.to_string())?;
    file.validate()?;
    Ok(file)
}

pub fn serialize_profile(file: &ProfileFile) -> Result<String, String> {
    file.validate()?;
    serde_json::to_string_pretty(file).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ProfileFile {
        serde_json::from_str(include_str!("../../assets/factory_profile.gmmkprofile.json")).unwrap()
    }

    #[test]
    fn embedded_profile_round_trips_without_changing_bytes() {
        let original = fixture();
        let restored = parse_profile_json(&serialize_profile(&original).unwrap()).unwrap();
        assert_eq!(serde_json::to_value(restored).unwrap(), serde_json::to_value(original).unwrap());
    }

    #[test]
    fn rejects_wrong_usb_identity_even_with_matching_serial_key() {
        let mut profile = fixture();
        profile.device.vid ^= 1;
        assert!(profile.validate().is_err());
        let mut profile = fixture();
        profile.device.pid ^= 1;
        assert!(profile.validate().is_err());
    }

    #[test]
    fn rejects_invalid_active_slot() {
        let mut profile = fixture();
        profile.active_profile = 3;
        assert!(profile.validate().is_err());
    }

    #[test]
    fn rejects_duplicate_or_out_of_order_slots_and_layers() {
        let mut profile = fixture();
        profile.profiles[1].id = 0;
        assert!(profile.validate().is_err());
        let mut profile = fixture();
        profile.profiles.swap(0, 1);
        assert!(profile.validate().is_err());
        let mut profile = fixture();
        profile.profiles[0].layers[1].layer = 0;
        assert!(profile.validate().is_err());
        let mut profile = fixture();
        profile.profiles[0].layers.swap(0, 1);
        assert!(profile.validate().is_err());
    }

    #[test]
    fn rejects_unsupported_polling_and_lighting_values() {
        for mutation in [
            (|layer: &mut LayerState| layer.polling_hz = 8000) as fn(&mut LayerState),
            |layer| layer.lighting.effect_ui_value = 255,
            |layer| layer.lighting.speed = 101,
            |layer| layer.lighting.brightness = 101,
        ] {
            let mut profile = fixture();
            mutation(&mut profile.profiles[0].layers[0]);
            assert!(profile.validate().is_err());
        }
    }

    #[test]
    fn custom_lighting_side_leds_round_trip_without_changing_key_matrix_limits() {
        let mut profile = fixture();
        profile.profiles[0].layers[0].lighting.per_key_colors = Some((120..140).map(|matrix_index| PerKeyColor {
            matrix_index, r: 12, g: 34, b: 56, visible: 254,
        }).collect());
        let serialized = serialize_profile(&profile).unwrap();
        let restored = parse_profile_json(&serialized).unwrap();
        assert_eq!(serialize_profile(&restored).unwrap(), serialized);
        assert_eq!(restored.profiles[0].layers[0].key_matrix.len(), 240);
        for index in [140, 159, usize::MAX] {
            profile.profiles[0].layers[0].lighting.per_key_colors.as_mut().unwrap()[0].matrix_index = index;
            assert!(profile.validate().is_err());
        }
        profile.profiles[0].layers[0].lighting.per_key_colors = Some(vec![
            PerKeyColor { matrix_index: 139, r: 0, g: 0, b: 0, visible: 255 }; 2
        ]);
        assert!(profile.validate().is_err());
    }

    #[test]
    fn rejects_out_of_range_and_duplicate_led_entries() {
        let mut profile = fixture();
        let color = PerKeyColor { matrix_index: LED_COUNT, r: 10, g: 20, b: 30, visible: 255 };
        profile.profiles[0].layers[0].lighting.per_key_colors = Some(vec![color.clone()]);
        assert!(profile.validate().is_err());
        let color = PerKeyColor { matrix_index: 0, ..color };
        profile.profiles[0].layers[0].lighting.per_key_colors = Some(vec![color.clone(), color]);
        assert!(profile.validate().is_err());
    }

    #[test]
    fn rejects_invalid_matrix_lengths() {
        for length in [0, KEY_MATRIX_LEN, KEY_MATRIX_LEN * 2 - 1, KEY_MATRIX_LEN * 2 + 1] {
            let mut profile = fixture();
            profile.profiles[0].layers[0].key_matrix = vec![0; length];
            assert!(profile.validate().is_err());
        }
    }
}
