use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceLayout {
    #[serde(alias = "serialKey")]
    pub serial_key: String,
    #[serde(alias = "deviceName")]
    pub device_name: String,
    pub vid: u16,
    pub pid: u16,
    #[serde(alias = "matrixKeyButtons")]
    pub matrix_key_buttons: Vec<String>,
    #[serde(alias = "matrixLedCode")]
    pub matrix_led_code: Vec<String>,
    #[serde(alias = "buttoninfoDefault")]
    pub buttoninfo_default: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize)]
struct LayoutFile {
    scale: f32,
    nodes: Vec<LayoutNode>,
}

#[derive(Debug, Clone, Deserialize)]
struct LayoutNode {
    id: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    #[serde(default = "default_true", alias = "isKey")]
    is_key: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone)]
pub struct KeyVisual {
    pub matrix_index: usize,
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Debug)]
pub struct KeyboardLayout {
    pub device: DeviceLayout,
    pub keys: Vec<KeyVisual>,
    pub led_index_by_matrix_key: HashMap<String, usize>,
}

impl KeyboardLayout {
    pub fn load_embedded() -> Result<Self, String> {
        let device: DeviceLayout = serde_json::from_str(include_str!("../../assets/gmmk_96_us_device.json"))
            .map_err(|e| e.to_string())?;
        let layout: LayoutFile =
            serde_json::from_str(include_str!("../../assets/gmmk_96_us_layout.json"))
            .map_err(|e| e.to_string())?;

        let mut led_index = HashMap::new();
        for (i, code) in device.matrix_led_code.iter().enumerate() {
            if !code.is_empty() {
                led_index.insert(normalize_key_id(code), i);
            }
        }

        let mut keys = Vec::new();
        for (matrix_index, key_id) in device.matrix_key_buttons.iter().enumerate() {
            let norm = normalize_key_id(key_id);
            let node = layout.nodes.iter().find(|node| node.is_key && layout_id_matches(&node.id, &norm));
            if let Some(node) = node {
                keys.push(KeyVisual {
                    matrix_index,
                    label: key_id.clone(),
                    x: node.x * layout.scale,
                    y: node.y * layout.scale,
                    w: node.w * layout.scale,
                    h: node.h * layout.scale,
                });
            }
        }

        Ok(Self {
            device,
            keys,
            led_index_by_matrix_key: led_index,
        })
    }

    pub fn led_index_for_matrix_key(&self, matrix_key: &str) -> Option<usize> {
        self.led_index_by_matrix_key
            .get(&normalize_key_id(matrix_key))
            .copied()
    }
}

fn normalize_key_id(s: &str) -> String {
    s.trim().to_lowercase().replace(' ', "")
}

fn layout_id_matches(layout_id: &str, matrix_norm: &str) -> bool {
    let normalized = layout_id.to_lowercase().replace(' ', "");
    if normalized == "period" {
        return matrix_norm == "dot";
    }
    if normalized == matrix_norm {
        return true;
    }
    if normalized.strip_prefix("key") == Some(matrix_norm)
        || normalized.strip_prefix("digit") == Some(matrix_norm)
    {
        return true;
    }
    match (normalized.as_str(), matrix_norm) {
        ("escape", "esc") => true,
        ("printscreen", "print") => true,
        ("controlleft", "ctrl") => true,
        ("metaleft", "leftwin") => true,
        ("altleft", "alt") => true,
        ("altright", "ralt") => true,
        ("custom_fnkey", "fn") => true,
        ("controlright", "rctrl") => true,
        ("shiftright", "rshift") => true,
        ("shiftleft", "shift") => true,
        ("arrowleft", "left") => true,
        ("arrowdown", "down") => true,
        ("arrowright", "right") => true,
        ("arrowup", "up") => true,
        ("numpaddivide", "numdivide") => true,
        ("numpadmultiply", "nummulti") => true,
        ("numpadsubtract", "numminus") => true,
        ("numpadadd", "numplus") => true,
        ("numpaddecimal", "numdot") => true,
        ("numpadenter", "numenter") => true,
        ("numpad0", "num0") => true,
        ("numpad1", "num1") => true,
        ("numpad2", "num2") => true,
        ("numpad3", "num3") => true,
        ("numpad4", "num4") => true,
        ("numpad5", "num5") => true,
        ("numpad6", "num6") => true,
        ("numpad7", "num7") => true,
        ("numpad8", "num8") => true,
        ("numpad9", "num9") => true,
        ("backquote", "period") => true,
        ("quote", "'") => true,
        ("semicolon", ";") => true,
        ("comma", ",") => true,
        ("bracketleft", "[") => true,
        ("bracketright", "]") => true,
        ("slash", "/") => true,
        ("backslash", "k29") => true,
        ("minus", "-") => true,
        ("equal", "=") => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_physical_key_has_a_unique_visual_and_led_address() {
        let layout = KeyboardLayout::load_embedded().unwrap();
        assert_eq!(layout.device.matrix_key_buttons.len(), 99);
        let missing: Vec<_> = layout.device.matrix_key_buttons.iter().enumerate()
            .filter(|(index, _)| !layout.keys.iter().any(|key| key.matrix_index == *index))
            .map(|(_, name)| name.as_str()).collect();
        assert!(missing.is_empty(), "missing keys: {missing:?}");
        let mut positions = std::collections::HashSet::new();
        let mut leds = std::collections::HashSet::new();
        for key in &layout.keys {
            assert!(positions.insert((key.x.to_bits(), key.y.to_bits())), "overlap: {}", key.label);
            let led = layout.led_index_for_matrix_key(&key.label).unwrap();
            assert!(leds.insert(led), "duplicate LED: {}", key.label);
        }
    }

    #[test]
    fn punctuation_and_numpad_enter_map_to_their_own_keys() {
        for (visual, matrix) in [
            ("Backquote", "period"), ("Period", "dot"), ("Comma", ","),
            ("BracketLeft", "["), ("BracketRight", "]"), ("NumpadEnter", "numenter"),
        ] {
            assert!(layout_id_matches(visual, matrix), "{visual} -> {matrix}");
        }
        assert!(!layout_id_matches("Period", "period"));
        assert!(!layout_id_matches("Comma", "dot"));
    }
}
