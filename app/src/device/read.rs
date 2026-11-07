use crate::protocol::POLLING_RATES_HZ;

pub const ONBOARD_READ_UNAVAILABLE: &str = "Onboard settings download is unverified for GMMK 2 96% ANSI. Import a known profile to edit. Factory templates and local files are not backups of the keyboard.";

pub const FIRMWARE_INFO_HELP: &str = "Shows cached USB release metadata, as used by Windows CORE. No feature-report query is sent. Reconnect to refresh the descriptor; this does not download onboard settings or check for updates.";

pub(crate) fn firmware_version_from_release(release_number: u16) -> Option<String> {
    let digits = format!("{release_number:04x}");
    if release_number == 0 || !digits.bytes().all(|digit| digit.is_ascii_digit()) {
        return None;
    }
    if release_number == 0x2000 {
        return Some("0001 (USB recovery marker 2000)".into());
    }
    Some(digits)
}

pub fn parse_ep2_profile_notify(data: &[u8]) -> Option<(u8, u8)> {
    if data.len() < 4 {
        return None;
    }
    if data[0] == 4 && data[1] == 241 {
        if !(1..=3).contains(&data[2]) || !(1..=3).contains(&data[3]) {
            return None;
        }
        let profile = data[2].checked_sub(1)?;
        let layer = data[3].checked_sub(1)?;
        return Some((profile, layer));
    }
    None
}

pub fn polling_hz_from_index(index: u8) -> Option<u32> {
    POLLING_RATES_HZ.get(index as usize).copied()
}

pub(crate) fn read_profile_notifications(
    read: &mut impl FnMut(&mut [u8]) -> Result<usize, String>,
) -> Result<Option<(u8, u8)>, String> {
    let mut buffer = [0; crate::protocol::REPORT_SIZE];
    let mut latest = None;
    for _ in 0..64 {
        let length = read(&mut buffer)?;
        if length == 0 { break; }
        let report = buffer.get(..length).ok_or("Notification length exceeds the input buffer")?;
        if let Some(notification) = parse_ep2_profile_notify(report) {
            latest = Some(notification);
        }
    }
    Ok(latest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_notifications_accept_all_nine_valid_four_byte_targets() {
        for profile in 1..=3 {
            for layer in 1..=3 {
                let notification = parse_ep2_profile_notify(&[4, 241, profile, layer])
                    .expect("the reference consumes four bytes for a profile notification");
                assert_eq!(notification.0, profile - 1);
                assert_eq!(notification.1, layer - 1);
            }
        }
    }

    #[test]
    fn profile_notifications_reject_zero_and_out_of_range_targets() {
        for target in 0..=u8::MAX {
            if (1..=3).contains(&target) { continue; }
            assert!(parse_ep2_profile_notify(&[4, 241, target, 1, 0]).is_none(), "profile {target}");
            assert!(parse_ep2_profile_notify(&[4, 241, 1, target, 0]).is_none(), "layer {target}");
        }
    }

    #[test]
    fn profile_notifications_ignore_other_report_types_and_truncated_data() {
        let notification = [4, 241, 1, 1, 0];
        for length in 0..4 {
            assert!(parse_ep2_profile_notify(&notification[..length]).is_none());
        }
        for value in 0..=u8::MAX {
            if value != 4 { assert!(parse_ep2_profile_notify(&[value, 241, 1, 1, 0]).is_none()); }
            if value != 241 { assert!(parse_ep2_profile_notify(&[4, value, 1, 1, 0]).is_none()); }
        }
    }

    #[test]
    fn profile_notifications_do_not_infer_lighting_from_trailing_bytes() {
        let expected = parse_ep2_profile_notify(&[4, 241, 2, 3, 0]);
        assert!(expected.is_some());
        for trailing in 0..=u8::MAX {
            assert_eq!(parse_ep2_profile_notify(&[4, 241, 2, 3, trailing]), expected);
        }
    }

    #[test]
    fn notification_reader_filters_other_input_and_returns_the_latest_profile_event() {
        let mut reports = std::collections::VecDeque::from([
            vec![1, 0, 4, 0, 0, 0, 0, 0],
            vec![4, 242, 2, 3],
            vec![4, 241, 1, 2],
            vec![4, 241, 0, 3],
            vec![4, 241, 3, 1, 255],
        ]);
        let notification = read_profile_notifications(&mut |buffer| {
            let Some(report) = reports.pop_front() else { return Ok(0) };
            buffer[..report.len()].copy_from_slice(&report);
            Ok(report.len())
        }).unwrap();
        assert_eq!(notification, Some((2, 0)));
        assert!(reports.is_empty());
    }

    #[test]
    fn notification_reader_bounds_work_when_input_never_stops() {
        let mut calls = 0;
        let notification = read_profile_notifications(&mut |buffer| {
            calls += 1;
            buffer[..4].copy_from_slice(&[4, 241, 2, 1]);
            Ok(4)
        }).unwrap();
        assert_eq!(calls, 64);
        assert_eq!(notification, Some((1, 0)));
    }

    #[test]
    fn notification_reader_propagates_failure_without_retry_or_stale_observation() {
        let mut calls = 0;
        let result = read_profile_notifications(&mut |buffer| {
            calls += 1;
            if calls == 1 {
                buffer[..4].copy_from_slice(&[4, 241, 2, 1]);
                Ok(4)
            } else {
                Err("reader disconnected".into())
            }
        });
        assert_eq!(result.unwrap_err(), "reader disconnected");
        assert_eq!(calls, 2);
        assert!(read_profile_notifications(&mut |buffer| Ok(buffer.len() + 1)).is_err());
        assert_eq!(read_profile_notifications(&mut |_| Ok(0)).unwrap(), None);
    }

    #[test]
    fn descriptor_version_preserves_windows_decimal_digits_and_leading_zeroes() {
        for (release, expected) in [(0x0001, "0001"), (0x0112, "0112"), (0x0123, "0123"), (0x9999, "9999")] {
            assert_eq!(firmware_version_from_release(release).as_deref(), Some(expected));
        }
    }

    #[test]
    fn unknown_or_non_decimal_descriptors_do_not_fabricate_a_version() {
        for release in [0, 0x0a12, 0x123a, 0xffff] {
            assert_eq!(firmware_version_from_release(release), None);
        }
    }

    #[test]
    fn windows_recovery_marker_is_visible_without_claiming_up_to_date() {
        assert_eq!(firmware_version_from_release(0x2000).as_deref(), Some("0001 (USB recovery marker 2000)"));
    }
}
