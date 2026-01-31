use heapless::Vec;

use crate::{
    packet::{
        PacketId, QoS, SUBSCRIBE_ID,
        decode::{self, Decode},
        encode::{self, RequiredSize, is_full},
    },
    protocol::FixedHeader,
};

pub struct Subscribe<'a, const N: usize = 16> {
    pub packet_id: PacketId,
    pub topics: Vec<Subscription<'a>, N>,
}

impl<'a, const P: usize> encode::Encode for Subscribe<'a, P> {
    fn encode(&self, cursor: &mut encode::Cursor) -> Result<(), crate::Error> {
        cursor.write_u8(SUBSCRIBE_ID)?;
        encode::remaining_length(self.required_space(), cursor)?;
        self.packet_id.encode(cursor)?;

        for topic in &self.topics {
            topic.encode(cursor)?;
        }

        Ok(())
    }
}

impl<'buf, const P: usize> decode::DecodePacket<'buf> for Subscribe<'buf, P> {
    fn decode<'cursor>(
        flags: u8,
        cursor: &'cursor mut decode::Cursor<'buf>,
    ) -> Result<Self, crate::Error> {
        let packet_id = PacketId::decode(cursor)?;

        let mut topics = Vec::<Subscription<'buf>, P>::new();

        while !cursor.is_empty() {
            let topic_filter = cursor.read_utf8()?;
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
}

impl<'a, const P: usize> RequiredSize for Subscribe<'a, P> {
    fn required_space(&self) -> usize {
        let mut required_space = self.packet_id.required_space();

        for topic in &self.topics {
            required_space += topic.required_space();
        }

        required_space
    }
}

pub struct Subscription<'a> {
    pub topic_filter: &'a str,
    pub qos: QoS,
}

impl<'a> encode::Encode for Subscription<'a> {
    fn encode(&self, cursor: &mut encode::Cursor) -> Result<(), crate::Error> {
        self.topic_filter.encode(cursor)?;
        self.qos.encode(cursor)
    }
}

pub struct SubAck<const N: usize = 16> {
    pub(crate) packet_id: PacketId,
    pub return_codes: Vec<SubAckReturnCode, N>,
}

impl<'a> encode::RequiredSize for Subscription<'a> {
    fn required_space(&self) -> usize {
        self.topic_filter.required_space() + self.qos.required_space()
    }
}

impl<'buf, const P: usize> decode::DecodePacket<'buf> for SubAck<P> {
    fn decode<'cursor>(
        flags: u8,
        cursor: &'cursor mut decode::Cursor<'buf>,
    ) -> Result<Self, crate::Error> {
        let packet_id = PacketId::decode(cursor)?;
        let mut return_codes = Vec::<SubAckReturnCode, P>::new();

        while !cursor.is_empty() {
            let code = SubAckReturnCode::try_from(cursor.read_u8()?)?;
            return_codes
                .push(code)
                .map_err(|_| crate::Error::VectorIsFull)?;
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
    use crate::{
        packet::{Packet, decode::DecodePacket, encode::Encode},
        protocol::PacketType,
    };

    use super::*;

    fn parse_suback<const N: usize>(body: &[u8]) -> Result<SubAck<N>, crate::Error> {
        SubAck::<N>::decode(0, &mut decode::Cursor::new(&body))
    }

    #[test]
    fn suback_single_success() {
        // packet_id = 16, return code = 1
        let body = [0x00, 0x10, 0x01];
        let packet = parse_suback::<1>(&body).unwrap();

        assert_eq!(packet.packet_id.0, 16);
        assert_eq!(packet.return_codes.len(), 1);
        assert!(matches!(
            packet.return_codes[0],
            SubAckReturnCode::SuccessMaxQoS1
        ));
    }

    #[test]
    fn suback_invalid_return_code() {
        let body = [0x00, 0x10, 0x05];
        assert!(parse_suback::<1>(&body).is_err());
    }
}
