use crate::{packet::{
    PacketId, QoS, decode,
    encode::{self, is_full},
}, protocol::FixedHeader};

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

impl<'a> encode::Encode for Publish<'a> {
    fn encode(&self, cursor: &mut encode::Cursor) -> Result<(), crate::Error> {
        self.topic.encode(cursor)?;
        self.packet_id.map(|id| id.0).unwrap_or(0).encode(cursor)?;
        self.payload.encode(cursor)?;

        Ok(())
    }
}

impl<'buf> decode::Decode<'buf> for Publish<'buf> {
    fn decode<'cursor>(
        header: &FixedHeader,
        cursor: &'cursor mut decode::Cursor<'buf>,
    ) -> Result<Self, crate::Error> {
        let flags = Flags::try_from(header.flags)?;

        let mut offset = 0;

        // // @todo this cannot be a topic filter unlike subscribe, so maybe chec for allowed chars
        let topic = cursor.read_utf8()?;

        let packet_id = if let QoS::AtMostOnce = flags.qos {
            None
        } else {
            let packet_id = PacketId::decode(header, cursor)?;

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
    use crate::{packet::decode::Decode, protocol::{FixedHeader, PacketType}};

    use super::*;

    #[test]
    fn parse_simple_packet() {
        let flags = 0b0000_0000;
        let body = [
            0x00, 0x05, b't', b'o', b'p', b'i', b'c', b'p', b'a', b'y', b'l', b'o', b'a', b'd',
        ];
        let mut cursor = decode::Cursor::new(&body);
        let header = FixedHeader {
            flags,
            remaining_len: 0,
            packet_type: PacketType::Publish,
        };

        let packet = Publish::decode(&header, &mut cursor).unwrap();

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
