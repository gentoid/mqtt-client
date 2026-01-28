#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
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
        match value {
            #[cfg(not(feature = "v50"))]
            1..=14 => Ok(unsafe { core::mem::transmute(value) }),
            #[cfg(feature = "v50")]
            1..=15 => Ok(unsafe { core::mem::transmute(value) }),
            _ => Err(crate::Error::InvalidPacketType),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FixedHeader {
    pub packet_type: PacketType,
    pub flags: u8,
    pub remaining_len: u32,
}
