use heapless::Vec;

use crate::packet::{PacketId, parse_packet_id, parse_utf8_str};


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
        topics.push(topic).map_err(|_| crate::Error::TooSmallHeaplessVector)?;
    }

    if topics.is_empty() {
        return Err(crate::Error::MalformedPacket);
    }

    Ok(Unsubscribe { packet_id, topics })
}
