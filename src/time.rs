use embedded_time::{Clock, Instant, rate::Fraction};

// Copied almost 1-to-1 from https://github.com/SimonIT/embassy-embedded-time/blob/main/src/lib.rs
pub struct EmbassyClock {
    start: embassy_time::Instant,
}

impl Default for EmbassyClock {
    fn default() -> Self {
        Self {
            start: embassy_time::Instant::now(),
        }
    }
}

impl Clock for EmbassyClock {
    type T = u64;

    const SCALING_FACTOR: Fraction = Fraction::new(1, 1_000_000);

    fn try_now(&self) -> Result<embedded_time::Instant<Self>, embedded_time::clock::Error> {
        let now = embassy_time::Instant::now();
        let elapsed = now.duration_since(self.start);

        Ok(Instant::new(elapsed.as_micros()))
    }
}

pub struct KeepAlive {}

impl KeepAlive {
    pub fn from_us(value: u64) -> embedded_time::duration::Generic<<EmbassyClock as Clock>::T> {
        embedded_time::duration::Generic::new(value, <EmbassyClock as Clock>::SCALING_FACTOR)
    }

    pub fn from_sec(value: u64) -> embedded_time::duration::Generic<<EmbassyClock as Clock>::T> {
        Self::from_us(value * <EmbassyClock as Clock>::SCALING_FACTOR)
    }
}
