use crate::{
    packet::subscribe::{SubAck, Subscribe},
    protocol::{FixedHeader, PacketType},
};

pub mod publish;
pub mod subscribe;

pub enum Packet<'a> {
    ConnAck(ConnAck),
    Publish(publish::Publish<'a>),
    PubAck(PacketId),
    PubRec(PacketId),
    PubRel(PacketId),
    PubComp(PacketId),
    Subscribe(Subscribe<'a>),
    SubAck(SubAck),
    PingReq,
    PingResp,
    Disconnect,
}

pub enum QoS {
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

impl TryFrom<u8> for QoS {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let qos = match value {
            0 => Self::AtMostOnce,
            1 => Self::AtLeastOnce,
            2 => Self::ExactlyOnce,
            _ => return Err(crate::Error::InvalidQoS),
        };

        Ok(qos)
    }
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

#[derive(Debug, PartialEq)]
pub struct PacketId(u16);

impl TryFrom<u16> for PacketId {
    type Error = crate::Error;

    fn try_from(id: u16) -> Result<Self, Self::Error> {
        if id == 0 {
            return Err(crate::Error::MalformedPacket);
        }

        Ok(Self(id))
    }
}

impl TryFrom<&[u8]> for PacketId {
    type Error = crate::Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != 2 {
            return Err(crate::Error::MalformedPacket);
        }

        let id = u16::from_be_bytes([bytes[0], bytes[1]]);
        Self::try_from(id)
    }
}

fn parse<'a>(header: FixedHeader, body: &'a [u8]) -> Result<Packet<'a>, crate::Error> {
    if header.remaining_len as usize != body.len() {
        return Err(crate::Error::MalformedPacket);
    }

    match header.packet_type {
        PacketType::Connect => todo!(),
        PacketType::ConnAck => parse_connack(body).map(Packet::ConnAck),
        PacketType::Publish => publish::parse(header.flags, body).map(Packet::Publish),
        PacketType::PubAck => only_packet_id(body).map(Packet::PubAck),
        PacketType::PubRec => only_packet_id(body).map(Packet::PubRec),
        PacketType::PubRel => only_packet_id(body).map(Packet::PubRel),
        PacketType::PubComp => only_packet_id(body).map(Packet::PubComp),
        PacketType::Subscribe => subscribe::parse(body).map(Packet::Subscribe),
        PacketType::SubAck => subscribe::parse_suback(&body).map(Packet::SubAck),
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

fn parse_connack(body: &[u8]) -> Result<ConnAck, crate::Error> {
    expect_body_len(body, 2)?;

    let flags = body[0];
    let return_code = ConnectReturnCode::try_from(body[1])?;

    if flags & 0b1111_1110 != 0 {
        return Err(crate::Error::MalformedPacket);
    }

    let session_present = (flags & 0b0000_0001) == 1;

    Ok(ConnAck {
        return_code,
        session_present,
    })
}

fn only_packet_id(body: &[u8]) -> Result<PacketId, crate::Error> {
    let len = 2;
    expect_body_len(body, len)?;

    let packet_id = parse_packet_id(body, &mut 0)?;
    Ok(packet_id)
}

fn parse_packet_id<'a>(body: &[u8], offset: &'a mut usize) -> Result<PacketId, crate::Error> {
    let len = 2;
    let bytes = get_bytes(body, offset)(len)?;
    PacketId::try_from(bytes)
}

fn parse_utf8_str<'a, 'b>(body: &'a [u8], offset: &'b mut usize) -> Result<&'a str, crate::Error> {
    let mut get_bytes = get_bytes(body, offset);
    let bytes = get_bytes(2)?;
    let len = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;

    core::str::from_utf8(get_bytes(len)?).map_err(|_| crate::Error::InvalidUtf8)
}

fn get_bytes<'a, 'b>(
    body: &'a [u8],
    offset: &'b mut usize,
) -> impl FnMut(usize) -> Result<&'a [u8], crate::Error> {
    |len: usize| {
        if body.len() < *offset + len {
            return Err(crate::Error::MalformedPacket);
        }

        let prev_offset = *offset;
        *offset += len;
        Ok(&body[prev_offset..*offset])
    }
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
            ConnAck {
                session_present: false,
                return_code: ConnectReturnCode::Accepted
            }
        ));
    }

    #[test]
    fn connack_invalid_flags() {
        let body = [0b0000_0010, 0x00];
        assert!(parse_connack(&body).is_err());
    }
}
