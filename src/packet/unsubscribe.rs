use heapless::Vec;

use crate::{
    buffer,
    packet::{
        PacketId, decode,
        encode::{self, Encode},
    },
    protocol::PacketType,
};

pub struct Unsubscribe<'a, const N: usize = 1> {
    pub packet_id: PacketId,
    pub topics: Vec<buffer::String<'a>, N>,
}

impl<'a, const N: usize> Unsubscribe<'a, N> {
    pub(crate) fn single(packet_id: PacketId, topic: &'a str) -> Self {
        let mut topics = Vec::new();
        topics.push(buffer::String::from(topic));

        Self { packet_id, topics }
    }

    pub(crate) fn decode(cursor: &mut decode::Cursor<'a>) -> Result<Self, crate::Error> {
        let packet_id = PacketId::decode(cursor)?;

        let mut topics = Vec::new();

        while !cursor.is_empty() {
            let topic = buffer::String::from(cursor.read_utf8()?);
            topics.push(topic).map_err(|_| crate::Error::VectorIsFull)?;
        }

        if topics.is_empty() {
            return Err(crate::Error::MalformedPacket);
        }

        Ok(Unsubscribe { packet_id, topics })
    }
}

impl<'a, const P: usize> encode::EncodePacket for &Unsubscribe<'a, P> {
    const PACKET_TYPE: PacketType = PacketType::Unsubscribe;

    fn flags(&self) -> u8 {
        0b0010
    }

    fn required_space(&self) -> usize {
        let mut required = self.packet_id.required_space();

        for topic in &self.topics {
            required += topic.required_space();
        }

        required
    }

    fn encode_body(&self, cursor: &mut encode::Cursor) -> Result<(), crate::Error> {
        self.packet_id.encode(cursor)?;

        for topic in &self.topics {
            topic.encode(cursor)?;
        }

        Ok(())
    }
}

// impl<'buf, P, const N: usize> decode::DecodePacket<'buf, P> for Unsubscribe<'buf, N>
// where
//     P: buffer::Provider<'buf>,
// {
// }
