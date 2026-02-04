use heapless::Vec;

use crate::packet::{
    Packet, PacketId, QoS,
    connect::ConnAck,
    publish::Publish,
    subscribe::{self, SubAck, Subscribe},
    unsubscribe::Unsubscribe,
};

#[derive(PartialEq)]
enum State {
    Disconnected,
    Connecting,
    Connected,
}

pub(crate) enum Action<'a> {
    Send(Packet<'a>),
    Event(Event<'a>),
    Nothing,
}

pub enum Event<'a> {
    Connected,
    Received(&'a Publish<'a>),
    Subscribed,
    Unsubscribed,
    Published,
}

struct Subscription<'s> {
    topic: &'s str,
    qos: QoS,
    active: bool,
    unsub_packet_id: Option<PacketId>,
}

impl<'a> Subscription<'a> {
    pub fn new(topic: &'a str, qos: Option<QoS>) -> Self {
        Self {
            topic,
            qos: qos.unwrap_or_default(),
            active: false,
            unsub_packet_id: None,
        }
    }
}

pub(crate) struct Session<'s, const N_PUB_IN: usize, const N_PUB_OUT: usize, const N_SUB: usize> {
    state: State,
    session_present: bool,
    ping_outstanding: bool,
    pool: PacketIdPool<N_PUB_OUT, N_SUB>,
    subscriptions: Vec<Subscription<'s>, N_SUB>,
}

impl<'s, const N_PUB_IN: usize, const N_PUB_OUT: usize, const N_SUB: usize>
    Session<'s, N_PUB_IN, N_PUB_OUT, N_SUB>
{
    pub(crate) fn new() -> Self {
        Self {
            state: State::Disconnected,
            session_present: false,
            ping_outstanding: false,
            pool: PacketIdPool::new(),
            subscriptions: Vec::new(),
        }
    }

    pub(crate) fn connect(&mut self, opts: ConnectOptions) -> Result<Action, crate::Error> {
        todo!()
    }

    pub(crate) fn publish<'a>(
        &mut self,
        msg: OutgoingPublish<'a>,
    ) -> Result<Action<'a>, crate::Error> {
        todo!()
    }

    pub(crate) fn subscribe<'a>(
        &mut self,
        sub: Subscription<'a>,
    ) -> Result<Action<'a>, crate::Error> {
        let id = self.pool.next_sub_id()?;
        self.subscriptions
            .push(sub)
            .map_err(|_| crate::Error::SubVectorIsFull)?;

        let mut topics: Vec<subscribe::Subscription<'a>, 16> = Vec::new();
        topics.push(subscribe::Subscription {
            topic_filter: sub.topic,
            qos: sub.qos,
        })?;

        Ok(Action::Send(Packet::Subscribe(Subscribe {
            packet_id: id,
            topics,
        })))
    }

    pub(crate) fn unsubscribe<'a>(
        &mut self,
        unsub_topic: &'a str,
    ) -> Result<Action<'a>, crate::Error> {
        let mut sub = self
            .subscriptions
            .iter()
            .find(|sub| sub.topic == unsub_topic)
            .ok_or(crate::Error::WrongTopicToUnsubscribe)?;
        let packet_id = self.pool.next_unsub_id()?;

        sub.unsub_packet_id = Some(packet_id);
        Ok(Action::Send(Packet::Unsubscribe(Unsubscribe {
            packet_id,
            topics: (),
        })))
    }

    pub(crate) fn disconnect(&mut self) -> Action {
        todo!()
    }

    pub(crate) fn ping(&mut self) -> Action {
        if self.ping_outstanding {
            // @todo is it a protocol error?
        }
        self.ping_outstanding = true;
        Action::Send(Packet::PingReq)
    }

    pub(crate) fn on_connack(&mut self, packet: &ConnAck) -> Result<Action, crate::Error> {
        if self.state != State::Connecting {
            return Err(crate::Error::ProtocolViolation);
        }

        self.state = State::Connected;
        self.session_present = packet.session_present;

        if !packet.session_present {
            self.pool.clear();
        }

        // @todo process queued pubs (QoS > 0)
        // @todo process queued subs

        Ok(Action::Event(Event::Connected))
    }

    pub(crate) fn on_publish<'a>(
        &mut self,
        packet: &'a Publish,
    ) -> Result<Action<'a>, crate::Error> {
        if self.state != State::Connected {
            return Err(crate::Error::ProtocolViolation);
        }

        // @todo prepare response?

        Ok(Action::Event(Event::Received(packet)))
    }

    pub(crate) fn on_puback(&mut self, packet_id: &PacketId) -> Result<Action, crate::Error> {
        if self.state != State::Connected {
            return Err(crate::Error::ProtocolViolation);
        }

        // @todo update inflight pubs

        Ok(Action::Event(Event::Published))
    }

    pub(crate) fn on_pubrec(&mut self, packet_id: &PacketId) -> Result<Action, crate::Error> {
        if self.state != State::Connected {
            return Err(crate::Error::ProtocolViolation);
        }

        // @todo update inflight pubs

        Ok(Action::Event(Event::Published))
    }

    pub(crate) fn on_pubrel(&mut self, packet_id: &PacketId) -> Result<Action, crate::Error> {
        if self.state != State::Connected {
            return Err(crate::Error::ProtocolViolation);
        }

        // @todo update inflight pubs

        Ok(Action::Event(Event::Published))
    }

    pub(crate) fn on_pubcomp(&mut self, packet_id: &PacketId) -> Result<Action, crate::Error> {
        if self.state != State::Connected {
            return Err(crate::Error::ProtocolViolation);
        }

        // @todo update inflight pubs

        Ok(Action::Event(Event::Published))
    }

    pub(crate) fn on_suback(&mut self, packet: &SubAck<16>) -> Result<Action, crate::Error> {
        if self.state != State::Connected {
            return Err(crate::Error::ProtocolViolation);
        }

        self.pool.release_sub_id(&packet.packet_id)?;
        Ok(Action::Event(Event::Subscribed))
    }

    pub(crate) fn on_unsuback(&mut self, packet_id: &PacketId) -> Result<Action, crate::Error> {
        if self.state != State::Connected {
            return Err(crate::Error::ProtocolViolation);
        }

        self.pool.release_unsub_id(packet_id)?;
        self.subscriptions
            .retain(|sub| sub.unsub_packet_id != Some(*packet_id));

        Ok(Action::Event(Event::Unsubscribed))
    }

    pub(crate) fn on_pingreq(&self) -> Action {
        Action::Send(Packet::PingResp)
    }

    pub(crate) fn on_pingresp(&mut self) -> Action {
        self.ping_outstanding = false;
        Action::Nothing
    }
}

enum Kind {
    Sub,
    Unsub,
}

#[derive(Clone, PartialEq)]
enum PubInFlightState {
    AwaitPubAck,
    AwaitPubRec,
    AwaitPubComp,
}

#[derive(Clone)]
struct PubInFlight {
    id: PacketId,
    state: PubInFlightState,
}

struct PacketIdPool<const N_PUB_OUT: usize, const N_SUB: usize> {
    in_flight_pub: [Option<PubInFlight>; N_PUB_OUT],
    in_flight_sub: [u16; N_SUB],
    in_flight_unsub: [u16; N_SUB],
    next_id: u16,
}

impl<const N_PUB_OUT: usize, const N_SUB: usize> PacketIdPool<N_PUB_OUT, N_SUB> {
    fn new() -> Self {
        Self {
            in_flight_pub: [const { None }; N_PUB_OUT],
            in_flight_sub: [0u16; N_SUB],
            in_flight_unsub: [0u16; N_SUB],
            next_id: 1,
        }
    }

    fn clear(&mut self) {
        self.in_flight_pub.fill(None);
        self.in_flight_sub.fill(0);
        self.in_flight_unsub.fill(0);
        self.next_id = 1;
    }

    fn next_pub_id(&mut self, qos: QoS) -> Result<PacketId, crate::Error> {
        let index = self.in_flight_pub.iter().position(|p| p.is_none());

        if index.is_none() {
            return Err(crate::Error::NoPacketIdAvailable);
        }

        let id = PacketId(self.next_id()?);
        let index = index.unwrap();

        let state = match qos {
            QoS::AtMostOnce => return Err(crate::Error::ProtocolViolation),
            QoS::AtLeastOnce => PubInFlightState::AwaitPubAck,
            QoS::ExactlyOnce => PubInFlightState::AwaitPubRec,
        };

        self.in_flight_pub[index] = Some(PubInFlight { id, state });

        Ok(id)
    }

    fn next_sub_id(&mut self) -> Result<PacketId, crate::Error> {
        self.next_for(Kind::Sub)
    }

    fn next_unsub_id(&mut self) -> Result<PacketId, crate::Error> {
        self.next_for(Kind::Unsub)
    }

    fn next_for(&mut self, kind: Kind) -> Result<PacketId, crate::Error> {
        let index = self.array_mut(&kind).iter().position(|id| *id == 0);

        if index.is_none() {
            return Err(crate::Error::NoPacketIdAvailable);
        }

        let id = self.next_id()?;
        let index = index.unwrap();

        let array = self.array_mut(&kind);
        array[index] = id;

        Ok(PacketId(id))
    }

    fn next_id(&mut self) -> Result<u16, crate::Error> {
        for _ in 0..u16::MAX {
            let id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1);

            if self.next_id == 0 {
                self.next_id = 1;
            }

            if self.contains(id) {
                continue;
            }

            return Ok(id);
        }

        Err(crate::Error::NoPacketIdAvailable)
    }

    fn set_pubrel(&mut self, packet_id: &PacketId) -> Result<(), crate::Error> {
        let index = self
            .in_flight_pub
            .iter()
            .position(|p| p.as_ref().map(|p| &p.id) == Some(packet_id))
            .ok_or(crate::Error::ProtocolViolation)?;

        let publ = self.in_flight_pub[index]
            .as_mut()
            .ok_or(crate::Error::ProtocolViolation)?;

        match publ.state {
            PubInFlightState::AwaitPubRec => {
                publ.state = PubInFlightState::AwaitPubComp;
                Ok(())
            }
            PubInFlightState::AwaitPubComp => Ok(()),
            _ => Err(crate::Error::ProtocolViolation),
        }
    }

    fn release_pub_id(&mut self, packet_id: &PacketId, qos: QoS) -> Result<(), crate::Error> {
        let compare_pub =
            |publ: &Option<PubInFlight>| publ.as_ref().map(|p| p.id == *packet_id).unwrap_or(false);

        match self.in_flight_pub.iter().position(compare_pub) {
            Some(index) => {
                let publ = self.in_flight_pub[index]
                    .as_ref()
                    .ok_or(crate::Error::ProtocolViolation)?;
                let state = &publ.state;

                if qos == QoS::AtLeastOnce && *state == PubInFlightState::AwaitPubAck {
                    self.in_flight_pub[index] = None;
                    return Ok(());
                }

                if qos == QoS::ExactlyOnce && *state == PubInFlightState::AwaitPubComp {
                    self.in_flight_pub[index] = None;
                    return Ok(());
                }

                Err(crate::Error::ProtocolViolation)
            }
            None => Err(crate::Error::ProtocolViolation),
        }
    }

    fn release_sub_id(&mut self, packet_id: &PacketId) -> Result<(), crate::Error> {
        self.release_for(Kind::Sub, packet_id)
    }

    fn release_unsub_id(&mut self, packet_id: &PacketId) -> Result<(), crate::Error> {
        self.release_for(Kind::Unsub, packet_id)
    }

    fn release_for(&mut self, kind: Kind, packet_id: &PacketId) -> Result<(), crate::Error> {
        let array = self.array_mut(&kind);

        match array.iter().position(|id| *id == packet_id.0) {
            Some(index) => {
                array[index] = 0;
                Ok(())
            }
            None => Err(crate::Error::ProtocolViolation),
        }
    }

    fn array_mut(&mut self, kind: &Kind) -> &mut [u16] {
        match kind {
            Kind::Sub => &mut self.in_flight_sub,
            Kind::Unsub => &mut self.in_flight_unsub,
        }
    }

    #[inline]
    fn contains(&self, looking_for: u16) -> bool {
        let compare_pub = |publ: &&Option<PubInFlight>| {
            publ.as_ref()
                .map(|p| p.id.0 == looking_for)
                .unwrap_or(false)
        };

        self.in_flight_pub.iter().find(compare_pub).is_some()
            || self.in_flight_sub.contains(&looking_for)
            || self.in_flight_unsub.contains(&looking_for)
    }
}
