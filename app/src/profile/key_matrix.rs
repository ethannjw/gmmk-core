use crate::layout::KeyboardLayout;
use crate::protocol::KEY_MATRIX_LEN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MediaAction {
    OpenMediaPlayer = 0,
    PlayPause = 1,
    Mute = 2,
    VolumeUp = 3,
    VolumeDown = 4,
    Stop = 5,
    PreviousTrack = 6,
    NextTrack = 7,
}

impl MediaAction {
    pub const ALL: [Self; 8] = [
        Self::OpenMediaPlayer, Self::PlayPause, Self::Stop, Self::Mute,
        Self::VolumeUp, Self::VolumeDown, Self::NextTrack, Self::PreviousTrack,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::OpenMediaPlayer => "Open Media Player",
            Self::PlayPause => "Play / Pause",
            Self::Stop => "Stop",
            Self::Mute => "Mute",
            Self::VolumeUp => "Volume Up",
            Self::VolumeDown => "Volume Down",
            Self::NextTrack => "Next Track",
            Self::PreviousTrack => "Previous Track",
        }
    }

    fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|action| *action as u8 == code)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Default,
    Disabled,
    HidKey { code: u8, ctrl: bool, shift: bool, alt: bool, win: bool },
    Multimedia(MediaAction),
}

pub fn decode_key_action(matrix: &[u8], layout: &KeyboardLayout, matrix_index: usize) -> KeyAction {
    if matrix.len() != KEY_MATRIX_LEN * 2 {
        return KeyAction::Default;
    }
    let Some(key) = layout.device.matrix_key_buttons.get(matrix_index) else {
        return KeyAction::Default;
    };
    let led = match layout.led_index_for_matrix_key(key) {
        Some(i) => i,
        None => return KeyAction::Default,
    };
    if led >= KEY_MATRIX_LEN {
        return KeyAction::Default;
    }
    let binding_type = matrix[led];
    let code = matrix[KEY_MATRIX_LEN + led];
    if binding_type == 3 {
        return MediaAction::from_code(code).map(KeyAction::Multimedia).unwrap_or(KeyAction::Default);
    }
    if binding_type == 1 && code == 0 {
        return KeyAction::Disabled;
    }
    if binding_type == 1 || (224..=239).contains(&binding_type) {
        let modifiers = if binding_type == 1 { 0 } else { binding_type - 224 };
        let ctrl = modifiers & 1 != 0;
        let shift = modifiers & 2 != 0;
        let alt = modifiers & 4 != 0;
        let win = modifiers & 8 != 0;
        return KeyAction::HidKey {
            code,
            ctrl,
            shift,
            alt,
            win,
        };
    }
    KeyAction::Default
}

pub fn apply_key_action(matrix: &mut [u8], layout: &KeyboardLayout, matrix_index: usize, action: KeyAction) {
    if matrix.len() != KEY_MATRIX_LEN * 2 {
        return;
    }
    let Some(key) = layout.device.matrix_key_buttons.get(matrix_index) else {
        return;
    };
    let led = match layout.led_index_for_matrix_key(key) {
        Some(i) => i,
        None => return,
    };
    if led >= KEY_MATRIX_LEN {
        return;
    }
    match action {
        KeyAction::Multimedia(action) => {
            matrix[led] = 3;
            matrix[KEY_MATRIX_LEN + led] = action as u8;
        }
        KeyAction::Default => {
            // leave bytes unchanged (factory default from import)
        }
        KeyAction::Disabled => {
            matrix[led] = 1;
            matrix[KEY_MATRIX_LEN + led] = 0;
        }
        KeyAction::HidKey {
            code,
            ctrl,
            shift,
            alt,
            win,
        } => {
            let mut t = 1u8;
            let mut complex = 0u8;
            if ctrl {
                complex |= 1;
            }
            if shift {
                complex |= 2;
            }
            if alt {
                complex |= 4;
            }
            if win {
                complex |= 8;
            }
            if complex > 0 {
                t = 224 + complex;
            }
            matrix[led] = t;
            matrix[KEY_MATRIX_LEN + led] = code;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardCategory {
    Alphanumeric,
    FRow,
    Modifiers,
    SpecialCharacters,
    Others,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardChoice {
    pub id: &'static str,
    pub label: &'static str,
    pub code: Option<u8>,
}

impl KeyboardCategory {
    pub const ALL: [Self; 5] = [
        Self::Alphanumeric, Self::FRow, Self::Modifiers, Self::SpecialCharacters, Self::Others,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Alphanumeric => "Alphanumeric",
            Self::FRow => "F-Row",
            Self::Modifiers => "Modifiers",
            Self::SpecialCharacters => "Special Characters",
            Self::Others => "Others",
        }
    }

    pub fn for_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|category| category.choice_for_code(code).is_some())
    }

    pub fn choice_for_code(self, code: u8) -> Option<&'static KeyboardChoice> {
        self.choices().iter().find(|choice| choice.code == Some(code))
    }

    pub fn choices(self) -> &'static [KeyboardChoice] {
        match self {
            Self::Alphanumeric => &[
                KeyboardChoice { id: "KeyA", label: "A", code: Some(4) },
                KeyboardChoice { id: "KeyB", label: "B", code: Some(5) },
                KeyboardChoice { id: "KeyC", label: "C", code: Some(6) },
                KeyboardChoice { id: "KeyD", label: "D", code: Some(7) },
                KeyboardChoice { id: "KeyE", label: "E", code: Some(8) },
                KeyboardChoice { id: "KeyF", label: "F", code: Some(9) },
                KeyboardChoice { id: "KeyG", label: "G", code: Some(10) },
                KeyboardChoice { id: "KeyH", label: "H", code: Some(11) },
                KeyboardChoice { id: "KeyI", label: "I", code: Some(12) },
                KeyboardChoice { id: "KeyJ", label: "J", code: Some(13) },
                KeyboardChoice { id: "KeyK", label: "K", code: Some(14) },
                KeyboardChoice { id: "KeyL", label: "L", code: Some(15) },
                KeyboardChoice { id: "KeyM", label: "M", code: Some(16) },
                KeyboardChoice { id: "KeyN", label: "N", code: Some(17) },
                KeyboardChoice { id: "KeyO", label: "O", code: Some(18) },
                KeyboardChoice { id: "KeyP", label: "P", code: Some(19) },
                KeyboardChoice { id: "KeyQ", label: "Q", code: Some(20) },
                KeyboardChoice { id: "KeyR", label: "R", code: Some(21) },
                KeyboardChoice { id: "KeyS", label: "S", code: Some(22) },
                KeyboardChoice { id: "KeyT", label: "T", code: Some(23) },
                KeyboardChoice { id: "KeyU", label: "U", code: Some(24) },
                KeyboardChoice { id: "KeyV", label: "V", code: Some(25) },
                KeyboardChoice { id: "KeyW", label: "W", code: Some(26) },
                KeyboardChoice { id: "KeyX", label: "X", code: Some(27) },
                KeyboardChoice { id: "KeyY", label: "Y", code: Some(28) },
                KeyboardChoice { id: "KeyZ", label: "Z", code: Some(29) },
                KeyboardChoice { id: "Digit1", label: "1", code: Some(30) },
                KeyboardChoice { id: "Digit2", label: "2", code: Some(31) },
                KeyboardChoice { id: "Digit3", label: "3", code: Some(32) },
                KeyboardChoice { id: "Digit4", label: "4", code: Some(33) },
                KeyboardChoice { id: "Digit5", label: "5", code: Some(34) },
                KeyboardChoice { id: "Digit6", label: "6", code: Some(35) },
                KeyboardChoice { id: "Digit7", label: "7", code: Some(36) },
                KeyboardChoice { id: "Digit8", label: "8", code: Some(37) },
                KeyboardChoice { id: "Digit9", label: "9", code: Some(38) },
                KeyboardChoice { id: "Digit0", label: "0", code: Some(39) },
            ],
            Self::FRow => &[
                KeyboardChoice { id: "F1", label: "F1", code: Some(58) },
                KeyboardChoice { id: "F2", label: "F2", code: Some(59) },
                KeyboardChoice { id: "F3", label: "F3", code: Some(60) },
                KeyboardChoice { id: "F4", label: "F4", code: Some(61) },
                KeyboardChoice { id: "F5", label: "F5", code: Some(62) },
                KeyboardChoice { id: "F6", label: "F6", code: Some(63) },
                KeyboardChoice { id: "F7", label: "F7", code: Some(64) },
                KeyboardChoice { id: "F8", label: "F8", code: Some(65) },
                KeyboardChoice { id: "F9", label: "F9", code: Some(66) },
                KeyboardChoice { id: "F10", label: "F10", code: Some(67) },
                KeyboardChoice { id: "F11", label: "F11", code: Some(68) },
                KeyboardChoice { id: "F12", label: "F12", code: Some(69) },
            ],
            Self::Modifiers => &[
                KeyboardChoice { id: "ShiftLeft", label: "ShiftLeft", code: Some(225) },
                KeyboardChoice { id: "ShiftRight", label: "ShiftRight", code: Some(229) },
                KeyboardChoice { id: "ControlLeft", label: "ControlLeft", code: Some(224) },
                KeyboardChoice { id: "ControlRight", label: "ControlRight", code: Some(228) },
                KeyboardChoice { id: "AltLeft", label: "AltLeft", code: Some(226) },
                KeyboardChoice { id: "AltRight", label: "AltRight", code: Some(230) },
                KeyboardChoice { id: "MetaLeft", label: "MetaLeft", code: Some(227) },
                KeyboardChoice { id: "Fn", label: "Fn", code: None },
                KeyboardChoice { id: "Tab", label: "Tab", code: Some(43) },
                KeyboardChoice { id: "Escape", label: "Escape", code: Some(41) },
                KeyboardChoice { id: "CapsLock", label: "CapsLock", code: Some(57) },
                KeyboardChoice { id: "Enter", label: "Enter", code: Some(40) },
                KeyboardChoice { id: "Backspace", label: "Backspace", code: Some(42) },
            ],
            Self::SpecialCharacters => &[
                KeyboardChoice { id: "Backquote", label: "`", code: Some(53) },
                KeyboardChoice { id: "Minus", label: "-", code: Some(45) },
                KeyboardChoice { id: "Equal", label: "=", code: Some(46) },
                KeyboardChoice { id: "BracketLeft", label: "[", code: Some(47) },
                KeyboardChoice { id: "BracketRight", label: "]", code: Some(48) },
                KeyboardChoice { id: "Backslash", label: "\\", code: Some(49) },
                KeyboardChoice { id: "Semicolon", label: ";", code: Some(51) },
                KeyboardChoice { id: "Quote", label: "'", code: Some(52) },
                KeyboardChoice { id: "Comma", label: ",", code: Some(54) },
                KeyboardChoice { id: "Period", label: ".", code: Some(55) },
                KeyboardChoice { id: "Slash", label: "/", code: Some(56) },
                KeyboardChoice { id: "Hashtag", label: "#", code: None },
                KeyboardChoice { id: "BackslashLeft", label: "\\", code: None },
            ],
            Self::Others => &[
                KeyboardChoice { id: "ArrowUp", label: "▲", code: Some(82) },
                KeyboardChoice { id: "ArrowDown", label: "▼", code: Some(81) },
                KeyboardChoice { id: "ArrowLeft", label: "◀", code: Some(80) },
                KeyboardChoice { id: "ArrowRight", label: "▶", code: Some(79) },
                KeyboardChoice { id: "Home", label: "Home", code: Some(74) },
                KeyboardChoice { id: "End", label: "End", code: Some(77) },
                KeyboardChoice { id: "PageUp", label: "PgUp", code: Some(75) },
                KeyboardChoice { id: "PageDown", label: "PgDn", code: Some(78) },
                KeyboardChoice { id: "Insert", label: "Ins", code: Some(73) },
                KeyboardChoice { id: "Delete", label: "Del", code: Some(76) },
                KeyboardChoice { id: "Space", label: "Space", code: Some(44) },
                KeyboardChoice { id: "Menu", label: "Menu", code: None },
                KeyboardChoice { id: "PrintScreen", label: "PrtSc", code: Some(70) },
                KeyboardChoice { id: "ScrollLock", label: "ScrLk", code: Some(71) },
                KeyboardChoice { id: "Pause", label: "Pause", code: Some(72) },
                KeyboardChoice { id: "ScreenLock", label: "ScrLk", code: None },
                KeyboardChoice { id: "Numpad0", label: "0", code: Some(98) },
                KeyboardChoice { id: "Numpad1", label: "1", code: Some(89) },
                KeyboardChoice { id: "Numpad2", label: "2", code: Some(90) },
                KeyboardChoice { id: "Numpad3", label: "3", code: Some(91) },
                KeyboardChoice { id: "Numpad4", label: "4", code: Some(92) },
                KeyboardChoice { id: "Numpad5", label: "5", code: Some(93) },
                KeyboardChoice { id: "Numpad6", label: "6", code: Some(94) },
                KeyboardChoice { id: "Numpad7", label: "7", code: Some(95) },
                KeyboardChoice { id: "Numpad8", label: "8", code: Some(96) },
                KeyboardChoice { id: "Numpad9", label: "9", code: Some(97) },
                KeyboardChoice { id: "NumpadAdd", label: "+", code: Some(87) },
                KeyboardChoice { id: "NumpadSubtract", label: "-", code: Some(86) },
                KeyboardChoice { id: "NumpadMultiply", label: "*", code: Some(85) },
                KeyboardChoice { id: "NumpadDivide", label: "/", code: Some(84) },
                KeyboardChoice { id: "NumpadDecimal", label: ".", code: Some(99) },
                KeyboardChoice { id: "NumpadEnter", label: "Enter", code: Some(88) },
                KeyboardChoice { id: "NumLock", label: "Num", code: Some(83) },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout_and_index() -> (KeyboardLayout, usize) {
        let layout = KeyboardLayout::load_embedded().unwrap();
        let index = layout.device.matrix_key_buttons.iter().position(|key| key == "A").unwrap();
        (layout, index)
    }

    #[test]
    fn keyboard_picker_includes_verified_modifier_punctuation_and_numpad_codes() {
        for code in [45, 46, 47, 48, 49, 51, 52, 53, 54, 55, 56, 57, 70, 71, 72, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 224, 225, 226, 227, 228, 229, 230] {
            assert!(KeyboardCategory::for_code(code).is_some(), "missing verified code {code}");
        }
    }

    #[test]
    fn keyboard_categories_preserve_reference_order_and_unresolved_identifiers() {
        let expected = [
            (KeyboardCategory::Alphanumeric, (4..=39).collect::<Vec<u8>>(), vec![]),
            (KeyboardCategory::FRow, (58..=69).collect(), vec![]),
            (KeyboardCategory::Modifiers, vec![225, 229, 224, 228, 226, 230, 227, 43, 41, 57, 40, 42], vec!["Fn"]),
            (KeyboardCategory::SpecialCharacters, vec![53, 45, 46, 47, 48, 49, 51, 52, 54, 55, 56], vec!["Hashtag", "BackslashLeft"]),
            (KeyboardCategory::Others, vec![82, 81, 80, 79, 74, 77, 75, 78, 73, 76, 44, 70, 71, 72, 98, 89, 90, 91, 92, 93, 94, 95, 96, 97, 87, 86, 85, 84, 99, 88, 83], vec!["Menu", "ScreenLock"]),
        ];
        assert_eq!(KeyboardCategory::ALL, expected.each_ref().map(|entry| entry.0));
        let mut identifiers = std::collections::HashSet::new();
        let mut codes = std::collections::HashSet::new();
        for (category, expected_codes, unresolved) in expected {
            assert_eq!(category.choices().iter().filter_map(|choice| choice.code).collect::<Vec<_>>(), expected_codes);
            assert_eq!(category.choices().iter().filter(|choice| choice.code.is_none()).map(|choice| choice.id).collect::<Vec<_>>(), unresolved);
            for choice in category.choices() {
                assert!(identifiers.insert(choice.id));
                assert!(!choice.label.is_empty());
                if let Some(code) = choice.code {
                    assert!(codes.insert(code));
                    assert_eq!(KeyboardCategory::for_code(code), Some(category));
                    assert_eq!(category.choice_for_code(code), Some(choice));
                }
            }
        }
        assert_eq!((identifiers.len(), codes.len()), (107, 102));
        for code in 0..=255 {
            assert_eq!(KeyboardCategory::for_code(code).is_some(), codes.contains(&code));
        }
    }

    #[test]
    fn every_verified_keyboard_choice_round_trips_at_every_physical_key() {
        let layout = KeyboardLayout::load_embedded().unwrap();
        for category in KeyboardCategory::ALL {
            for code in category.choices().iter().filter_map(|choice| choice.code) {
                for (index, key) in layout.device.matrix_key_buttons.iter().enumerate() {
                    let original = &layout.device.buttoninfo_default;
                    let mut matrix = original.clone();
                    let led = layout.led_index_for_matrix_key(key).unwrap();
                    let action = KeyAction::HidKey { code, ctrl: false, shift: false, alt: false, win: false };
                    apply_key_action(&mut matrix, &layout, index, action);
                    assert_eq!((matrix[led], matrix[KEY_MATRIX_LEN + led]), (1, code));
                    assert_eq!(decode_key_action(&matrix, &layout, index), action);
                    matrix[led] = original[led];
                    matrix[KEY_MATRIX_LEN + led] = original[KEY_MATRIX_LEN + led];
                    assert_eq!(&matrix, original);
                }
            }
        }
    }

    #[test]
    fn media_binding_is_recognized_in_an_imported_matrix() {
        let (layout, index) = layout_and_index();
        let led = layout.led_index_for_matrix_key("A").unwrap();
        let mut matrix = layout.device.buttoninfo_default.clone();
        matrix[led] = 3;
        matrix[KEY_MATRIX_LEN + led] = 0;
        assert_eq!(decode_key_action(&matrix, &layout, index), KeyAction::Multimedia(MediaAction::OpenMediaPlayer));
    }

    #[test]
    fn every_media_action_uses_windows_bytes_and_round_trips_on_every_physical_key() {
        let layout = KeyboardLayout::load_embedded().unwrap();
        let expected = [
            (MediaAction::OpenMediaPlayer, 0), (MediaAction::PlayPause, 1),
            (MediaAction::Stop, 5), (MediaAction::Mute, 2),
            (MediaAction::VolumeUp, 3), (MediaAction::VolumeDown, 4),
            (MediaAction::NextTrack, 7), (MediaAction::PreviousTrack, 6),
        ];
        assert_eq!(MediaAction::ALL, expected.map(|(action, _)| action));
        for (action, expected_code) in expected {
            for (index, key) in layout.device.matrix_key_buttons.iter().enumerate() {
                let led = layout.led_index_for_matrix_key(key).unwrap();
                let original = vec![0xa5; KEY_MATRIX_LEN * 2];
                let mut matrix = original.clone();
                apply_key_action(&mut matrix, &layout, index, KeyAction::Multimedia(action));
                assert_eq!((matrix[led], matrix[KEY_MATRIX_LEN + led]), (3, expected_code));
                assert_eq!(decode_key_action(&matrix, &layout, index), KeyAction::Multimedia(action));
                for offset in 0..matrix.len() {
                    if offset != led && offset != KEY_MATRIX_LEN + led { assert_eq!(matrix[offset], original[offset]); }
                }
            }
        }
    }

    #[test]
    fn unsupported_type_three_codes_are_preserved_as_opaque_bindings() {
        let (layout, index) = layout_and_index();
        let led = layout.led_index_for_matrix_key("A").unwrap();
        for code in 8..=255u8 {
            let mut matrix = layout.device.buttoninfo_default.clone();
            matrix[led] = 3;
            matrix[KEY_MATRIX_LEN + led] = code;
            let original = matrix.clone();
            let action = decode_key_action(&matrix, &layout, index);
            assert_eq!(action, KeyAction::Default);
            apply_key_action(&mut matrix, &layout, index, action);
            assert_eq!(matrix, original);
        }
    }

    #[test]
    fn media_assignment_rejects_bad_matrix_lengths_and_key_indices_without_mutation() {
        let (layout, index) = layout_and_index();
        for length in [0, 120, 239, 241] {
            let mut matrix = vec![0xa5; length];
            let original = matrix.clone();
            apply_key_action(&mut matrix, &layout, index, KeyAction::Multimedia(MediaAction::PlayPause));
            assert_eq!(matrix, original);
        }
        let mut matrix = layout.device.buttoninfo_default.clone();
        let original = matrix.clone();
        for index in [layout.device.matrix_key_buttons.len(), usize::MAX] {
            apply_key_action(&mut matrix, &layout, index, KeyAction::Multimedia(MediaAction::PlayPause));
            assert_eq!(matrix, original);
        }
    }

    #[test]
    fn ordinary_key_has_no_modifiers() {
        let (layout, index) = layout_and_index();
        let action = KeyAction::HidKey { code: 4, ctrl: false, shift: false, alt: false, win: false };
        let mut matrix = layout.device.buttoninfo_default.clone();
        apply_key_action(&mut matrix, &layout, index, action);
        assert_eq!(decode_key_action(&matrix, &layout, index), action);
    }

    #[test]
    fn every_supported_modifier_combination_round_trips() {
        let (layout, index) = layout_and_index();
        for modifiers in 1..16u8 {
            let action = KeyAction::HidKey {
                code: 8,
                ctrl: modifiers & 1 != 0,
                shift: modifiers & 2 != 0,
                alt: modifiers & 4 != 0,
                win: modifiers & 8 != 0,
            };
            let mut matrix = layout.device.buttoninfo_default.clone();
            apply_key_action(&mut matrix, &layout, index, action);
            assert_eq!(decode_key_action(&matrix, &layout, index), action, "modifier bits {modifiers}");
        }
    }

    #[test]
    fn editing_one_key_preserves_every_other_byte() {
        let (layout, index) = layout_and_index();
        let led = layout.led_index_for_matrix_key("A").unwrap();
        let original = vec![0xa5; KEY_MATRIX_LEN * 2];
        let mut matrix = original.clone();
        apply_key_action(&mut matrix, &layout, index, KeyAction::Disabled);
        assert_eq!(decode_key_action(&matrix, &layout, index), KeyAction::Disabled);
        for offset in 0..matrix.len() {
            if offset != led && offset != KEY_MATRIX_LEN + led {
                assert_eq!(matrix[offset], original[offset], "byte {offset}");
            }
        }
    }

    #[test]
    fn unchanged_action_preserves_unknown_binding() {
        let (layout, index) = layout_and_index();
        let original = vec![0xa5; KEY_MATRIX_LEN * 2];
        let mut matrix = original.clone();
        assert_eq!(decode_key_action(&matrix, &layout, index), KeyAction::Default);
        apply_key_action(&mut matrix, &layout, index, KeyAction::Default);
        assert_eq!(matrix, original);
    }

    #[test]
    fn truncated_matrices_are_not_read_or_partially_modified() {
        let (layout, index) = layout_and_index();
        for length in 0..KEY_MATRIX_LEN * 2 {
            let original = vec![0xa5; length];
            let mut matrix = original.clone();
            assert_eq!(decode_key_action(&matrix, &layout, index), KeyAction::Default);
            apply_key_action(&mut matrix, &layout, index, KeyAction::Disabled);
            assert_eq!(matrix, original);
        }
    }

    #[test]
    fn invalid_key_index_is_not_read_or_modified() {
        let (layout, _) = layout_and_index();
        let original = layout.device.buttoninfo_default.clone();
        let mut matrix = original.clone();
        let index = layout.device.matrix_key_buttons.len();
        assert_eq!(decode_key_action(&matrix, &layout, index), KeyAction::Default);
        apply_key_action(&mut matrix, &layout, index, KeyAction::Disabled);
        assert_eq!(matrix, original);
    }
}
