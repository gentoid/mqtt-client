use crate::{
    packet::{
        connect::{ConnAck, Connect},
        encode::Encode,
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

#[cfg(feature = "defmt")]
#[derive(defmt::Format)]
pub(crate) enum Packet<'a> {
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
    pub(crate) fn encode(&self, cursor: &mut encode::Cursor) -> Result<(), crate::Error> {
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

    pub(crate) fn required_space(&self) -> usize {
        todo!()
    }

    pub(crate) fn decode(header: &FixedHeader, body: &'buf [u8]) -> Result<Self, crate::Error> {
        let cursor = &mut decode::Cursor::new(&body);

        // @todo this looks wrong
        if header.remaining_len as usize != cursor.remaining() {
            return Err(crate::Error::MalformedPacket);
        }

        let flags = header.flags;

        match header.packet_type {
            PacketType::Connect => connect::Connect::decode(cursor).map(Packet::Connect),
            PacketType::ConnAck => connect::ConnAck::decode(cursor).map(Packet::ConnAck),
            PacketType::Publish => publish::Publish::decode(cursor, flags).map(Packet::Publish),
            PacketType::PubAck => only_packet_id(cursor).map(Packet::PubAck),
            PacketType::PubRec => only_packet_id(cursor).map(Packet::PubRec),
            PacketType::PubRel => only_packet_id(cursor).map(Packet::PubRel),
            PacketType::PubComp => only_packet_id(cursor).map(Packet::PubComp),
            PacketType::Subscribe => subscribe::Subscribe::decode(cursor).map(Packet::Subscribe),
            PacketType::SubAck => subscribe::SubAck::decode(cursor).map(Packet::SubAck),
            PacketType::Unsubscribe => {
                unsubscribe::Unsubscribe::decode(cursor).map(Packet::Unsubscribe)
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
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg(feature = "defmt")]
#[derive(defmt::Format)]
pub enum QoS {
    #[default]
    AtMostOnce = 0,
    AtLeastOnce = 1,
    ExactlyOnce = 2,
}

impl QoS {
    fn decode<'cursor>(cursor: &'cursor mut decode::Cursor) -> Result<Self, crate::Error> {
        let byte = cursor.read_u8()?;
        Self::try_from(byte)
    }
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

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg(feature = "defmt")]
#[derive(defmt::Format)]
pub(crate) struct PacketId(pub(crate) u16);

impl PacketId {
    fn decode(cursor: &mut decode::Cursor) -> Result<Self, crate::Error> {
        Self::try_from(cursor.read_u16()?)
    }
}

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

        let res = u16::from_be_bytes([bytes[0], bytes[1]]);
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

// pub struct Assembled<'a> {
//     pub header: FixedHeader,
//     pub body: &'a [u8],
// }

// pub struct Assembler {
//     parser: parser::Parser,
//     header: Option<FixedHeader>,
//     // body_chunk: Option<&'a [u8]>,
// }

// impl Assembler {
//     pub fn new() -> Self {
//         Self {
//             parser: parser::Parser::new(),
//             header: None,
//             // body_chunk: None,
//         }
//     }

//     pub fn feed<'p, P: buffer::Provider<'p>>(
//         &mut self,
//         input: &[u8],
//         provider: P,
//     ) -> Result<(usize, Option<Assembled>), crate::Error> {
//         let mut offset = 0;

//         loop {
//             let (consumed, event) = self.parser.parse(&input[offset..])?;

//             offset += consumed;

//             if let Some(event) = event {
//                 match event {
//                     parser::Event::PacketStart { header } => {
//                         self.header = Some(header);
//                         // self.body_chunk = None;
//                     }
//                     parser::Event::PacketBody { chunk } => {
//                         // self.body_chunk = Some(chunk);
//                     }
//                     parser::Event::PacketEnd => {
//                         let header = self.header.take().ok_or(crate::Error::MalformedPacket)?;
//                         Packet::decode(&header, cursor, provider)?;
//                         // let body = self
//                         //     .body_chunk
//                         //     .take()
//                         //     .ok_or(crate::Error::MalformedPacket)?;
//                         let body = &input[0..2];

//                         return Ok((offset, Some(Assembled { header, body })));
//                     }
//                 }
//             }

//             if consumed == 0 {
//                 break;
//             }
//         }

//         Ok((offset, None))
//     }
// }

// #[cfg(test)]
// mod tests {
//     use crate::{
//         buffer,
//         packet::{QoS, connect, encode, encode_packet, publish},
//         protocol::PacketType,
//     };

//     #[test]
//     fn test_assembler_connect() {
//         let packet = connect::Connect {
//             client_id: buffer::String::from("Client"),
//             keep_alive: 60,
//             clean_session: true,
//             password: None,
//             username: None,
//             will: None,
//         };

//         let mut buf = [0u8; 128];
//         let mut cursor = encode::Cursor::new(&mut buf);
//         encode_packet(&packet, &mut cursor).unwrap();
//         let buf = cursor.written();
//         let mut assembler = Assembler::new();

//         // [16, 18, 0, 4, 77, 81, 84, 84, 4, 0, 0, 60, 0, 6, 67, 108, 105, 101, 110, 116]

//         let (consumed, packet) = assembler.feed(&buf).unwrap();

//         assert_eq!(consumed, buf.len());
//         let packet = packet.expect("Packet should be ready");

//         assert!(matches!(packet.header.packet_type, PacketType::Connect));
//         assert_eq!(packet.header.remaining_len as usize, packet.body.len());
//     }

//     #[test]
//     fn test_assembler_publish() {
//         let packet = publish::Publish {
//             topic: buffer::String::from("topic/test"),
//             payload: buffer::Slice::from(b"hello mqtt".as_slice()),
//             flags: publish::Flags {
//                 dup: false,
//                 qos: QoS::AtMostOnce,
//                 retain: false,
//             },
//             packet_id: None,
//         };

//         let mut buf = [0u8; 128];
//         let mut cursor = encode::Cursor::new(&mut buf);
//         crate::packet::encode_packet(&packet, &mut cursor).unwrap();

//         let buf = cursor.written();
//         let mut assembler = Assembler::new();

//         let (consumed, packet) = assembler.feed(&buf).unwrap();

//         assert_eq!(consumed, buf.len());
//         let packet = packet.expect("Packet should be ready");

//         assert!(matches!(packet.header.packet_type, PacketType::Publish));
//     }
// }
