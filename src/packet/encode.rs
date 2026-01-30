use crate::packet::{Packet, publish, subscribe, unsubscribe};

pub trait Encode {
    fn encode<const N: usize>(&self, out: &mut heapless::Vec<u8, N>) -> Result<(), crate::Error>;
}

pub(super) fn empty_body<const N: usize>(
    out: &mut heapless::Vec<u8, N>,
    id: u8,
) -> Result<(), crate::Error> {
    out.push(id).map_err(is_full)?;
    out.push(0).map_err(is_full)?;
    Ok(())
}

pub(super) fn is_full<T>(_: T) -> crate::Error {
    crate::Error::VectorIsFull
}

impl Encode for u16 {
    fn encode<const N: usize>(&self, out: &mut heapless::Vec<u8, N>) -> Result<(), crate::Error> {
        out.extend_from_slice(&self.to_be_bytes())
            .map_err(is_full)?;
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

impl<T: Encode, const P: usize> Encode for heapless::Vec<T, P> {
    fn encode<const N: usize>(&self, out: &mut heapless::Vec<u8, N>) -> Result<(), crate::Error> {
        for item in self.as_slice() {
            item.encode(out)?;
        }

        Ok(())
    }
}

impl Encode for &[u8] {
    fn encode<const N: usize>(&self, out: &mut heapless::Vec<u8, N>) -> Result<(), crate::Error> {
        out.extend_from_slice(self).map_err(is_full)?;
        Ok(())
    }
}
