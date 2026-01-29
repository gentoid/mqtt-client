use heapless::Vec;

use crate::packet::{PacketId, QoS, get_bytes, parse_packet_id, parse_utf8_str};

pub struct Subscribe<'a, const N: usize = 16> {
    pub packet_id: PacketId,
    pub topics: Vec<Subscription<'a>, N>,
}

pub struct Subscription<'a> {
    pub topic_filter: &'a str,
    pub qos: QoS,
}

pub struct SubAck<const N: usize = 16> {
    pub(crate) packet_id: PacketId,
    pub return_codes: Vec<SubAckReturnCode, N>,
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

pub(super) fn parse<'a, const N: usize>(body: &'a [u8]) -> Result<Subscribe<'a, N>, crate::Error> {
    let mut offset = 0;

    let packet_id = parse_packet_id(body, &mut offset)?;

    let mut topics = Vec::<Subscription<'a>, N>::new();

    while offset < body.len() {
        let topic_filter = parse_utf8_str(body, &mut offset)?;
        let qos_byte = get_bytes(body, &mut offset)(1)?[0];
        let qos = QoS::try_from(qos_byte)?;

        topics
            .push(Subscription { topic_filter, qos })
            .map_err(|_| crate::Error::TooSmallSubscriptionVector)?;
    }

    if topics.is_empty() {
        return Err(crate::Error::MalformedPacket);
    }

    Ok(Subscribe { packet_id, topics })
}

pub(super) fn parse_suback<const N: usize>(body: &[u8]) -> Result<SubAck<N>, crate::Error> {
    if body.len() < 2 {
        return Err(crate::Error::MalformedPacket);
    }

    let packet_id = PacketId::try_from(&body[..2])?;
    let mut return_codes = Vec::<SubAckReturnCode, N>::new();

    for &byte in &body[2..] {
        let code = SubAckReturnCode::try_from(byte)?;
        return_codes
            .push(code)
            .map_err(|_| crate::Error::TooSmallSubAckVector)?;
    }

    Ok(SubAck {
        packet_id,
        return_codes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
