use embedded_io_async::Read;

use crate::protocol::{FixedHeader, PacketType};

async fn read_fixed_header<R: Read>(read: &mut R) -> Result<FixedHeader, crate::Error> {
    let byte = read_one_byte(read).await?;
    let (packet_type, flags) = parse_first_byte(byte)?;
    let remaining_len = read_remaining_len(read).await?;

    Ok(FixedHeader {
        flags,
        packet_type,
        remaining_len,
    })
}

fn parse_first_byte(byte: u8) -> Result<(PacketType, u8), crate::Error> {
    let packet_type = PacketType::try_from(byte >> 4)?;
    let flags = byte & 0x0F;

    if !packet_type.validate_flags(flags) {
        return Err(crate::Error::InvalidFlags);
    }

    Ok((packet_type, flags))
}

async fn read_remaining_len<R: Read>(read: &mut R) -> Result<u32, crate::Error> {
    let mut bytes_read = 0;
    let mut remaining_len = 0;
    let mut multiplier = 1;

    loop {
        let byte = read_one_byte(read).await?;
        bytes_read += 1;

        let digit = (byte & 0x7F) as u32;
        remaining_len += digit * multiplier;
        multiplier *= 128;

        if (byte & 0x80) == 0 {
            return Ok(remaining_len);
        }

        if bytes_read > 3 {
            return Err(crate::Error::MalformedRemainingLength);
        }
    }
}

async fn read_one_byte<R: Read>(read: &mut R) -> Result<u8, crate::Error> {
    let mut buf = [0u8; 1];
    read.read_exact(&mut buf)
        .await
        .map_err(|_| crate::Error::TransportError)?;

    Ok(buf[0])
}

// fn parse_remaining_len_bytes(
//     remaining: &mut Remaining,
//     byte: u8,
// ) -> Result<Option<FixedHeader>, crate::Error> {

//

// }

// pub struct Parser {
//     state: State,
// }

// pub(crate) enum State {
//     Start,
//     RemainingLen(Remaining),
//     Body { remaining: u32 },
// }

// pub(crate) struct Remaining {
//     packet_type: PacketType,
//     flags: u8,
//     multiplier: u32,
//     value: u32,
//     bytes_read: u8,
// }

// impl Remaining {
//     const fn new(packet_type: PacketType, flags: u8) -> Self {
//         Self {
//             packet_type,
//             flags,
//             multiplier: 1,
//             value: 0,
//             bytes_read: 0,
//         }
//     }
// }

// #[derive(Debug)]
// pub enum Event<'a> {
//     PacketStart { header: FixedHeader },
//     PacketBody { chunk: &'a [u8] },
//     PacketEnd,
// }

// impl Parser {
//     pub const fn new() -> Self {
//         Self {
//             state: State::Start,
//         }
//     }

//     pub fn parse<'a>(
//         &mut self,
//         input: &'a [u8],
//     ) -> Result<(usize, Option<Event<'a>>), crate::Error> {
//         match &mut self.state {
//             State::Start => {
//                 if input.is_empty() {
//                     return Ok((0, None));
//                 }

//                 let (packet_type, flags) = parse_first_byte(input[0])?;
//                 self.state = State::RemainingLen(Remaining::new(packet_type, flags));

//                 Ok((1, None))
//             }
//             State::RemainingLen(remaining) => {
//                 if input.is_empty() {
//                     return Ok((0, None));
//                 }

//                 if let Some(header) = parse_remaining_len_bytes(remaining, input[0])? {
//                     self.state = State::Body {
//                         remaining: remaining.value,
//                     };

//                     return Ok((1, Some(Event::PacketStart { header })));
//                 };

//                 Ok((1, None))
//             }
//             State::Body { remaining } => {
//                 if *remaining == 0 {
//                     self.state = State::Start;
//                     return Ok((0, Some(Event::PacketEnd)));
//                 }

//                 if input.is_empty() {
//                     return Ok((0, None));
//                 }

//                 let take = core::cmp::min(*remaining as usize, input.len());
//                 let chunk = &input[..take];
//                 *remaining -= take as u32;

//                 Ok((take, Some(Event::PacketBody { chunk })))
//             }
//         }
//     }
// }

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
