use embedded_io_async::{Read, Write};
use embedded_time::{Instant, duration, rate};

use crate::{
    buffer,
    packet::Packet,
    parser::read_packet,
    session::{self, Session},
};

pub struct Client<'c, C, T, P, const N_PUB_IN: usize, const N_PUB_OUT: usize, const N_SUB: usize>
where
    T: Read + Write,
    P: buffer::Provider<'c>,
    C: embedded_time::Clock,
{
    clock: C,
    transport: T,
    provider: &'c mut P,
    keep_alive: KeepAlive<C>,
    session: Session<'c, N_PUB_IN, N_PUB_OUT, N_SUB>,
}

impl<'c, C, T, P, const N_PUB_IN: usize, const N_PUB_OUT: usize, const N_SUB: usize>
    Client<'c, C, T, P, N_PUB_IN, N_PUB_OUT, N_SUB>
where
    T: Read + Write,
    P: buffer::Provider<'c>,
    C: embedded_time::Clock,
{
    pub fn try_new(
        clock: C,
        keep_alive: duration::Generic<C::T>,
        transport: T,
        provider: &'c mut P,
    ) -> Result<Self, crate::Error> {
        let keep_alive = KeepAlive::try_new(&clock, keep_alive)?;

        Ok(Self {
            clock,
            transport,
            provider,
            session: Session::new(),
            keep_alive,
        })
    }

    pub async fn poll(&'c mut self) -> Result<(), crate::Error> {
        let now = self.clock.try_now().map_err(|_| crate::Error::TimeError)?;
        let packet = read_packet::<T, P, 16>(&mut self.transport, &mut self.provider).await?;

        self.keep_alive.update(now);

        let action = match &packet {
            Packet::ConnAck(conn_ack) => self.session.on_connack(conn_ack)?,
            Packet::Publish(publish) => self.session.on_publish(publish)?,
            Packet::PubAck(packet_id) => self.session.on_puback(packet_id)?,
            Packet::PubRec(packet_id) => self.session.on_pubrec(packet_id)?,
            Packet::PubRel(packet_id) => self.session.on_pubrel(packet_id)?,
            Packet::PubComp(packet_id) => self.session.on_pubcomp(packet_id)?,
            Packet::SubAck(sub_ack) => self.session.on_suback(sub_ack)?,
            Packet::UnsubAck(packet_id) => self.session.on_unsuback(packet_id)?,
            Packet::PingReq => self.session.on_pingreq()?,
            Packet::PingResp => self.session.on_pingresp()?,
            Packet::Disconnect => self.session.on_disconnect(),
            _ => session::Action::Nothing,
        };

        if self.keep_alive.should_ping(now)? {
            // Send Packet::PingReq
        }

        if self.keep_alive.timed_out(now)? {
            return Err(crate::Error::TimedOut);
        }

        Ok(())
    }
}
struct KeepAlive<C: embedded_time::Clock> {
    keep_alive: duration::Generic<C::T>,
    half_keep_alive: duration::Generic<C::T>,
    last_activity: Instant<C>,
}

impl<C> KeepAlive<C>
where
    C: embedded_time::Clock,
{
    fn try_new(clock: &C, keep_alive: duration::Generic<C::T>) -> Result<Self, crate::Error> {
        let half_keep_alive = duration::Generic::new(
            keep_alive.integer(),
            *keep_alive.scaling_factor() * rate::Fraction::from_integer(2),
        );

        Ok(Self {
            keep_alive,
            half_keep_alive,
            last_activity: clock.try_now().map_err(|_| crate::Error::TimeError)?,
            // ping_outstanding: false,
        })
    }

    fn update(&mut self, now: Instant<C>) {
        self.last_activity = now;
        // self.ping_outstanding = false;
    }

    fn should_ping(&mut self, now: Instant<C>) -> Result<bool, crate::Error> {
        if self.elapsed(now)? >= self.half_keep_alive
        /* && !self.ping_outstanding */
        {
            // this changes on receiving PINGRESP
            // self.ping_outstanding = true;
            // self.last_activity = now;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn timed_out(&self, now: Instant<C>) -> Result<bool, crate::Error> {
        // if !self.ping_outstanding {
        //     return Ok(false);
        // }

        Ok(self.elapsed(now)? >= self.keep_alive)
    }

    fn elapsed(&self, now: Instant<C>) -> Result<duration::Generic<C::T>, crate::Error> {
        now.checked_duration_since(&self.last_activity)
            .ok_or(crate::Error::TimeError)
    }
}
