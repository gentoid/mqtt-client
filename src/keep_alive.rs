use embedded_time::{Instant, duration, rate};

pub(crate) struct KeepAlive<C: embedded_time::Clock> {
    keep_alive: duration::Generic<C::T>,
    half_keep_alive: duration::Generic<C::T>,
    last_activity: Instant<C>,
    ping_outstanding: bool,
    enabled: bool,
}

impl<C> KeepAlive<C>
where
    C: embedded_time::Clock,
{
    pub(crate) fn try_new(
        clock: &C,
        keep_alive: duration::Generic<C::T>,
    ) -> Result<Self, crate::Error> {
        let enabled = keep_alive.integer() != 0u32.into();
        let half_keep_alive = if enabled {
            duration::Generic::new(
                keep_alive.integer(),
                *keep_alive.scaling_factor() / rate::Fraction::from_integer(2),
            )
        } else {
            keep_alive
        };

        Ok(Self {
            keep_alive,
            half_keep_alive,
            last_activity: clock.try_now().map_err(|_| crate::Error::TimeError)?,
            ping_outstanding: false,
            enabled,
        })
    }

    pub(crate) fn on_send(&mut self, now: Instant<C>) {
        self.last_activity = now;
    }

    pub(crate) fn on_receive(&mut self, now: Instant<C>) {
        self.last_activity = now;
        self.ping_outstanding = false;
    }

    pub(crate) fn should_ping(&mut self, now: Instant<C>) -> Result<bool, crate::Error> {
        if !self.enabled || self.ping_outstanding {
            return Ok(false);
        }

        if self.elapsed(now)? >= self.half_keep_alive {
            self.ping_outstanding = true;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub(crate) fn timed_out(&self, now: Instant<C>) -> Result<bool, crate::Error> {
        if !self.enabled || !self.ping_outstanding {
            return Ok(false);
        }

        Ok(self.elapsed(now)? >= self.keep_alive)
    }

    fn elapsed(&self, now: Instant<C>) -> Result<duration::Generic<C::T>, crate::Error> {
        now.checked_duration_since(&self.last_activity)
            .ok_or(crate::Error::TimeError)
    }
}
