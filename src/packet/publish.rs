use crate::{
    packet::{
        PacketId, QoS,
        decode::{self, Decode},
        encode::{self, Encode, is_full},
    },
    protocol::PacketType,
};

pub struct Publish<'a> {
    pub flags: Flags,
    pub topic: &'a str,
    pub packet_id: Option<PacketId>,
    pub payload: &'a [u8],
}

pub struct Flags {
    pub dup: bool,
    pub qos: QoS,
    pub retain: bool,
}

impl TryFrom<u8> for Flags {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let dup = value & 0b1000 == 1;
        let qos = QoS::try_from((value >> 1) & 0b11)?;
        let retain = value & 0b0001 == 1;

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
        self.packet_id.map(|id| id.0).unwrap_or(0).encode(cursor)?;
        self.payload.encode(cursor)?;

        Ok(())
    }

    fn flags(&self) -> u8 {
        (&self.flags).into()
    }

    fn required_space(&self) -> usize {
        self.topic.required_space()
            + self.packet_id.map(|id| id.0).unwrap_or(0).required_space()
            + self.payload.required_space()
    }
}

impl<'buf> decode::DecodePacket<'buf> for Publish<'buf> {
    fn decode<'cursor>(
        flags: u8,
        cursor: &'cursor mut decode::Cursor<'buf>,
    ) -> Result<Self, crate::Error> {
        let flags = Flags::try_from(flags)?;

        let mut offset = 0;

        // // @todo this cannot be a topic filter unlike subscribe, so maybe chec for allowed chars
        let topic = cursor.read_utf8()?;

        let packet_id = if let QoS::AtMostOnce = flags.qos {
            None
        } else {
            let packet_id = PacketId::decode(cursor)?;

            Some(packet_id)
        };

        let payload = cursor.read_bytes(cursor.remaining())?;

        Ok(Publish {
            flags,
            topic,
            packet_id,
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{packet::decode::DecodePacket, protocol::PacketType};

    use super::*;

    #[test]
    fn parse_simple_packet() {
        let flags = 0b0000_0000;
        let body = [
            0x00, 0x05, b't', b'o', b'p', b'i', b'c', b'p', b'a', b'y', b'l', b'o', b'a', b'd',
        ];
        let mut cursor = decode::Cursor::new(&body);
        let packet = Publish::decode(0, &mut cursor).unwrap();

        assert!(matches!(
            packet.flags,
            Flags {
                dup: false,
                qos: QoS::AtMostOnce,
                retain: false
            }
        ));
        assert_eq!(packet.packet_id, None);
        assert_eq!(packet.topic, "topic");
        assert_eq!(packet.payload, "payload".as_bytes());
    }
}
