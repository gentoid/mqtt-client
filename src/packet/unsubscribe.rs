use heapless::Vec;

use crate::{
    packet::{
        Packet, PacketId, decode,
        encode::{self, is_full},
    },
    protocol::FixedHeader,
};

pub struct Unsubscribe<'a, const N: usize = 16> {
    pub packet_id: PacketId,
    pub topics: Vec<&'a str, N>,
}

impl<'a, const P: usize> encode::Encode for Unsubscribe<'a, P> {
    fn encode(&self, cursor: &mut encode::Cursor) -> Result<(), crate::Error> {
        self.packet_id.encode(cursor)?;
        // self.topics.encode(cursor)?;

        Ok(())
    }
}

impl<'buf, const P: usize> decode::Decode<'buf> for Unsubscribe<'buf, P> {
    fn decode<'cursor>(
        header: &FixedHeader,
        cursor: &'cursor mut decode::Cursor<'buf>,
    ) -> Result<Self, crate::Error> {
        let packet_id = PacketId::decode(header, cursor)?;

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
