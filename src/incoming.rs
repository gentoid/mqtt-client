use heapless::Vec;

use crate::packet::PacketId;

#[derive(PartialEq)]
enum PubInState {
    AwaitPubRel,
    Done,
}

struct PubInFlightIn {
    id: PacketId,
    state: PubInState,
}

pub(crate) struct Publish<const N_PUB_IN: usize> {
    cursor: usize,
    pubs: Vec<PubInFlightIn, N_PUB_IN>,
}

impl<const N_PUB_IN: usize> Publish<N_PUB_IN> {
    pub(crate) fn new() -> Self {
        Self {
            cursor: 0,
            pubs: Vec::new(),
        }
    }

    pub(crate) fn track(&mut self, packet_id: &PacketId) -> Result<(), crate::Error> {
        if self.pubs.iter().any(|p| p.id == *packet_id) {
            return Ok(());
        }

        let entry = PubInFlightIn {
            id: *packet_id,
            state: PubInState::AwaitPubRel,
        };

        if !self.pubs.is_full() {
            self.pubs
                .push(entry)
                .map_err(|_| crate::Error::VectorIsFull)?;
            return Ok(());
        }

        for _ in 0..self.pubs.len() {
            if self.pubs[self.cursor].state == PubInState::Done {
                self.pubs[self.cursor] = entry;
                self.shift_cursor();
                return Ok(());
            }

            self.shift_cursor();
        }

        Err(crate::Error::VectorIsFull)
    }

    fn shift_cursor(&mut self) {
        self.cursor += 1;

        if self.cursor >= N_PUB_IN {
            self.cursor = 0;
        }
    }

    pub(crate) fn mark_complete(&mut self, packet_id: &PacketId) -> Result<(), crate::Error> {
        let entry = self
            .pubs
            .iter_mut()
            .find(|p| p.id == *packet_id)
            .ok_or(crate::Error::ProtocolViolation)?;

        if entry.state == PubInState::AwaitPubRel {
            entry.state = PubInState::Done;
        }

        Ok(())
    }
}
