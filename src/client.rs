use embedded_io_async::{Read, Write};

use crate::{buffer, packet::Packet, parser::read_packet, session::Session};

pub struct Client<'c, T, P>
where
    T: Read + Write,
    P: buffer::Provider<'c>,
{
    session: Session,
    transport: T,
    provider: &'c mut P,
}

impl<'c, T, P> Client<'c, T, P>
where
    T: Read + Write,
    P: buffer::Provider<'c>,
{
    pub fn new(transport: T, provider: &'c mut P) -> Self {
        Self {
            transport,
            provider,
            session: Session::new(),
        }
    }

    pub async fn poll<R: Read>(&'c mut self) -> Result<(), crate::Error> {
        let packet = read_packet::<T, P, 16>(&mut self.transport, &mut self.provider).await?;

        match &packet {
            Packet::ConnAck(conn_ack) => self.session.on_connack(conn_ack),
            Packet::Publish(publish) => self.session.on_publish(publish),
            Packet::PubAck(packet_id) => self.session.on_puback(packet_id),
            Packet::PubRec(packet_id) => self.session.on_pubrec(packet_id),
            Packet::PubRel(packet_id) => self.session.on_pubrel(packet_id),
            Packet::PubComp(packet_id) => self.session.on_pubcomp(packet_id),
            Packet::SubAck(sub_ack) => self.session.on_suback(sub_ack),
            Packet::UnsubAck(packet_id) => self.session.on_unsuback(packet_id),
            Packet::PingReq => self.session.on_pingreq(),
            Packet::PingResp => self.session.on_pingresp(),
            _ => {}
        }

        Ok(())
    }
}
