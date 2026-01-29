use crate::packet::{Packet, subscribe, unsubscribe};

// const UNSUBSCRIBE_ID: u8 = ...;
const PING_REQ_ID: u8 = 0b1100_0000;
const PING_RESP_ID: u8 = 0b1101_0000;
const DISCONNECT_ID: u8 = 0b1110_0000;

pub trait Encode {
    fn encode<const N: usize>(&self, out: &mut heapless::Vec<u8, N>) -> Result<(), crate::Error>;
}

impl Encode for Packet<'_> {
    fn encode<const N: usize>(&self, out: &mut heapless::Vec<u8, N>) -> Result<(), crate::Error> {
        match self {
            Self::Connect(_) => todo!(),
            Self::Publish(_) => todo!(),
            Self::Subscribe(packet) => subscribe::encode(out, &packet),
            Self::Unsubscribe(packet) => unsubscribe::encode(out, &packet),
            Self::PingReq => empty_body(out, PING_REQ_ID),
            Self::PingResp => empty_body(out, PING_RESP_ID),
            Self::Disconnect => empty_body(out, DISCONNECT_ID),
            _ => Err(crate::Error::EncodeNotImplemented),
        }
    }
}

fn empty_body<const N: usize>(out: &mut heapless::Vec<u8, N>, id: u8) -> Result<(), crate::Error> {
    out.push(id).map_err(is_full)?;
    out.push(0).map_err(is_full)?;
    Ok(())
}

pub(super) fn is_full<T>(_: T) -> crate::Error {
    crate::Error::VectorIsFull
}
