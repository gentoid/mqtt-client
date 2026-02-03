use crate::packet::{PacketId, connect::ConnAck, publish::Publish, subscribe::SubAck};

pub(crate) struct Session {}

impl Session {
    pub(crate) fn new() -> Self {
        Self {}
    }
    // fn next_packet_id(&mut self) -> Result<PacketId, crate::Error> {}

    pub(crate) fn on_publish(&mut self, packet: &Publish) {}
    pub(crate) fn on_connack(&mut self, packet: &ConnAck) {
        todo!()
    }
    pub(crate) fn on_puback(&mut self, packet_id: &PacketId) {}
    pub(crate) fn on_pubrec(&mut self, packet_id: &PacketId) {}
    pub(crate) fn on_pubrel(&mut self, packet_id: &PacketId) {}
    pub(crate) fn on_pubcomp(&mut self, packet_id: &PacketId) {}
    pub(crate) fn on_suback(&mut self, packet: &SubAck<16>) {}
    pub(crate) fn on_unsuback(&mut self, packet_id: &PacketId) {}
    pub(crate) fn on_pingreq(&mut self) {}
    pub(crate) fn on_pingresp(&mut self) {}
}
