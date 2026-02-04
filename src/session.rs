use crate::packet::{Packet, PacketId, connect::ConnAck, publish::Publish, subscribe::SubAck};

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
    ProtocolError,
}

pub enum Event<'a> {
    Connected,
    Received(&'a Publish<'a>),
    Subscribed,
    Unsubscribed,
    Published,
}

pub(crate) struct Session<const N_PUB_IN: usize, const N_PUB_OUT: usize, const N_SUB: usize> {
    state: State,
    session_present: bool,
    //     pub_out:
    //     sub:
    //     pub_in:
    //     packet_ids:
    //     ping_outstanding: bool,
}

impl<const N_PUB_IN: usize, const N_PUB_OUT: usize, const N_SUB: usize>
    Session<N_PUB_IN, N_PUB_OUT, N_SUB>
{
    pub(crate) fn new() -> Self {
        Self {
            state: State::Disconnected,
            session_present: false,
        }
    }
    // fn next_packet_id(&mut self) -> Result<PacketId, crate::Error> {}

    pub(crate) fn connect(&mut self, opts: ConnectOptions) -> Result<Action, crate::Error> {
        todo!()
    }

    pub(crate) fn publish<'a>(
        &mut self,
        msg: OutgoingPublish<'a>,
    ) -> Result<Action<'a>, crate::Error> {
        todo!()
    }

    pub(crate) fn subscribe<'a>(&mut self, sub: Subscribe<'a>) -> Result<Action<'a>, crate::Error> {
        todo!()
    }

    pub(crate) fn unsubscribe<'a>(
        &mut self,
        unsub: Unsubscribe<'a>,
    ) -> Result<Action<'a>, crate::Error> {
        todo!()
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

        // @todo update inflight subs

        Ok(Action::Event(Event::Subscribed))
    }

    pub(crate) fn on_unsuback(&mut self, packet_id: &PacketId) -> Result<Action, crate::Error> {
        if self.state != State::Connected {
            return Err(crate::Error::ProtocolViolation);
        }

        // @todo update current subs
        Ok(Action::Event(Event::Unsubscribed))
    }

    pub(crate) fn on_pingreq(&self) -> Action {
        Action::Send(Packet::PingResp)
    }

    pub(crate) fn on_pingresp(&self) -> Action {
        self.ping_outstanding = false;
        Action::Nothing
    }
}

struct PacketIdPool<const N: usize> {
    in_flight: [u16; N],
    next_id: u16,
}

impl<const N: usize> PacketIdPool<N> {
    fn new() -> Self {
        Self {
            in_flight: [0u16; N],
            next_id: 1,
        }
    }

    fn allocate(&mut self) -> Result<PacketId, crate::Error> {
        let index = self.index(0).ok_or(crate::Error::VectorIsFull)?;

        for _ in 0..u16::MAX {
            let id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1);

            if self.next_id == 0 {
                self.next_id = 1;
            }

            if !self.in_flight.contains(&id) {
                continue;
            }

            self.in_flight[index] = id;
            return Ok(PacketId(id));
        }

        Err(crate::Error::NoPacketIdAvailable)
    }

    fn release(&mut self, packet_id: PacketId) -> Result<(), crate::Error> {
        match self.index(packet_id.0) {
            Some(index) => {
                self.in_flight[index] = 0;
                Ok(())
            }
            None => Err(crate::Error::ProtocolViolation),
        }
    }

    #[inline]
    fn index(&self, looking_for: u16) -> Option<usize> {
        self.in_flight.iter().position(|id| *id == looking_for)
    }
}
