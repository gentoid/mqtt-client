pub trait Decode<'buf>: Sized {
    fn decode<'cursor>(cursor: &'cursor mut Cursor<'buf>) -> Result<Self, crate::Error>;
}

pub trait DecodePacket<'buf>: Sized {
    fn decode<'cursor>(flags: u8, cursor: &'cursor mut Cursor<'buf>) -> Result<Self, crate::Error>;
}

pub struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub const fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn read_u8(&mut self) -> Result<u8, crate::Error> {
        self.ensure_remaining(1)?;
        let res = self.buf[self.pos];
        self.pos += 1;

        Ok(res)
    }

    pub fn read_u16(&mut self) -> Result<u16, crate::Error> {
        self.ensure_remaining(2)?;
        let res = u16::from_be_bytes([self.buf[self.pos], self.buf[self.pos + 1]]);
        self.pos += 2;

        Ok(res)
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], crate::Error> {
        self.ensure_remaining(len)?;
        let res = &self.buf[self.pos..self.pos + len];
        self.pos += len;

        Ok(res)
    }

    pub fn read_binary_chunk(&mut self) -> Result<&'a [u8], crate::Error> {
        let len = self.read_u16()? as usize;
        self.read_bytes(len)
    }

    pub fn read_utf8(&mut self) -> Result<&'a str, crate::Error> {
        let len = self.read_u16()? as usize;
        let bytes = self.read_bytes(len)?;

        core::str::from_utf8(bytes).map_err(|_| crate::Error::InvalidUtf8)
    }

    pub fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    pub fn expect_empty(&self) -> Result<(), crate::Error> {
        if !self.is_empty() {
            Err(crate::Error::MalformedPacket)
        } else {
            Ok(())
        }
    }

    fn ensure_remaining(&self, n: usize) -> Result<(), crate::Error> {
        if self.remaining() < n {
            Err(crate::Error::UnexpectedEof)
        } else {
            Ok(())
        }
    }
}
