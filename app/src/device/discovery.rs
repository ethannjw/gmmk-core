use std::ffi::CString;

type Observation = Result<Option<CString>, String>;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Change {
    Unchanged,
    Absent,
    Present,
    Failed(String),
}

#[derive(Default)]
pub(super) struct Discovery {
    previous: Option<Observation>,
    paused: bool,
}

impl Discovery {
    pub(super) fn pause(&mut self) { self.paused = true; }

    pub(super) fn retry(&mut self) {
        self.paused = false;
        self.previous = None;
    }

    pub(super) fn observe(&mut self, current: Observation) -> Change {
        if self.paused || self.previous.as_ref() == Some(&current) { return Change::Unchanged; }
        let change = match &current {
            Ok(Some(_)) => Change::Present,
            Ok(None) => Change::Absent,
            Err(error) => Change::Failed(error.clone()),
        };
        self.previous = Some(current);
        change
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn present(path: &str) -> Observation { Ok(Some(CString::new(path).unwrap())) }

    #[test]
    fn discovery_connects_once_per_presence_and_reconnects_after_removal() {
        let mut discovery = Discovery::default();
        assert_eq!(discovery.observe(Ok(None)), Change::Absent);
        assert_eq!(discovery.observe(Ok(None)), Change::Unchanged);
        assert_eq!(discovery.observe(present("keyboard")), Change::Present);
        assert_eq!(discovery.observe(present("keyboard")), Change::Unchanged);
        assert_eq!(discovery.observe(Ok(None)), Change::Absent);
        assert_eq!(discovery.observe(present("keyboard")), Change::Present);
        assert_eq!(discovery.observe(present("replacement")), Change::Present);
    }

    #[test]
    fn discovery_never_retries_a_stable_open_failure_without_explicit_retry() {
        let mut discovery = Discovery::default();
        assert_eq!(discovery.observe(present("denied")), Change::Present);
        for _ in 0..100 {
            assert_eq!(discovery.observe(present("denied")), Change::Unchanged);
        }
        discovery.retry();
        assert_eq!(discovery.observe(present("denied")), Change::Present);
    }

    #[test]
    fn discovery_manual_disconnect_stays_paused_until_retry() {
        let mut discovery = Discovery::default();
        discovery.observe(present("keyboard"));
        discovery.pause();
        assert_eq!(discovery.observe(Ok(None)), Change::Unchanged);
        assert_eq!(discovery.observe(present("keyboard")), Change::Unchanged);
        discovery.retry();
        assert_eq!(discovery.observe(present("keyboard")), Change::Present);
    }

    #[test]
    fn discovery_reports_ambiguity_or_scan_failure_once_and_recovers() {
        let mut discovery = Discovery::default();
        assert_eq!(discovery.observe(Err("ambiguous".into())), Change::Failed("ambiguous".into()));
        assert_eq!(discovery.observe(Err("ambiguous".into())), Change::Unchanged);
        assert_eq!(discovery.observe(present("keyboard")), Change::Present);
    }
}
