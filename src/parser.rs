use embedded_io_async::Read;

use crate::{
    packet::{Packet, decode},
    protocol::{FixedHeader, PacketType},
};

fn parse_fixed_header(buf: &[u8]) -> Result<Option<(FixedHeader, usize)>, crate::Error> {
    if buf.len() == 0 {
        return Ok(None);
    }

    let cursor = &mut decode::Cursor::new(buf);
    let byte = cursor.read_u8()?;
    let mut header_len = 1;
    let (packet_type, flags) = parse_first_byte(byte)?;

    if let Some((remaining_len, read)) = parse_remaining_len(cursor)? {
        header_len += read;

        return Ok(Some((
            FixedHeader {
                flags,
                packet_type,
                remaining_len,
            },
            header_len,
        )));
    };

    Ok(None)
}

fn parse_first_byte(byte: u8) -> Result<(PacketType, u8), crate::Error> {
    let packet_type = PacketType::try_from(byte >> 4)?;
    let flags = byte & 0x0F;

    if !packet_type.validate_flags(flags) {
        return Err(crate::Error::InvalidFlags);
    }

    Ok((packet_type, flags))
}

fn parse_remaining_len(
    cursor: &mut decode::Cursor,
) -> Result<Option<(usize, usize)>, crate::Error> {
    let mut bytes_read = 0;
    let mut remaining_len: usize = 0;
    let mut multiplier = 1;

    loop {
        if cursor.is_empty() {
            return Ok(None);
        }

        let byte = cursor.read_u8()?;
        bytes_read += 1;

        let digit = (byte & 0x7F) as usize;
        remaining_len = remaining_len
            .checked_add(digit * multiplier)
            .ok_or(crate::Error::MalformedRemainingLength)?;

        if (byte & 0x80) == 0 {
            return Ok(Some((remaining_len, bytes_read)));
        }

        if bytes_read >= 4 {
            return Err(crate::Error::MalformedRemainingLength);
        }

        multiplier *= 128;
    }
}

pub struct StreamParser<'a> {
    buf: &'a mut [u8],
    start: usize,
    end: usize,
}

impl<'a> StreamParser<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self {
            buf,
            start: 0,
            end: 0,
        }
    }

    pub async fn read<R: Read>(&mut self, read: &mut R) -> Result<Packet<'_>, crate::Error> {
        let mut fixed_header = None;

        loop {
            match fixed_header {
                None => {
                    if let Some((header, header_len)) =
                        parse_fixed_header(&self.buf[self.start..self.end])?
                    {
                        fixed_header = Some(header);
                        self.start += header_len;
                    }
                }
                Some(header) => {
                    let remaining_len = header.remaining_len;

                    if remaining_len > self.buf.len() {
                        return Err(crate::Error::BufferTooSmall);
                    }

                    if self.available_data_len() >= remaining_len {
                        let packet = Packet::decode(
                            &header,
                            &self.buf[self.start..self.start + remaining_len],
                        )?;
                        self.start += remaining_len;

                        if self.start == self.end {
                            self.start = 0;
                            self.end = 0;
                        }

                        return Ok(packet);
                    }
                }
            }

            self.compact();
            let read_buf = &mut self.buf[self.end..];
            let n = read
                .read(read_buf)
                .await
                .map_err(|_| crate::Error::TransportError)?;

            if n == 0 {
                return Err(crate::Error::TransportError);
            }

            self.end += n;
        }
    }

    fn available_data_len(&self) -> usize {
        self.end - self.start
    }

    fn compact(&mut self) {
        if self.start == 0 {
            return;
        }

        if self.start == self.end {
            self.start = 0;
            self.end = 0;
            return;
        }

        self.buf.copy_within(self.start..self.end, 0);
        self.end -= self.start;
        self.start = 0;
    }
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
