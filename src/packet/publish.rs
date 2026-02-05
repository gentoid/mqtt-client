use crate::{
    buffer,
    packet::{
        PacketId, QoS,
        encode::{self, Encode},
    },
    protocol::PacketType,
};

pub struct Publish<'a> {
    pub flags: Flags,
    pub topic: buffer::String<'a>,
    pub packet_id: Option<PacketId>,
    pub payload: buffer::Slice<'a>,
}

pub struct Flags {
    pub dup: bool,
    pub qos: QoS,
    pub retain: bool,
}

impl TryFrom<u8> for Flags {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let dup = (value & 0b1000) != 0;
        let qos = QoS::try_from((value >> 1) & 0b11)?;
        let retain = (value & 0b0001) != 0;

        if qos == QoS::AtMostOnce && dup {
            return Err(crate::Error::MalformedPacket);
        }

        Ok(Self { dup, qos, retain })
    }
}

impl From<&Flags> for u8 {
    fn from(value: &Flags) -> Self {
        (value.dup as u8) << 3 | (value.qos as u8) << 1 | (value.retain as u8)
    }
}

impl<'a> encode::EncodePacket for &Publish<'a> {
    const PACKET_TYPE: PacketType = PacketType::Publish;

    fn encode_body(&self, cursor: &mut encode::Cursor) -> Result<(), crate::Error> {
        self.topic.encode(cursor)?;
        if let Some(id) = self.packet_id {
            id.0.encode(cursor)?;
        }
        self.payload.encode(cursor)?;

        Ok(())
    }

    fn flags(&self) -> u8 {
        (&self.flags).into()
    }

    fn required_space(&self) -> usize {
        self.topic.required_space()
            + self.packet_id.map(|id| id.0.required_space()).unwrap_or(0)
            + self.payload.required_space()
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    // #[test]
    // fn parse_simple_packet() {
    //     let flags = 0b0000_0000;
    //     let body = [
    //         0x00, 0x05, b't', b'o', b'p', b'i', b'c', b'p', b'a', b'y', b'l', b'o', b'a', b'd',
    //     ];
    //     let mut cursor = decode::Cursor::new(&body);
    //     let mut buf = [0u8; 32];
    //     let mut provider = buffer::Bump::new(&mut buf);
    //     let packet = Publish::decode(&mut cursor, &mut provider, flags).unwrap();

    //     assert!(matches!(
    //         packet.flags,
    //         Flags {
    //             dup: false,
    //             qos: QoS::AtMostOnce,
    //             retain: false
    //         }
    //     ));
    //     assert_eq!(packet.packet_id, None);
    //     assert_eq!(packet.topic, "topic");
    //     assert_eq!(packet.payload, b"payload".as_slice());
    // }
}
