use heapless::Vec;

use crate::packet::{PacketId, QoS, get_bytes};

pub struct Subscribe<'a, const N: usize = 16> {
    pub packet_id: PacketId,
    pub topics: Vec<Subscription<'a>, N>,
}

pub struct Subscription<'a> {
    pub topic_filter: &'a str,
    pub qos: QoS,
}

pub(super) fn parse<'a, const N: usize>(body: &'a [u8]) -> Result<Subscribe<'a, N>, crate::Error> {
    let mut offset = 0;
    let mut get_bytes = move |len: usize| {
        let (bytes, new_offset) = get_bytes(body, offset, len)?;
        offset = new_offset;
        Ok(bytes)
    };

    let bytes = get_bytes(2)?;
    let packet_id = PacketId::try_from(bytes)?;

    let mut topics = Vec::<Subscription<'a>, N>::new();

    while offset < body.len() {
        let bytes = get_bytes(2)?;
        let len = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;
        let topic_bytes = get_bytes(len)?;
        let topic_filter = core::str::from_utf8(topic_bytes).map_err(|_| crate::Error::InvalidUtf8)?;

        let qos_byte = get_bytes(1)?[0];
        let qos = QoS::try_from(qos_byte)?;

        topics.push(Subscription { topic_filter, qos }).map_err(|_| crate::Error::TooSmallSubscriptionVector)?;
    }
    
    if topics.is_empty() {
        return Err(crate::Error::MalformedPacket);
    }

    Ok(Subscribe { packet_id, topics })
}
