use core::ops::Range;

use embedded_io_async::{Read, Write};
use embedded_time::{Instant, duration, rate};
use heapless::Deque;

use crate::{
    packet::{self, Packet, connect, publish},
    parser,
    session::{self, Session},
};

pub struct Client<
    'c,
    C,
    T,
    const N_PUB_IN: usize,
    const N_PUB_OUT: usize,
    const N_SUB: usize,
    const OUT_QUEUE_SIZE: usize,
> where
    T: Read + Write,
    C: embedded_time::Clock,
{
    clock: C,
    transport: T,
    keep_alive: KeepAlive<C>,
    session: Session<'c, N_PUB_IN, N_PUB_OUT, N_SUB>,
    parser: parser::StreamParser<'c>,
    outbox: Outbox<'c, OUT_QUEUE_SIZE>,
}

impl<
    'c,
    C,
    T,
    const N_PUB_IN: usize,
    const N_PUB_OUT: usize,
    const N_SUB: usize,
    const OUT_Q: usize,
> Client<'c, C, T, N_PUB_IN, N_PUB_OUT, N_SUB, OUT_Q>
where
    T: Read + Write,
    C: embedded_time::Clock,
{
    pub fn try_new(
        clock: C,
        keep_alive: duration::Generic<C::T>,
        transport: T,
        rx_buf: &'c mut [u8],
        tx_buf: &'c mut [u8],
    ) -> Result<Self, crate::Error> {
        let keep_alive = KeepAlive::try_new(&clock, keep_alive)?;

        Ok(Self {
            clock,
            transport,
            session: Session::new(),
            keep_alive,
            parser: parser::StreamParser::new(rx_buf),
            outbox: Outbox::new(tx_buf),
        })
    }

    pub fn schedule_connect(&'c mut self, opts: connect::Options<'c>) -> Result<(), crate::Error> {
        let packet = self.session.connect(opts)?;
        self.outbox.enqueue(packet)
    }

    pub fn schedule_disconnect(&mut self) -> Result<(), crate::Error> {
        if let Some(packet) = self.session.disconnect() {
            self.outbox.enqueue(packet)?;
        };

        Ok(())
    }

    pub fn schedule_ping(&mut self) -> Result<(), crate::Error> {
        let packet = self.session.ping()?;
        self.outbox.enqueue(packet)
    }

    pub fn schedule_publish(&mut self, opts: publish::Options<'c>) -> Result<(), crate::Error> {
        let packet = self.session.publish(opts)?;
        self.outbox.enqueue(packet)
    }

    /// High-level poll. Runs timers, then performs one I/O step.
    /// Recommended default for simple loops.
    pub async fn poll<'a>(&'a mut self) -> Result<Option<session::Event<'a>>, crate::Error> {
        self.poll_timers()?;
        self.poll_io().await
    }

    /// Timer-only step. Enqueues PINGREQ/DISCONNECT when needed.
    /// Use when your framework schedules timers separately.
    pub fn poll_timers(&mut self) -> Result<(), crate::Error> {
        let now = self.clock.try_now().map_err(|_| crate::Error::TimeError)?;

        if self.keep_alive.should_ping(now)? {
            self.schedule_ping()?;
        }

        if self.keep_alive.timed_out(now)? {
            self.schedule_disconnect()?;
            // @todo return some status maybe? E.g. enum TimedOut { Yes, No }
            // @todo reconnect
        }

        Ok(())
    }

    /// I/O step. Sends one queued packet if any; otherwise reads and processes one incoming packet.
    pub async fn poll_io<'a>(&'a mut self) -> Result<Option<session::Event<'a>>, crate::Error> {
        let now = self.clock.try_now().map_err(|_| crate::Error::TimeError)?;

        if self.outbox.has_pending() {
            self.outbox.flush_one(&mut self.transport).await?;
            self.keep_alive.update(now);
            return Ok(None);
        }

        let packet = self.parser.read(&mut self.transport).await?;

        self.keep_alive.update(now);

        let action = match packet {
            Packet::ConnAck(conn_ack) => self.session.on_connack(&conn_ack)?,
            Packet::Publish(publish) => self.session.on_publish(publish)?,
            Packet::PubAck(packet_id) => self.session.on_puback(&packet_id)?,
            Packet::PubRec(packet_id) => self.session.on_pubrec(&packet_id)?,
            Packet::PubRel(packet_id) => self.session.on_pubrel(&packet_id)?,
            Packet::PubComp(packet_id) => self.session.on_pubcomp(&packet_id)?,
            Packet::SubAck(sub_ack) => self.session.on_suback(&sub_ack)?,
            Packet::UnsubAck(packet_id) => self.session.on_unsuback(&packet_id)?,
            Packet::PingReq => self.session.on_pingreq()?,
            Packet::PingResp => self.session.on_pingresp()?,
            Packet::Disconnect => self.session.on_disconnect(),
            _ => session::Action::Nothing,
        };

        apply_action(&mut self.outbox, action)
    }
}

fn apply_action<'a, 'b, const Q: usize>(
    tx: &mut Outbox<'b, Q>,
    action: session::Action<'a>,
) -> Result<Option<session::Event<'a>>, crate::Error> {
    match action {
        session::Action::Send(packet) => {
            tx.enqueue(packet)?;
            Ok(None)
        }
        session::Action::Event(event) => Ok(Some(event)),
        session::Action::Nothing => Ok(None),
    }
}

struct Outbox<'a, const QUEUE_SIZE: usize> {
    buf: &'a mut [u8],
    cursor: usize,
    queue: Deque<Range<usize>, QUEUE_SIZE>,
}

impl<'a, const QUEUE_SIZE: usize> Outbox<'a, QUEUE_SIZE> {
    fn new(buf: &'a mut [u8]) -> Self {
        Self {
            buf,
            cursor: 0,
            queue: Deque::new(),
        }
    }

    fn has_pending(&self) -> bool {
        !self.queue.is_empty()
    }

    fn enqueue(&mut self, packet: Packet<'_>) -> Result<(), crate::Error> {
        if self.queue.is_empty() {
            self.cursor = 0;
        }

        let needed = packet.required_space();

        if self.cursor + needed > self.buf.len() {
            return Err(crate::Error::BufferTooSmall);
        }

        let start = self.cursor;
        let end = start + needed;
        let mut cursor = packet::encode::Cursor::new(&mut self.buf[start..end]);
        packet.encode(&mut cursor)?;

        self.queue
            .push_back(start..end)
            .map_err(|_| crate::Error::VectorIsFull)?;
        self.cursor = end;

        Ok(())
    }

    async fn flush_one<T: Write>(&mut self, transport: &mut T) -> Result<(), crate::Error> {
        if let Some(range) = self.queue.pop_front() {
            transport
                .write_all(&self.buf[range])
                .await
                .map_err(|_| crate::Error::TransportError)?;
        }

        self.compact()
    }

    fn compact(&mut self) -> Result<(), crate::Error> {
        let mut cursor = 0;
        for range in &self.queue {
            if range.start < cursor {
                return Err(crate::Error::QueueRangeError);
            }

            if range.start - cursor > 0 {
                self.buf.copy_within(range.clone(), cursor);
            }

            cursor += range.len();
        }

        self.cursor = cursor;

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
