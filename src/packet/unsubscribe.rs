use heapless::Vec;

use crate::{
    packet::{
        Packet, PacketId, decode::{self, Decode},
        encode::{self, Encode, is_full},
    },
    protocol::{FixedHeader, PacketType},
};

pub struct Unsubscribe<'a, const N: usize = 16> {
    pub packet_id: PacketId,
    pub topics: Vec<&'a str, N>,
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

impl<'buf, const P: usize> decode::DecodePacket<'buf> for Unsubscribe<'buf, P> {
    fn decode<'cursor>(
        flags: u8,
        cursor: &'cursor mut decode::Cursor<'buf>,
    ) -> Result<Self, crate::Error> {
        let packet_id = PacketId::decode(cursor)?;

        let mut topics = Vec::new();

        while !cursor.is_empty() {
            let topic = cursor.read_utf8()?;
            topics.push(topic).map_err(|_| crate::Error::VectorIsFull)?;
        }

        if topics.is_empty() {
            return Err(crate::Error::MalformedPacket);
        }

        Ok(Unsubscribe { packet_id, topics })
    }
}
