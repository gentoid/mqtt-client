use heapless::Vec;

use crate::packet::{Packet, PacketId, encode::is_full, parse_packet_id, parse_utf8_str};

pub struct Unsubscribe<'a, const N: usize = 16> {
    pub packet_id: PacketId,
    pub topics: Vec<&'a str, N>,
}

pub(super) fn parse<'a>(body: &'a [u8]) -> Result<Unsubscribe<'a>, crate::Error> {
    let mut offset = 0;
    let packet_id = parse_packet_id(body, &mut offset)?;

    let mut topics = Vec::new();

    while offset < body.len() {
        let topic = parse_utf8_str(body, &mut offset)?;
        topics
            .push(topic)
            .map_err(|_| crate::Error::VectorIsFull)?;
    }

    if topics.is_empty() {
        return Err(crate::Error::MalformedPacket);
    }

    Ok(Unsubscribe { packet_id, topics })
}

pub(super) fn encode<const N: usize>(out: &mut heapless::Vec<u8, N>, packet: &Unsubscribe<'_>) -> Result<(), crate::Error> {
    out.extend_from_slice(&packet.packet_id.0.to_be_bytes()).map_err(is_full)?;

    for topic in &packet.topics {
        out.extend_from_slice(topic.as_bytes()).map_err(is_full)?;
    }

    Ok(())
}
