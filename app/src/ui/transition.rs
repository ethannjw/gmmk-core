#[derive(Clone)]
pub(super) struct Transition<const CHANNELS: usize, const MILLIS: u16 = 300> {
    start: [f64; CHANNELS],
    end: [f64; CHANNELS],
    reversing_start: [f64; CHANNELS],
    started: f64,
    shortening: f64,
}

impl<const CHANNELS: usize, const MILLIS: u16> Transition<CHANNELS, MILLIS> {
    pub(super) fn settled(value: [f64; CHANNELS]) -> Self {
        Self { start: value, end: value, reversing_start: value, started: 0.0, shortening: 1.0 }
    }

    pub(super) fn running(&self, now: f64) -> bool {
        self.start != self.end && now < self.started + f64::from(MILLIS) / 1000.0 * self.shortening
    }

    fn progress(&self, now: f64) -> f64 {
        if !self.running(now) { return 1.0; }
        css_ease(((now - self.started) / (f64::from(MILLIS) / 1000.0 * self.shortening)).clamp(0.0, 1.0))
    }

    fn value(&self, progress: f64) -> [f64; CHANNELS] {
        if progress == 1.0 { return self.end; }
        std::array::from_fn(|channel| self.start[channel] + (self.end[channel] - self.start[channel]) * progress)
    }

    pub(super) fn sample(&mut self, target: [f64; CHANNELS], now: f64) -> [f64; CHANNELS] {
        let progress = self.progress(now);
        let current = self.value(progress);
        if target != self.end {
            if current == target {
                *self = Self::settled(target);
            } else {
                let reversing = self.running(now) && target == self.reversing_start;
                self.shortening = if reversing {
                    (progress * self.shortening + 1.0 - self.shortening).abs().clamp(0.0, 1.0)
                } else { 1.0 };
                self.reversing_start = if reversing { self.end } else { current };
                self.start = current;
                self.end = target;
                self.started = now;
            }
        }
        self.value(self.progress(now))
    }
}

fn css_ease(progress: f64) -> f64 {
    if progress <= 0.0 { return 0.0; }
    if progress >= 1.0 { return 1.0; }
    let mut lower = 0.0;
    let mut upper = 1.0;
    for _ in 0..30 {
        let parameter = (lower + upper) * 0.5;
        let horizontal = parameter * (0.75 + parameter * (-0.75 + parameter));
        if horizontal < progress { lower = parameter; } else { upper = parameter; }
    }
    let parameter = (lower + upper) * 0.5;
    parameter * (0.3 + parameter * (2.4 - 1.7 * parameter))
}
