use crate::packet::{Packet, publish, subscribe, unsubscribe};

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
            Self::Publish(packet) => publish::encode(out, &packet),
            Self::Subscribe(packet) => packet.encode(out),
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

impl Encode for u16 {
    fn encode<const N: usize>(&self, out: &mut heapless::Vec<u8, N>) -> Result<(), crate::Error> {
        out.extend_from_slice(&self.to_be_bytes()).map_err(is_full)?;
        Ok(())
    }
}

impl Encode for u8 {
    fn encode<const N: usize>(&self, out: &mut heapless::Vec<u8, N>) -> Result<(), crate::Error> {
        out.push(*self).map_err(is_full)?;
        Ok(())
    }
}

impl Encode for &str {
    fn encode<const N: usize>(&self, out: &mut heapless::Vec<u8, N>) -> Result<(), crate::Error> {
        let len = self.len() as u16;
        len.encode(out)?;
        out.extend_from_slice(self.as_bytes()).map_err(is_full)?;
        Ok(())
    }
}

impl <T: Encode, const  P: usize> Encode for heapless::Vec<T, P> {
    fn encode<const N: usize>(&self, out: &mut heapless::Vec<u8, N>) -> Result<(), crate::Error> {
        for item in self.as_slice() {
            item.encode(out)?;
        }

        Ok(())
    }
}
