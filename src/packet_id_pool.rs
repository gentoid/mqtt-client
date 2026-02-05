use crate::packet::PacketId;

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

pub(crate) struct PacketIdPool<const N_PUB_OUT: usize, const N_SUB: usize> {
    in_flight_pub: [Option<PubInFlight>; N_PUB_OUT],
    in_flight_sub: [u16; N_SUB],
    in_flight_unsub: [u16; N_SUB],
    next_id: u16,
}

impl<const N_PUB_OUT: usize, const N_SUB: usize> PacketIdPool<N_PUB_OUT, N_SUB> {
    pub(crate) fn new() -> Self {
        Self {
            in_flight_pub: [const { None }; N_PUB_OUT],
            in_flight_sub: [0u16; N_SUB],
            in_flight_unsub: [0u16; N_SUB],
            next_id: 1,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.in_flight_pub.fill(None);
        self.in_flight_sub.fill(0);
        self.in_flight_unsub.fill(0);
        self.next_id = 1;
    }

    pub(crate) fn next_pub_id(&mut self, just_ack: bool) -> Result<PacketId, crate::Error> {
        let index = self.in_flight_pub.iter().position(|p| p.is_none());

        if index.is_none() {
            return Err(crate::Error::NoPacketIdAvailable);
        }

        let id = PacketId(self.next_id()?);
        let index = index.unwrap();

        let state = match just_ack {
            true => PubInFlightState::AwaitPubAck,
            false => PubInFlightState::AwaitPubRec,
        };

        self.in_flight_pub[index] = Some(PubInFlight { id, state });

        Ok(id)
    }

    pub(crate) fn next_sub_id(&mut self) -> Result<PacketId, crate::Error> {
        self.next_for(Kind::Sub)
    }

    pub(crate) fn next_unsub_id(&mut self) -> Result<PacketId, crate::Error> {
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

    pub(crate) fn set_pubrel(&mut self, packet_id: &PacketId) -> Result<(), crate::Error> {
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

    pub(crate) fn release_pub_id(
        &mut self,
        packet_id: &PacketId,
        just_ack: bool,
    ) -> Result<(), crate::Error> {
        let compare_pub =
            |publ: &Option<PubInFlight>| publ.as_ref().map(|p| p.id == *packet_id).unwrap_or(false);

        match self.in_flight_pub.iter().position(compare_pub) {
            Some(index) => {
                let entry = self.in_flight_pub[index]
                    .as_ref()
                    .ok_or(crate::Error::ProtocolViolation)?;
                let state = &entry.state;

                match (just_ack, state) {
                    (true, PubInFlightState::AwaitPubAck)
                    | (false, PubInFlightState::AwaitPubComp) => {
                        self.in_flight_pub[index] = None;
                        Ok(())
                    }
                    _ => Err(crate::Error::ProtocolViolation),
                }
            }
            None => Err(crate::Error::ProtocolViolation),
        }
    }

    pub(crate) fn release_sub_id(&mut self, packet_id: &PacketId) -> Result<(), crate::Error> {
        self.release_for(Kind::Sub, packet_id)
    }

    pub(crate) fn release_unsub_id(&mut self, packet_id: &PacketId) -> Result<(), crate::Error> {
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
