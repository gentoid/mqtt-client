use crate::{
    buffer,
    packet::{
        PacketId, QoS, decode,
        encode::{self, Encode},
    },
    protocol::PacketType,
};

#[cfg(feature = "defmt")]
#[derive(defmt::Format)]
pub(crate) struct Publish<'a> {
    pub(crate) flags: Flags,
    pub(crate) topic: buffer::String<'a>,
    pub(crate) packet_id: Option<PacketId>,
    payload: buffer::Slice<'a>,
}

impl<'a: 'b, 'b> From<Msg<'a>> for Publish<'b> {
    fn from(value: Msg<'a>) -> Self {
        Self {
            flags: Flags {
                dup: false,
                qos: value.qos,
                retain: value.retain,
            },
            topic: buffer::String::from(value.topic),
            packet_id: None,
            payload: buffer::Slice::from(value.payload),
        }
    }
}

pub struct Msg<'a> {
    pub qos: QoS,
    pub retain: bool,
    pub topic: &'a str,
    pub payload: &'a [u8],
}

#[cfg(feature = "defmt")]
#[derive(defmt::Format)]
pub(crate) struct Flags {
    pub(crate) dup: bool,
    pub(crate) qos: QoS,
    retain: bool,
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

impl<'a> Publish<'a> {
    pub(crate) fn decode(cursor: &mut decode::Cursor<'a>, flags: u8) -> Result<Self, crate::Error> {
        let flags = Flags::try_from(flags)?;
        let topic = buffer::String::from(buffer::Slice::from(cursor.read_binary()?));

        let packet_id = if let QoS::AtMostOnce = flags.qos {
            None
        } else {
            Some(PacketId::decode(cursor)?)
        };

        let payload = buffer::Slice::from(cursor.read_bytes(cursor.remaining())?);

        Ok(Self {
            flags,
            topic,
            packet_id,
            payload,
        })
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
