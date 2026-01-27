#[repr(u8)]
#[derive(Clone, Copy)]
pub enum PacketType {
    Connect = 1,
    ConnAck = 2,
    Publish = 3,
    PubAck = 4,
    PubRec = 5,
    PubRel = 6,
    PubComp = 7,
    Subscribe = 8,
    SubAck = 9,
    Unsubscribe = 10,
    UnsubAck = 11,
    PingReq = 12,
    PingResp = 13,
    Disconnect = 14,
    #[cfg(feature = "v50")]
    Auth = 15,
}

impl PacketType {
    pub fn validate_flags(&self, flags: u8) -> bool {
        match self {
            Self::Publish => true,
            Self::PubRel | Self::Subscribe | Self::Unsubscribe => flags == 0b0100,
            _ => flags == 0,
        }
    }
}

impl TryFrom<u8> for PacketType {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        todo!()
    }
}

pub struct FixedHeader {
    pub packet_type: PacketType,
    pub flags: u8,
    pub remaining_len: u32,
}
enum State {
    Start,
    RemainingLen {
        multiplier: u32,
        value: u32,
        byte_read: u8,
        packet_type: PacketType,
        flags: u8,
    },
    Body {
        header: FixedHeader,
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

fn parse_remaining_len_byte(state: &mut State, byte: u8) -> Result<Option<FixedHeader>, crate::Error> {
    if let State::RemainingLen { multiplier, value, byte_read: bytes_read, packet_type, flags } = state {
        let digit = (byte &0x7F) as u32;
        *value += digit * (*multiplier);
        *multiplier *= 128;
        *bytes_read += 1;

        if *bytes_read > 4 {
            return Err(crate::Error::MalformedRemainingLength);
        }

        if (byte & 0x80) != 0 {
            return Ok(None);
        }
        
        Ok(Some(FixedHeader { packet_type: *packet_type, flags: *flags, remaining_len: *value }))
    } else {
        unreachable!()
    }
}
