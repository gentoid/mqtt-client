use heapless::Vec;

use crate::{
    buffer,
    packet::{
        PacketId, QoS, decode,
        encode::{self, Encode},
    },
    protocol::PacketType,
    session,
};

pub struct Subscribe<'a, const N: usize = 1> {
    pub packet_id: PacketId,
    pub topics: Vec<Subscription<'a>, N>,
}

impl<'a> Subscribe<'a> {
    pub(crate) fn decode(cursor: &mut decode::Cursor<'a>) -> Result<Self, crate::Error> {
        let packet_id = PacketId::decode(cursor)?;

        let mut topics = Vec::<Subscription<'a>, 1>::new();

        while !cursor.is_empty() {
            let topic_filter = buffer::String::from(cursor.read_utf8()?);
            let qos = QoS::decode(cursor)?;

            topics
                .push(Subscription { topic_filter, qos })
                .map_err(|_| crate::Error::VectorIsFull)?;
        }

        if topics.is_empty() {
            return Err(crate::Error::MalformedPacket);
        }

        Ok(Subscribe { packet_id, topics })
    }

    pub(crate) fn single(packet_id: PacketId, sub: session::Subscription<'a>) -> Self {
        let mut topics = Vec::new();

        topics.push(Subscription {
            topic_filter: buffer::String::from(sub.topic),
            qos: sub.qos,
        });

        Self { packet_id, topics }
    }
}

impl<'a, const P: usize> encode::EncodePacket for &Subscribe<'a, P> {
    const PACKET_TYPE: PacketType = PacketType::Subscribe;

    fn flags(&self) -> u8 {
        0b0010
    }

    fn required_space(&self) -> usize {
        let mut required_space = self.packet_id.required_space();

        for topic in &self.topics {
            required_space += topic.required_space();
        }

        required_space
    }

    fn encode_body(&self, cursor: &mut encode::Cursor) -> Result<(), crate::Error> {
        self.packet_id.encode(cursor)?;

        for topic in &self.topics {
            topic.encode(cursor)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Subscription<'a> {
    pub topic_filter: buffer::String<'a>,
    pub qos: QoS,
}

impl<'a> encode::Encode for Subscription<'a> {
    fn encode(&self, cursor: &mut encode::Cursor) -> Result<(), crate::Error> {
        self.topic_filter.encode(cursor)?;
        self.qos.encode(cursor)
    }

    fn required_space(&self) -> usize {
        self.topic_filter.required_space() + self.qos.required_space()
    }
}

pub struct SubAck<const N: usize = 1> {
    pub(crate) packet_id: PacketId,
    pub return_codes: Vec<SubAckReturnCode, N>,
}

impl<const N: usize> SubAck<N> {
    pub(crate) fn decode(cursor: &mut decode::Cursor<'_>) -> Result<SubAck<N>, crate::Error> {
        let packet_id = PacketId::decode(cursor)?;
        let mut return_codes = Vec::<SubAckReturnCode, N>::new();

        while !cursor.is_empty() {
            let code = SubAckReturnCode::try_from(cursor.read_u8()?)?;
            return_codes
                .push(code)
                .map_err(|_| crate::Error::VectorIsFull)?;
        }

        if return_codes.is_empty() {
            return Err(crate::Error::MalformedPacket);
        }

        Ok(SubAck {
            packet_id,
            return_codes,
        })
    }
}

#[repr(u8)]
pub enum SubAckReturnCode {
    SuccessMaxQoS0 = 0x00,
    SuccessMaxQoS1 = 0x01,
    SuccessMaxQoS2 = 0x02,
    Failure = 0x80,
}

impl TryFrom<u8> for SubAckReturnCode {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let code = match value {
            0x00 => Self::SuccessMaxQoS0,
            0x01 => Self::SuccessMaxQoS1,
            0x02 => Self::SuccessMaxQoS2,
            0x80 => Self::Failure,
            _ => return Err(crate::Error::MalformedPacket),
        };

        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use crate::packet::encode::EncodePacket;

    use super::*;

    // fn parse_suback<const N: usize>(
    //     body: &[u8],
    //     buf: &mut [u8],
    // ) -> Result<SubAck<N>, crate::Error> {
    //     let mut provider = buffer::Bump::new(buf);
    //     SubAck::<N>::decode(&mut decode::Cursor::new(&body), &mut provider, 0)
    // }

    // #[test]
    // fn suback_single_success() {
    //     // packet_id = 16, return code = 1
    //     let body = [0x00, 0x10, 0x01];
    //     let mut buf = [0u8; 16];
    //     let packet = parse_suback::<1>(&body, &mut buf[..]).unwrap();

    //     assert_eq!(packet.packet_id.0, 16);
    //     assert_eq!(packet.return_codes.len(), 1);
    //     assert!(matches!(
    //         packet.return_codes[0],
    //         SubAckReturnCode::SuccessMaxQoS1
    //     ));
    // }

    // #[test]
    // fn suback_invalid_return_code() {
    //     let body = [0x00, 0x10, 0x05];
    //     let mut buf = [0u8; 16];
    //     assert!(parse_suback::<1>(&body, &mut buf[..]).is_err());
    // }

    fn make_subscribe<'a, const N: usize>() -> Subscribe<'a, N> {
        let mut topics: Vec<Subscription, N> = Vec::new();
        topics
            .push(Subscription {
                topic_filter: buffer::String::from("a/b"),
                qos: QoS::AtLeastOnce,
            })
            .unwrap();
        Subscribe {
            packet_id: PacketId(10),
            topics,
        }
    }

    #[test]
    fn encode_subscribe_single_topic() {
        let packet = make_subscribe::<'_, 1>();
        let mut buf = [0u8; 32];
        let mut cursor = encode::Cursor::new(&mut buf);

        (&packet).encode_body(&mut cursor).unwrap();

        let encoded = cursor.written();

        assert_eq!(encoded, &[0x00, 0x0A, 0x00, 0x03, b'a', b'/', b'b', 0x01]);
    }
}
