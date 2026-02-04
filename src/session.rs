use heapless::Vec;

use crate::{
    packet::{
        Packet, PacketId, QoS,
        connect::ConnAck,
        publish::Publish,
        subscribe::{self, SubAck, Subscribe},
        unsubscribe::Unsubscribe,
    },
    packet_id_pool::PacketIdPool,
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
    SubscribeFailed,
    Unsubscribed,
    Published,
}

#[derive(PartialEq)]
enum SubState {
    New,
    Pending(PacketId),
    Active,
    UnsubPending(PacketId),
}

struct Subscription<'s> {
    topic: &'s str,
    qos: QoS,
    state: SubState,
}

impl<'a> Subscription<'a> {
    pub fn new(topic: &'a str, qos: Option<QoS>) -> Self {
        Self {
            topic,
            qos: qos.unwrap_or_default(),
            state: SubState::New,
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

    fn ensure_state(&self, state: State) -> Result<(), crate::Error> {
        if self.state != state {
            return Err(crate::Error::ProtocolViolation);
        }

        Ok(())
    }

    pub(crate) fn on_connack(&mut self, packet: &ConnAck) -> Result<Action, crate::Error> {
        self.ensure_state(State::Connecting)?;

        self.state = State::Connected;
        self.session_present = packet.session_present;

        self.pool.clear();

        if !packet.session_present {
            self.subscriptions.clear();
        }

        // @todo re-subscribe

        Ok(Action::Event(Event::Connected))
    }

    pub(crate) fn on_publish<'a>(
        &mut self,
        packet: &'a Publish,
    ) -> Result<Action<'a>, crate::Error> {
        self.publish(State::Connected)?;

        if packet.flags.dup && packet.flags.qos == QoS::AtMostOnce {
            return Err(crate::Error::ProtocolViolation);
        }

        let sub = self
            .subscriptions
            .iter()
            .find(|sub| sub.state == SubState::Active && sub.topic == packet.topic)
            .ok_or(crate::Error::Unsubscribed)?;

        match packet.flags.qos {
            QoS::AtMostOnce => Ok(Action::Event(Event::Received(packet))),
            QoS::AtLeastOnce => {
                let id = match packet.packet_id {
                    Some(id) => id,
                    None => return Err(crate::Error::ProtocolViolation),
                };

                // @todo note somewhere that we've processed this case?
                Ok(Action::Send(Packet::PubAck(id)))
            }
            QoS::ExactlyOnce => {
                let id = match packet.packet_id {
                    Some(id) => id,
                    None => return Err(crate::Error::ProtocolViolation),
                };

                // store id - waiting for PUBCOMP

                Ok(Action::Send(Packet::PubRec(id)))
            }
        }
    }

    pub(crate) fn on_puback(&mut self, packet_id: &PacketId) -> Result<Action, crate::Error> {
        self.ensure_state(State::Connected)?;
        self.pool.release_pub_id(packet_id, QoS::AtLeastOnce)?;

        Ok(Action::Event(Event::Published))
    }

    pub(crate) fn on_pubrec(&mut self, packet_id: &PacketId) -> Result<Action, crate::Error> {
        self.ensure_state(State::Connected)?;

        // @todo update inflight pubs

        Ok(Action::Event(Event::Published))
    }

    pub(crate) fn on_pubrel(&mut self, packet_id: &PacketId) -> Result<Action, crate::Error> {
        self.ensure_state(State::Connected)?;

        // @todo update inflight pubs

        Ok(Action::Event(Event::Published))
    }

    pub(crate) fn on_pubcomp(&mut self, packet_id: &PacketId) -> Result<Action, crate::Error> {
        self.ensure_state(State::Connected)?;

        // @todo update inflight pubs

        Ok(Action::Event(Event::Published))
    }

    pub(crate) fn on_suback(&mut self, packet: &SubAck<16>) -> Result<Action, crate::Error> {
        self.ensure_state(State::Connected)?;

        self.pool.release_sub_id(&packet.packet_id)?;
        if packet.return_codes.is_empty() {
            return Err(crate::Error::ProtocolViolation);
        }

        if packet.return_codes.len() > 1 {
            return Err(crate::Error::UnsupportedIncomingPacket);
        }

        debug_assert_eq!(
            self.subscriptions
                .iter()
                .filter(|s| s.state == SubState::Pending(packet.packet_id))
                .count(),
            1
        );

        let sub = self
            .subscriptions
            .iter_mut()
            .find(|sub| sub.state == SubState::Pending(packet.packet_id))
            .ok_or(crate::Error::ProtocolViolation)?;

        match packet.return_codes[0] {
            subscribe::SubAckReturnCode::SuccessMaxQoS0 => {
                sub.qos = QoS::AtMostOnce;
                sub.state = SubState::Active;
                Ok(Action::Event(Event::Subscribed))
            }
            subscribe::SubAckReturnCode::SuccessMaxQoS1 => {
                sub.qos = QoS::AtLeastOnce;
                sub.state = SubState::Active;
                Ok(Action::Event(Event::Subscribed))
            }
            subscribe::SubAckReturnCode::SuccessMaxQoS2 => {
                sub.qos = QoS::ExactlyOnce;
                sub.state = SubState::Active;
                Ok(Action::Event(Event::Subscribed))
            }
            subscribe::SubAckReturnCode::Failure => {
                sub.state = SubState::New;
                Ok(Action::Event(Event::SubscribeFailed))
            }
        }
    }

    pub(crate) fn on_unsuback(&mut self, packet_id: &PacketId) -> Result<Action, crate::Error> {
        self.ensure_state(State::Connected)?;

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
