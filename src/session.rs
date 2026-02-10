use heapless::Vec;

use crate::{
    incoming,
    packet::{
        Packet, PacketId, QoS,
        connect::{self, ConnAck},
        publish,
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
    Received(publish::Publish<'a>),
    Subscribed,
    SubscribeFailed,
    Unsubscribed,
    Published,
    Disconnected,
}

#[derive(Clone, PartialEq)]
enum SubState {
    New,
    Pending(PacketId),
    Active,
    UnsubPending(PacketId),
    Failed,
}

#[derive(Clone)]
pub(crate) struct Subscription<'s> {
    pub(crate) topic: &'s str,
    pub(crate) qos: QoS,
    state: SubState,
}

impl<'a> From<subscribe::Options<'a>> for Subscription<'a> {
    fn from(value: subscribe::Options<'a>) -> Self {
        Self {
            topic: value.topic,
            qos: value.qos.unwrap_or_default(),
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
    pub_inflight_in: incoming::Publish<N_PUB_IN>,
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
            pub_inflight_in: incoming::Publish::new(),
        }
    }

    pub(crate) fn connect(
        &'s mut self,
        opts: connect::Options<'s>,
    ) -> Result<Packet<'s>, crate::Error> {
        self.ensure_state(State::Disconnected)?;

        self.state = State::Connecting;
        self.ping_outstanding = false;
        self.session_present = false;

        self.pool.clear();

        if opts.clean_session {
            self.subscriptions.clear();
        }

        let packet = connect::Connect::from(opts);

        Ok(Packet::Connect(packet))
    }

    pub(crate) fn on_connack(&mut self, packet: &ConnAck) -> Result<Action<'_>, crate::Error> {
        self.ensure_state(State::Connecting)?;

        self.state = State::Connected;
        self.session_present = packet.session_present;

        self.pool.clear();

        if !packet.session_present {
            self.subscriptions.clear();
        }

        Ok(Action::Event(Event::Connected))
    }

    pub(crate) fn publish<'a>(
        &mut self,
        msg: publish::Msg<'a>,
    ) -> Result<Packet<'a>, crate::Error> {
        self.ensure_state(State::Connected)?;

        let qos = msg.qos;
        let mut packet = publish::Publish::from(msg);

        match qos {
            QoS::AtMostOnce => Ok(Packet::Publish(packet)),
            QoS::AtLeastOnce | QoS::ExactlyOnce => {
                let packet_id = self.pool.next_pub_id(qos == QoS::AtLeastOnce)?;
                packet.packet_id = Some(packet_id);

                Ok(Packet::Publish(packet))
            }
        }
    }

    pub(crate) fn subscribe(
        &mut self,
        opts: subscribe::Options<'s>,
    ) -> Result<Option<Packet<'s>>, crate::Error> {
        self.ensure_state(State::Connected)?;

        if let Some(existing) = self
            .subscriptions
            .iter_mut()
            .find(|s| s.topic == opts.topic)
        {
            match existing.state {
                SubState::Active | SubState::Pending(_) => return Ok(None),
                SubState::New | SubState::Failed => {
                    existing.qos = opts.qos.unwrap_or_default();
                }
                SubState::UnsubPending(_) => return Err(crate::Error::ProtocolViolation),
            };

            let id = self.pool.next_sub_id()?;
            existing.state = SubState::Pending(id);
            let packet = Subscribe::single(id, existing.clone());

            return Ok(Some(Packet::Subscribe(packet)));
        }

        let id = self.pool.next_sub_id()?;
        let sub = Subscription::from(opts);

        self.subscriptions
            .push(sub.clone())
            .map_err(|_| crate::Error::SubVectorIsFull)?;
        let packet = Subscribe::single(id, sub);

        Ok(Some(Packet::Subscribe(packet)))
    }

    pub(crate) fn unsubscribe<'a>(&mut self, topic: &'a str) -> Result<Action<'a>, crate::Error> {
        self.ensure_state(State::Connected)?;

        let sub = self
            .subscriptions
            .iter_mut()
            .find(|sub| sub.topic == topic)
            .ok_or(crate::Error::WrongTopicToUnsubscribe)?;

        match sub.state {
            SubState::Active => {}
            SubState::UnsubPending(_) => return Ok(Action::Nothing),
            _ => return Err(crate::Error::ProtocolViolation),
        }

        let packet_id = self.pool.next_unsub_id()?;

        sub.state = SubState::UnsubPending(packet_id);

        let unsub = Unsubscribe::single(packet_id, topic);
        Ok(Action::Send(Packet::Unsubscribe(unsub)))
    }

    pub(crate) fn disconnect(&mut self) -> Option<Packet<'_>> {
        if self.state == State::Disconnected {
            return None;
        }

        self.state = State::Disconnected;
        self.pool.clear();
        self.pub_inflight_in.clear();
        self.ping_outstanding = false;

        if !self.session_present {
            self.subscriptions.clear();
        }

        Some(Packet::Disconnect)
    }

    pub(crate) fn ping(&mut self) -> Result<Packet<'_>, crate::Error> {
        self.ensure_state(State::Connected)?;

        if self.ping_outstanding {
            // @note or maybe ignore the request
            return Err(crate::Error::PingOutstanding);
        }

        self.ping_outstanding = true;
        Ok(Packet::PingReq)
    }

    fn ensure_state(&self, state: State) -> Result<(), crate::Error> {
        if self.state != state {
            return Err(crate::Error::ProtocolViolation);
        }

        Ok(())
    }

    pub(crate) fn on_publish<'a>(
        &mut self,
        packet: publish::Publish<'a>,
    ) -> Result<Action<'a>, crate::Error> {
        self.ensure_state(State::Connected)?;

        if packet.flags.dup && packet.flags.qos == QoS::AtMostOnce {
            return Err(crate::Error::ProtocolViolation);
        }

        let _ = self
            .subscriptions
            .iter()
            .find(|sub| sub.state == SubState::Active && sub.topic == packet.topic)
            .ok_or(crate::Error::Unsubscribed)?;

        match packet.flags.qos {
            QoS::AtMostOnce => Ok(Action::Event(Event::Received(packet))),
            QoS::AtLeastOnce => {
                let id = packet.packet_id.ok_or(crate::Error::ProtocolViolation)?;
                self.pub_inflight_in.track(&id, true)?;

                Ok(Action::Send(Packet::PubAck(id)))
            }
            QoS::ExactlyOnce => {
                let id = packet.packet_id.ok_or(crate::Error::ProtocolViolation)?;
                self.pub_inflight_in.track(&id, false)?;

                Ok(Action::Send(Packet::PubRec(id)))
            }
        }
    }

    pub(crate) fn on_puback(&mut self, packet_id: &PacketId) -> Result<Action<'_>, crate::Error> {
        self.ensure_state(State::Connected)?;
        self.pool.release_pub_id(packet_id, true)?;

        Ok(Action::Event(Event::Published))
    }

    pub(crate) fn on_pubrec(&mut self, packet_id: &PacketId) -> Result<Action<'_>, crate::Error> {
        self.ensure_state(State::Connected)?;
        self.pool.set_pubrel(packet_id)?;

        Ok(Action::Send(Packet::PubRel(*packet_id)))
    }

    pub(crate) fn on_pubrel(&mut self, packet_id: &PacketId) -> Result<Action<'_>, crate::Error> {
        self.ensure_state(State::Connected)?;
        self.pub_inflight_in.mark_complete(packet_id)?;

        Ok(Action::Send(Packet::PubComp(*packet_id)))
    }

    pub(crate) fn on_pubcomp(&mut self, packet_id: &PacketId) -> Result<Action<'_>, crate::Error> {
        self.ensure_state(State::Connected)?;
        self.pool.release_pub_id(packet_id, false)?;

        Ok(Action::Event(Event::Published))
    }

    pub(crate) fn on_suback(&mut self, packet: &SubAck<1>) -> Result<Action<'_>, crate::Error> {
        self.ensure_state(State::Connected)?;
        self.pool.release_sub_id(&packet.packet_id)?;

        if packet.return_codes.is_empty() {
            return Err(crate::Error::ProtocolViolation);
        }

        if packet.return_codes.len() > 1 {
            return Err(crate::Error::UnsupportedIncomingPacket);
        }

        if self
            .subscriptions
            .iter()
            .filter(|sub| sub.state == SubState::Pending(packet.packet_id))
            .count()
            != 1
        {
            return Err(crate::Error::ProtocolViolation);
        }

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
                sub.state = SubState::Failed;
                Ok(Action::Event(Event::SubscribeFailed))
            }
        }
    }

    pub(crate) fn on_unsuback(&mut self, packet_id: &PacketId) -> Result<Action<'_>, crate::Error> {
        self.ensure_state(State::Connected)?;

        self.pool.release_unsub_id(packet_id)?;

        let removed = {
            let before = self.subscriptions.len();
            self.subscriptions
                .retain(|sub| sub.state != SubState::UnsubPending(*packet_id));
            before - self.subscriptions.len()
        };

        if removed != 1 {
            return Err(crate::Error::ProtocolViolation);
        }

        Ok(Action::Event(Event::Unsubscribed))
    }

    pub(crate) fn on_pingreq(&self) -> Result<Action<'_>, crate::Error> {
        self.ensure_state(State::Connected)?;
        Ok(Action::Send(Packet::PingResp))
    }

    pub(crate) fn on_pingresp(&mut self) -> Result<Action<'_>, crate::Error> {
        self.ensure_state(State::Connected)?;
        self.ping_outstanding = false;
        Ok(Action::Nothing)
    }

    pub(crate) fn on_disconnect(&mut self) -> Action<'_> {
        if self.state == State::Disconnected {
            return Action::Nothing;
        }

        self.state = State::Disconnected;
        self.pool.clear();
        self.pub_inflight_in.clear();
        self.ping_outstanding = false;

        if !self.session_present {
            self.subscriptions.clear();
        }

        Action::Event(Event::Disconnected)
    }
}
