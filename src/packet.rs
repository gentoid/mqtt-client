use crate::{
    packet::{
        connect::{ConnAck, Connect},
        decode::{Decode, DecodePacket},
        encode::{Encode, EncodePacket},
        subscribe::{SubAck, Subscribe},
        unsubscribe::Unsubscribe,
    },
    protocol::{FixedHeader, PacketType},
};

pub mod connect;
pub mod decode;
pub mod encode;
pub mod publish;
pub mod subscribe;
pub mod unsubscribe;

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

impl<'buf> Packet<'buf> {
    fn encode(&self, cursor: &mut encode::Cursor) -> Result<(), crate::Error> {
        match self {
            Self::Connect(packet) => encode_packet(packet, cursor),
            Self::Publish(packet) => encode_packet(packet, cursor),
            Self::Subscribe(packet) => encode_packet(packet, cursor),
            Self::Unsubscribe(packet) => encode_packet(packet, cursor),
            Self::PingReq => empty_body(cursor, PacketType::PingReq),
            Self::PingResp => empty_body(cursor, PacketType::PingResp),
            Self::Disconnect => empty_body(cursor, PacketType::Disconnect),
            _ => Err(crate::Error::EncodeNotImplemented),
        }
    }

    fn decode<'cursor>(
        header: &FixedHeader,
        cursor: &'cursor mut decode::Cursor<'buf>,
    ) -> Result<Self, crate::Error> {
        // @todo this looks wrong
        if header.remaining_len as usize != cursor.remaining() {
            return Err(crate::Error::MalformedPacket);
        }

        let flags = header.flags;

        match header.packet_type {
            PacketType::Connect => connect::Connect::decode(flags, cursor).map(Packet::Connect),
            PacketType::ConnAck => connect::ConnAck::decode(flags, cursor).map(Packet::ConnAck),
            PacketType::Publish => publish::Publish::decode(flags, cursor).map(Packet::Publish),
            PacketType::PubAck => only_packet_id(cursor).map(Packet::PubAck),
            PacketType::PubRec => only_packet_id(cursor).map(Packet::PubRec),
            PacketType::PubRel => only_packet_id(cursor).map(Packet::PubRel),
            PacketType::PubComp => only_packet_id(cursor).map(Packet::PubComp),
            PacketType::Subscribe => {
                subscribe::Subscribe::decode(flags, cursor).map(Packet::Subscribe)
            }
            PacketType::SubAck => subscribe::SubAck::decode(flags, cursor).map(Packet::SubAck),
            PacketType::Unsubscribe => {
                unsubscribe::Unsubscribe::decode(flags, cursor).map(Packet::Unsubscribe)
            }
            PacketType::UnsubAck => only_packet_id(cursor).map(Packet::UnsubAck),
            PacketType::PingReq => cursor.expect_empty().map(|_| Packet::PingReq),
            PacketType::PingResp => cursor.expect_empty().map(|_| Packet::PingResp),
            PacketType::Disconnect => cursor.expect_empty().map(|_| Packet::Disconnect),
        }
    }
}

fn encode_packet<P: encode::EncodePacket>(
    packet: P,
    cursor: &mut encode::Cursor<'_>,
) -> Result<(), crate::Error> {
    let header = ((P::PACKET_TYPE as u8) << 4) | (packet.flags() & 0x0F);
    cursor.write_u8(header)?;

    encode::remaining_length(packet.required_space(), cursor)?;

    packet.encode_body(cursor)
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
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

impl encode::Encode for QoS {
    fn encode(&self, cursor: &mut encode::Cursor) -> Result<(), crate::Error> {
        (*self as u8).encode(cursor)?;
        Ok(())
    }

    fn required_space(&self) -> usize {
        1
    }
}

impl<'buf> decode::Decode<'buf> for QoS {
    fn decode<'cursor>(cursor: &'cursor mut decode::Cursor<'buf>) -> Result<Self, crate::Error> {
        let byte = cursor.read_u8()?;
        Self::try_from(byte)
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
        let mut cursor = decode::Cursor::new(bytes);
        let res = cursor.read_u16()?;

        cursor.expect_empty()?;

        Self::try_from(res)
    }
}

impl encode::Encode for PacketId {
    fn encode(&self, cursor: &mut encode::Cursor) -> Result<(), crate::Error> {
        self.0.encode(cursor)?;
        Ok(())
    }

    fn required_space(&self) -> usize {
        2
    }
}

impl<'buf> decode::Decode<'buf> for PacketId {
    fn decode(cursor: &mut decode::Cursor<'buf>) -> Result<Self, crate::Error> {
        Self::try_from(cursor.read_u16()?)
    }
}

fn only_packet_id(cursor: &mut decode::Cursor<'_>) -> Result<PacketId, crate::Error> {
    let packet_id = PacketId::decode(cursor)?;
    cursor.expect_empty()?;
    Ok(packet_id)
}

pub(super) fn empty_body(
    cursor: &mut encode::Cursor,
    packet_type: PacketType,
) -> Result<(), crate::Error> {
    let header = ((packet_type as u8) << 4) | 0b0010;

    header.encode(cursor)?;
    0u8.encode(cursor)
}
