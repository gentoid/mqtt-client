use crate::protocol::{FixedHeader, PacketType};

pub struct Parser {
    state: State,
}

pub(crate) enum State {
    Start,
    RemainingLen(Remaining),
    Body { remaining: u32 },
}

struct Remaining {
    packet_type: PacketType,
    flags: u8,
    multiplier: u32,
    value: u32,
    bytes_read: u8,
}

impl Remaining {
    const fn new(packet_type: PacketType, flags: u8) -> Self {
        Self {
            packet_type,
            flags,
            multiplier: 1,
            value: 0,
            bytes_read: 0,
        }
    }
}

pub enum Event<'a> {
    PacketStart { header: FixedHeader },
    PacketBody { chunk: &'a [u8] },
    PacketEnd,
}

impl Parser {
    pub const fn new() -> Self {
        Self {
            state: State::Start,
        }
    }

    pub fn parse<'a>(
        &mut self,
        input: &'a [u8],
    ) -> Result<(usize, Option<Event<'a>>), crate::Error> {
        match &mut self.state {
            State::Start => {
                if input.is_empty() {
                    return Ok((0, None));
                }

                let (packet_type, flags) = parse_first_byte(input[0])?;
                self.state = State::RemainingLen(Remaining::new(packet_type, flags));

                Ok((1, None))
            }
            State::RemainingLen(remaining) => {
                if input.is_empty() {
                    return Ok((0, None));
                }

                if let Some(header) = parse_remaining_len_byte(remaining, input[0])? {
                    self.state = State::Body {
                        remaining: remaining.value,
                    };

                    return Ok((1, Some(Event::PacketStart { header })));
                };

                Ok((1, None))
            }
            State::Body { remaining } => {
                if *remaining == 0 {
                    self.state = State::Start;
                    return Ok((0, Some(Event::PacketEnd)));
                }

                if input.is_empty() {
                    return Ok((0, None));
                }

                let take = core::cmp::min(*remaining as usize, input.len());
                let chunk = &input[..take];
                *remaining -= take as u32;

                Ok((take, Some(Event::PacketBody { chunk })))
            }
        }
    }
}

fn parse_first_byte(byte: u8) -> Result<(PacketType, u8), crate::Error> {
    let packet_type = PacketType::try_from(byte >> 4)?;
    let flags = byte & 0x0F;

    if !packet_type.validate_flags(flags) {
        return Err(crate::Error::InvalidFlags);
    }

    Ok((packet_type, flags))
}

fn parse_remaining_len_byte(
    remaining: &mut Remaining,
    byte: u8,
) -> Result<Option<FixedHeader>, crate::Error> {
    let digit = (byte & 0x7F) as u32;
    remaining.value += digit * remaining.multiplier;
    remaining.multiplier *= 128;
    remaining.bytes_read += 1;

    if remaining.bytes_read > 4 {
        return Err(crate::Error::MalformedRemainingLength);
    }

    if (byte & 0x80) != 0 {
        return Ok(None);
    }

    Ok(Some(FixedHeader {
        packet_type: remaining.packet_type,
        flags: remaining.flags,
        remaining_len: remaining.value,
    }))
}
