use crate::packet::{Packet, publish, subscribe, unsubscribe};

pub trait Encode {
    fn encode(&self, cursor: &mut Cursor) -> Result<(), crate::Error>;
}

pub trait RequiredSize {
    fn required_space(&self) -> usize;
}

pub(super) fn remaining_length(mut len: usize, cursor: &mut Cursor) -> Result<usize, crate::Error> {
    let mut i = 0;

    loop {
        let mut byte = (len % 128) as u8;
        len /= 128;

        if len > 0 {
            byte |= 0x80;
        }

        cursor.write_u8(byte)?;
        i += 1;

        if len == 0 {
            break;
        }

        if len == 4 {
            return Err(crate::Error::MalformedPacket);
        }
    }

    Ok(i)
}

pub struct Cursor<'buf> {
    buf: &'buf mut [u8],
    pos: usize,
}

impl<'buf> Cursor<'buf> {
    pub const fn new(buf: &'buf mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn write_u8(&mut self, byte: u8) -> Result<(), crate::Error> {
        self.ensure_remaining(1)?;
        self.buf[self.pos] = byte;
        self.pos += 1;

        Ok(())
    }

    pub fn write_u16(&mut self, value: u16) -> Result<(), crate::Error> {
        self.ensure_remaining(2)?;
        let [one, two] = value.to_be_bytes();
        self.buf[self.pos] = one;
        self.buf[self.pos + 1] = two;
        self.pos += 2;

        Ok(())
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), crate::Error> {
        let len = bytes.len();
        self.ensure_remaining(len)?;
        let res = &self.buf[self.pos..self.pos + len].copy_from_slice(bytes);
        self.pos += len;

        Ok(())
    }

    pub fn write_binary_chunk(&mut self, bytes: &[u8]) -> Result<(), crate::Error> {
        self.write_u16(bytes.len() as u16)?;
        self.write_bytes(bytes)
    }

    pub fn write_utf8(&mut self, value: &str) -> Result<(), crate::Error> {
        self.write_binary_chunk(value.as_bytes())
    }

    pub fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    fn ensure_remaining(&self, n: usize) -> Result<(), crate::Error> {
        if self.remaining() < n {
            Err(crate::Error::UnexpectedEof)
        } else {
            Ok(())
        }
    }
}

// pub(super) fn empty_body<const N: usize>(
//     cursor: &mut Cursor,
//     id: u8,
// ) -> Result<(), crate::Error> {
//     out.push(id).map_err(is_full)?;
//     out.push(0).map_err(is_full)?;
//     Ok(())
// }

pub(super) fn is_full<T>(_: T) -> crate::Error {
    crate::Error::VectorIsFull
}

impl Encode for u16 {
    fn encode(&self, cursor: &mut Cursor) -> Result<(), crate::Error> {
        cursor.write_u16(*self)
    }
}

impl Encode for u8 {
    fn encode(&self, cursor: &mut Cursor) -> Result<(), crate::Error> {
        cursor.write_u8(*self)
    }
}

impl Encode for &str {
    fn encode(&self, cursor: &mut Cursor) -> Result<(), crate::Error> {
        cursor.write_utf8(&self)
    }
}

impl RequiredSize for &str {
    fn required_space(&self) -> usize {
        self.as_bytes().len() + 2
    }
}

// impl<T: Encode, const P: usize> Encode for heapless::Vec<T, P> {
//     fn encode(&self, cursor: &mut Cursor) -> Result<(), crate::Error> {
//         for item in self.as_slice() {
//             item.encode(out)?;
//         }

//         Ok(())
//     }
// }

impl Encode for &[u8] {
    fn encode(&self, cursor: &mut Cursor) -> Result<(), crate::Error> {
        cursor.write_bytes(&self)
    }
}
