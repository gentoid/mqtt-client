use heapless::Vec;

use crate::{
    packet::{
        PacketId, QoS, SUBSCRIBE_ID, decode,
        encode::{self, is_full},
    },
    protocol::FixedHeader,
};

pub struct Subscribe<'a, const N: usize = 16> {
    pub packet_id: PacketId,
    pub topics: Vec<Subscription<'a>, N>,
}

impl<'a, const P: usize> encode::Encode for Subscribe<'a, P> {
    fn encode<const N: usize>(&self, out: &mut heapless::Vec<u8, N>) -> Result<(), crate::Error> {
        let mut body: Vec<u8, N> = Vec::new();
        self.packet_id.encode(&mut body)?;
        self.topics.encode(&mut body)?;

        out.push(SUBSCRIBE_ID).map_err(is_full)?;
        body.encode(out)?;
        Ok(())
    }
}

impl<'buf, const P: usize> decode::Decode<'buf> for Subscribe<'buf, P> {
    fn decode<'cursor>(
        header: &FixedHeader,
        cursor: &'cursor mut decode::Cursor<'buf>,
    ) -> Result<Self, crate::Error> {
        let packet_id = PacketId::decode(header, cursor)?;

        let mut topics = Vec::<Subscription<'buf>, P>::new();

        while !cursor.is_empty() {
            let topic_filter = cursor.read_utf8()?;
            let qos = QoS::decode(header, cursor)?;

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

pub struct Subscription<'a> {
    pub topic_filter: &'a str,
    pub qos: QoS,
}

impl<'a> encode::Encode for Subscription<'a> {
    fn encode<const N: usize>(&self, out: &mut heapless::Vec<u8, N>) -> Result<(), crate::Error> {
        self.topic_filter.encode(out)?;
        self.qos.encode(out)?;
        Ok(())
    }
}

pub struct SubAck<const N: usize = 16> {
    pub(crate) packet_id: PacketId,
    pub return_codes: Vec<SubAckReturnCode, N>,
}

impl <'buf, const P: usize> decode::Decode<'buf> for SubAck<P> {
    fn decode<'cursor>(
        header: &FixedHeader,
        cursor: &'cursor mut decode::Cursor<'buf>,
    ) -> Result<Self, crate::Error> {

    let packet_id = PacketId::decode(header, cursor)?;
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
    use crate::{packet::decode::{Cursor, Decode}, protocol::PacketType};

    use super::*;

    fn parse_suback<const N: usize>(body: &[u8]) -> Result<SubAck<N>, crate::Error> {
        let header = FixedHeader {flags: 0, packet_type: PacketType::SubAck, remaining_len: 0};
        SubAck::<N>::decode(&header, &mut Cursor::new(&body))
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
