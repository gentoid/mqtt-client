use heapless::Vec;

use crate::protocol::{FixedHeader, PacketType};

pub enum Packet {
    ConnAck(ConnAck),
    SubAck(SubAck),
    PingReq,
    PingResp,
    Disconnect,
}

pub struct ConnAck {
    pub session_present: bool,
    pub return_code: ConnectReturnCode,
}

// @note: for MQTT 5.0 it is a whole another story
#[repr(u8)]
pub enum ConnectReturnCode {
    Accepted = 0,
    UnacceptableProtocolVersion = 1,
    IdentifierRejected = 2,
    ServerUnavailable = 3,
    BadUserNameOrPassword = 4,
    NotAuthorized = 5,
}

impl TryFrom<u8> for ConnectReturnCode {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let code = match value {
            0 => Self::Accepted,
            1 => Self::UnacceptableProtocolVersion,
            2 => Self::IdentifierRejected,
            3 => Self::ServerUnavailable,
            4 => Self::BadUserNameOrPassword,
            5 => Self::NotAuthorized,
            _ => return Err(crate::Error::InvalidConnectReturnCode),
        };

        Ok(code)
    }
}

pub struct SubAck<const N: usize = 16> {
    packet_id: u16,
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

fn parse<'a>(header: FixedHeader, body: &'a [u8]) -> Result<Packet, crate::Error> {
    if header.remaining_len as usize != body.len() {
        return Err(crate::Error::MalformedPacket);
    }

    match header.packet_type {
        PacketType::Connect => todo!(),
        PacketType::ConnAck => parse_connack(body),
        PacketType::Publish => todo!(),
        PacketType::PubAck => todo!(),
        PacketType::PubRec => todo!(),
        PacketType::PubRel => todo!(),
        PacketType::PubComp => todo!(),
        PacketType::Subscribe => todo!(),
        PacketType::SubAck => parse_suback(&body).map(Packet::SubAck),
        PacketType::Unsubscribe => todo!(),
        PacketType::UnsubAck => todo!(),
        PacketType::PingReq => expect_body_len(body, 0).map(|_| Packet::PingReq),
        PacketType::PingResp => expect_body_len(body, 0).map(|_| Packet::PingResp),
        PacketType::Disconnect => expect_body_len(body, 0).map(|_| Packet::Disconnect),
    }
}

fn expect_body_len(body: &[u8], len: usize) -> Result<(), crate::Error> {
    if body.len() == len {
        return Ok(());
    }

    Err(crate::Error::MalformedPacket)
}

fn parse_connack(body: &[u8]) -> Result<Packet, crate::Error> {
    expect_body_len(body, 2)?;

    let flags = body[0];
    let return_code = ConnectReturnCode::try_from(body[1])?;

    if flags & 0b1111_1110 != 0 {
        return Err(crate::Error::MalformedPacket);
    }

    let session_present = (flags & 0b0000_0001) == 1;

    Ok(Packet::ConnAck(ConnAck {
        return_code,
        session_present,
    }))
}

fn parse_suback<const N: usize>(body: &[u8]) -> Result<SubAck<N>, crate::Error> {
    if body.len() < 2 {
        return Err(crate::Error::MalformedPacket);
    }

    let packet_id = u16::from_be_bytes([body[0], body[1]]);
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
    fn connack_accepted() {
        let body = [0x00, 0x00];
        let packet = parse_connack(&body).unwrap();

        assert!(matches!(
            packet,
            Packet::ConnAck(ConnAck {
                session_present: false,
                return_code: ConnectReturnCode::Accepted
            })
        ));
    }

    #[test]
    fn connack_invalid_flags() {
        let body = [0b0000_0010, 0x00];
        assert!(parse_connack(&body).is_err());
    }

    #[test]
    fn suback_single_success() {
        // packet_id = 16, return code = 1
        let body = [0x00, 0x10, 0x01];
        let packet = parse_suback::<1>(&body).unwrap();

        assert_eq!(packet.packet_id, 16);
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
