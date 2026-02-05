use embedded_io_async::Read;
use heapless::Vec;

use crate::{
    buffer,
    packet::{
        Packet, PacketId, QoS,
        connect::{ConnAck, ConnectReturnCode},
        encode::Encode,
        publish::{self, Publish},
        subscribe::{SubAck, SubAckReturnCode},
    },
    protocol::{FixedHeader, PacketType},
};

pub(crate) async fn read_packet<'p, R: Read, P: buffer::Provider<'p>, const N: usize>(
    read: &mut R,
    buf_provider: &'p mut P,
) -> Result<Packet<'p>, crate::Error> {
    let header = read_fixed_header(read).await?;
    read_packet_body::<R, P, N>(read, buf_provider, &header).await
}

async fn read_fixed_header<R: Read>(read: &mut R) -> Result<FixedHeader, crate::Error> {
    let byte = read_u8(read).await?;
    let (packet_type, flags) = parse_first_byte(byte)?;
    let remaining_len = read_remaining_len(read).await?;

    Ok(FixedHeader {
        flags,
        packet_type,
        remaining_len,
    })
}

async fn read_packet_body<'p, R: Read, P: buffer::Provider<'p>, const N: usize>(
    read: &mut R,
    buf_provider: &'p mut P,
    header: &FixedHeader,
) -> Result<Packet<'p>, crate::Error> {
    match header.packet_type {
        PacketType::ConnAck => read_connack(read, header).await,
        PacketType::Publish => read_publish(read, buf_provider, header).await,
        PacketType::PubAck => read_only_packet_id(read, header).await.map(Packet::PubAck),
        PacketType::PubRec => read_only_packet_id(read, header).await.map(Packet::PubRec),
        PacketType::PubRel => read_only_packet_id(read, header).await.map(Packet::PubRel),
        PacketType::PubComp => read_only_packet_id(read, header).await.map(Packet::PubComp),
        PacketType::SubAck => read_suback::<R, N>(read, header).await,
        PacketType::UnsubAck => read_only_packet_id(read, header)
            .await
            .map(Packet::UnsubAck),
        PacketType::PingReq => expect_empty(header).map(|_| Packet::PingReq),
        PacketType::PingResp => expect_empty(header).map(|_| Packet::PingResp),
        _ => Err(crate::Error::UnsupportedIncomingPacket),
    }
}

async fn read_connack<'p, R: Read>(
    read: &mut R,
    header: &FixedHeader,
) -> Result<Packet<'p>, crate::Error> {
    if header.remaining_len != 2 {
        return Err(crate::Error::MalformedRemainingLength);
    }

    let flags = read_u8(read).await?;
    if flags & 0b1111_1110 != 0 {
        return Err(crate::Error::MalformedPacket);
    }

    let session_present = (flags & 0b0000_0001) != 0;
    let return_code = ConnectReturnCode::try_from(read_u8(read).await?)?;

    if return_code != ConnectReturnCode::Accepted && session_present {
        return Err(crate::Error::MalformedPacket);
    }

    Ok(Packet::ConnAck(ConnAck {
        return_code,
        session_present,
    }))
}

async fn read_publish<'p, R: Read, P: buffer::Provider<'p>>(
    read: &mut R,
    buf_provider: &'p mut P,
    header: &FixedHeader,
) -> Result<Packet<'p>, crate::Error> {
    let flags = publish::Flags::try_from(header.flags)?;

    // // @todo this cannot be a topic filter unlike subscribe, so maybe check for allowed chars
    let buf = read_binary(read, buf_provider).await?;
    let topic = buffer::String::from(buf);

    let packet_id = if let QoS::AtMostOnce = flags.qos {
        None
    } else {
        let packet_id = PacketId::try_from(read_u16(read).await?)?;

        Some(packet_id)
    };

    let read_bytes = topic.required_space() + packet_id.map(|id| id.required_space()).unwrap_or(0);

    let len = header
        .remaining_len
        .checked_sub(read_bytes)
        .ok_or(crate::Error::MalformedRemainingLength)?;

    let buf = buf_provider
        .provide(len)
        .map_err(|_| crate::Error::BufferTooSmall)?;

    read.read_exact(buf)
        .await
        .map_err(|_| crate::Error::TransportError)?;

    let payload = buffer::Slice::from(buf);

    Ok(Packet::Publish(Publish {
        flags,
        topic,
        packet_id,
        payload,
    }))
}

async fn read_only_packet_id<R: Read>(
    read: &mut R,
    header: &FixedHeader,
) -> Result<PacketId, crate::Error> {
    if header.remaining_len != 2 {
        return Err(crate::Error::MalformedRemainingLength);
    }

    PacketId::try_from(read_u16(read).await?)
}

async fn read_suback<'p, R: Read, const N: usize>(
    read: &mut R,
    header: &FixedHeader,
) -> Result<Packet<'p>, crate::Error> {
    if header.remaining_len <= 2 {
        return Err(crate::Error::MalformedRemainingLength);
    }

    let mut remaining = header.remaining_len;

    let packet_id = PacketId::try_from(read_u16(read).await?)?;
    remaining -= 2;

    let mut return_codes = Vec::<SubAckReturnCode, 1>::new();

    while remaining > 0 {
        let code = SubAckReturnCode::try_from(read_u8(read).await?)?;
        remaining -= 1;

        return_codes
            .push(code)
            .map_err(|_| crate::Error::VectorIsFull)?;
    }

    Ok(Packet::SubAck(SubAck {
        packet_id,
        return_codes,
    }))
}

fn expect_empty(header: &FixedHeader) -> Result<(), crate::Error> {
    if header.remaining_len != 0 {
        return Err(crate::Error::MalformedRemainingLength);
    }

    Ok(())
}

fn parse_first_byte(byte: u8) -> Result<(PacketType, u8), crate::Error> {
    let packet_type = PacketType::try_from(byte >> 4)?;
    let flags = byte & 0x0F;

    if !packet_type.validate_flags(flags) {
        return Err(crate::Error::InvalidFlags);
    }

    Ok((packet_type, flags))
}

async fn read_remaining_len<R: Read>(read: &mut R) -> Result<usize, crate::Error> {
    let mut bytes_read = 0;
    let mut remaining_len: usize = 0;
    let mut multiplier = 1;

    loop {
        let byte = read_u8(read).await?;
        bytes_read += 1;

        let digit = (byte & 0x7F) as usize;
        remaining_len = remaining_len
            .checked_add(digit * multiplier)
            .ok_or(crate::Error::MalformedRemainingLength)?;

        if (byte & 0x80) == 0 {
            return Ok(remaining_len);
        }

        if bytes_read >= 4 {
            return Err(crate::Error::MalformedRemainingLength);
        }

        multiplier *= 128;
    }
}

async fn read_u8<R: Read>(read: &mut R) -> Result<u8, crate::Error> {
    let mut buf = [0u8; 1];
    read.read_exact(&mut buf)
        .await
        .map_err(|_| crate::Error::TransportError)?;

    Ok(buf[0])
}

pub(crate) async fn read_u16<R: Read>(read: &mut R) -> Result<u16, crate::Error> {
    let mut buf = [0u8; 2];
    read.read_exact(&mut buf)
        .await
        .map_err(|_| crate::Error::TransportError)?;

    Ok(u16::from_be_bytes(buf))
}

async fn read_binary<'p, R: Read, P: buffer::Provider<'p>>(
    read: &mut R,
    buf_provider: &mut P,
) -> Result<&'p mut [u8], crate::Error> {
    let len = read_u16(read).await? as usize;
    let buf = buf_provider
        .provide(len)
        .map_err(|_| crate::Error::BufferTooSmall)?;

    read.read_exact(buf)
        .await
        .map_err(|_| crate::Error::TransportError)?;

    Ok(buf)
}

// #[cfg(test)]
// mod tests {
//     use heapless::Vec;

//     use crate::Error;

//     use super::*;

//     fn collect_events<'a, const N: usize>(
//         parser: &mut Parser,
//         mut input: &'a [u8],
//     ) -> Result<Vec<Event<'a>, N>, Error> {
//         let mut events = Vec::new();

//         loop {
//             let (consumed, event) = parser.parse(input)?;

//             if let Some(event) = event {
//                 events.push(event).unwrap();
//             };

//             if input.is_empty() {
//                 return Ok(events);
//             }

//             input = &input[consumed..];
//         }
//     }

//     #[test]
//     fn remaining_len_single_byte() {
//         let mut parser = Parser::new();

//         // PingReq, remaining length = 0
//         let data = [0b1100_0000, 0x00];
//         let events = collect_events::<2>(&mut parser, &data).unwrap();

//         assert_eq!(events.len(), 2);

//         assert!(matches!(events[0], Event::PacketStart { header } if header.remaining_len == 0));
//         assert!(matches!(events[1], Event::PacketEnd));
//     }

//     #[test]
//     fn remaining_len_multibyte() {
//         let mut parser = Parser::new();

//         // PingReq, remaining length = 321
//         let data = [0b1100_0000, 0xC1, 0x02];
//         let events = collect_events::<1>(&mut parser, &data).unwrap();
//         assert_eq!(events.len(), 1);

//         match &events[0] {
//             Event::PacketStart { header } => assert_eq!(header.remaining_len, 321),
//             _ => panic!("Expected PacketStart"),
//         }
//     }

//     #[test]
//     fn body_single_chunk() {
//         let mut parser = Parser::new();

//         // Publish, remaining length = 3
//         let data = [0b0011_0000, 0x03, 0xAA, 0xBB, 0xCC];
//         let events = collect_events::<3>(&mut parser, &data).unwrap();

//         assert_eq!(events.len(), 3);
//         assert!(matches!(events[0], Event::PacketStart { .. }));

//         match events[1] {
//             Event::PacketBody { chunk } => assert_eq!(chunk, &[0xAA, 0xBB, 0xCC]),
//             _ => panic!("Expected PacketBody"),
//         }

//         assert!(matches!(events[2], Event::PacketEnd));
//     }

//     #[test]
//     fn body_split_across_inputs() {
//         let mut parser = Parser::new();

//         let data1 = [0b0011_0000, 0x04, 0x01];
//         let data2 = [0x23, 0x45];
//         let data3 = [0x67];

//         let mut events: Vec<Event<'_>, 6> = Vec::new();

//         for chunk in [&data1[..], &data2[..], &data3[..]] {
//             for event in collect_events::<2>(&mut parser, &chunk)
//                 .unwrap()
//                 .into_iter()
//             {
//                 events.push(event).unwrap();
//             }
//         }

//         assert!(matches!(events[0], Event::PacketStart { .. }));

//         let body: Vec<u8, 4> = events
//             .iter()
//             .filter(|event| matches!(event, Event::PacketBody { .. }))
//             .flat_map(|event| match event {
//                 Event::PacketBody { chunk } => chunk.into_iter().copied(),
//                 _ => unreachable!(),
//             })
//             .collect();

//         assert_eq!(&body, &[0x01, 0x23, 0x45, 0x67]);
//         assert!(matches!(events.last(), Some(Event::PacketEnd)));
//     }

//     #[test]
//     fn two_packets_back_to_back() {
//         let mut parser = Parser::new();
//         let data = [0b1100_0000, 0x00, 0b1100_0000, 0x00];

//         let events = collect_events::<4>(&mut parser, &data).unwrap();

//         assert_eq!(events.len(), 4);
//         assert!(matches!(events[1], Event::PacketEnd));
//         assert!(matches!(events[3], Event::PacketEnd));
//     }

//     #[test]
//     fn remaining_len_too_long() {
//         let mut parser = Parser::new();
//         let data = [0b1100_0000, 0xFF, 0xFF, 0xFF, 0xFF, 0x01];
//         let err = collect_events::<2>(&mut parser, &data).unwrap_err();

//         assert!(matches!(err, crate::Error::MalformedRemainingLength));
//     }
// }
