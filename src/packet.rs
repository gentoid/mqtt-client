use crate::{
    packet::{
        connect::{ConnAck, Connect},
        subscribe::{SubAck, Subscribe},
        unsubscribe::Unsubscribe,
    },
    protocol::{FixedHeader, PacketType},
};

pub mod connect;
pub mod publish;
pub mod subscribe;
pub mod unsubscribe;
pub mod encode;

pub enum Packet<'a> {
    Connect(Connect<'a>),
    ConnAck(ConnAck),
    Publish(publish::Publish<'a>),
    PubAck(PacketId),
    PubRec(PacketId),
    PubRel(PacketId),
    PubComp(PacketId),
    Subscribe(Subscribe<'a>),
    SubAck(SubAck),
    Unsubscribe(Unsubscribe<'a>),
    UnsubAck(PacketId),
    PingReq,
    PingResp,
    Disconnect,
}

#[repr(u8)]
#[derive(Clone, Copy)]
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

#[derive(Clone, Copy, Debug, PartialEq)]
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

        Self::try_from(parse_u16(bytes, &mut 0)?)
    }
}

fn parse<'a>(header: FixedHeader, body: &'a [u8]) -> Result<Packet<'a>, crate::Error> {
    if header.remaining_len as usize != body.len() {
        return Err(crate::Error::MalformedPacket);
    }

    match header.packet_type {
        PacketType::Connect => connect::parse(body).map(Packet::Connect),
        PacketType::ConnAck => connect::parse_connack(body).map(Packet::ConnAck),
        PacketType::Publish => publish::parse(header.flags, body).map(Packet::Publish),
        PacketType::PubAck => only_packet_id(body).map(Packet::PubAck),
        PacketType::PubRec => only_packet_id(body).map(Packet::PubRec),
        PacketType::PubRel => only_packet_id(body).map(Packet::PubRel),
        PacketType::PubComp => only_packet_id(body).map(Packet::PubComp),
        PacketType::Subscribe => subscribe::parse(body).map(Packet::Subscribe),
        PacketType::SubAck => subscribe::parse_suback(&body).map(Packet::SubAck),
        PacketType::Unsubscribe => unsubscribe::parse(body).map(Packet::Unsubscribe),
        PacketType::UnsubAck => only_packet_id(body).map(Packet::UnsubAck),
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

fn parse_u16(body: &[u8], offset: &mut usize) -> Result<u16, crate::Error> {
    let bytes = get_bytes(body, offset)(2)?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn parse_binary_data<'a, 'b>(
    body: &'a [u8],
    offset: &'b mut usize,
) -> Result<&'a [u8], crate::Error> {
    let len = parse_u16(body, offset)? as usize;

    if body.len() < *offset + len {
        return Err(crate::Error::MalformedPacket);
    }

    Ok(&body[*offset..*offset + len])
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
    let len = parse_u16(body, offset)? as usize;
    core::str::from_utf8(get_bytes(body, offset)(len)?).map_err(|_| crate::Error::InvalidUtf8)
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
