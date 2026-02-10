pub(crate) struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub(crate) const fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8, crate::Error> {
        self.ensure_remaining(1)?;
        let res = self.buf[self.pos];
        self.pos += 1;
        Ok(res)
    }

    pub(crate) fn read_u16(&mut self) -> Result<u16, crate::Error> {
        self.ensure_remaining(2)?;
        let res = u16::from_be_bytes([self.buf[self.pos], self.buf[self.pos + 1]]);
        self.pos += 2;
        Ok(res)
    }

    pub(crate) fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], crate::Error> {
        self.ensure_remaining(len)?;
        let start = self.pos;
        self.pos += len;
        Ok(&self.buf[start..self.pos])
    }

    pub(crate) fn read_binary(&mut self) -> Result<&'a [u8], crate::Error> {
        let len = self.read_u16()? as usize;
        self.read_bytes(len)
    }

    pub(crate) fn read_utf8(&mut self) -> Result<&'a str, crate::Error> {
        let len = self.read_u16()? as usize;
        let bytes = self.read_bytes(len)?;

        core::str::from_utf8(bytes).map_err(|_| crate::Error::InvalidUtf8)
    }

    pub(crate) fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    pub(crate) fn expect_exact_len(&self, len: usize) -> Result<(), crate::Error> {
        if self.remaining() != len {
            Err(crate::Error::MalformedPacket)
        } else {
            Ok(())
        }
    }

    pub(crate) fn expect_empty(&self) -> Result<(), crate::Error> {
        self.expect_exact_len(0)
    }

    fn ensure_remaining(&self, n: usize) -> Result<(), crate::Error> {
        if self.remaining() < n {
            Err(crate::Error::UnexpectedEof)
        } else {
            Ok(())
        }
    }
}
