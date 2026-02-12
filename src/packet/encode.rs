use crate::protocol;

pub(crate) trait EncodePacket {
    const PACKET_TYPE: protocol::PacketType;
    fn flags(&self) -> u8;
    fn required_space(&self) -> usize;
    fn encode_body(&self, cursor: &mut Cursor) -> Result<(), crate::Error>;
}

pub(crate) trait Encode {
    fn encode(&self, cursor: &mut Cursor) -> Result<(), crate::Error>;
    fn required_space(&self) -> usize;
}

trait RequiredSize {
    fn required_space(&self) -> usize;
}

pub(super) fn calculate_remaining_length(mut len: usize) -> Result<usize, crate::Error> {
    let mut i = 0;

    loop {
        len /= 128;
        i += 1;

        if len == 0 {
            break;
        }

        if i == 4 {
            return Err(crate::Error::MalformedPacket);
        }
    }

    Ok(i)
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

        if i == 4 {
            return Err(crate::Error::MalformedPacket);
        }
    }

    Ok(i)
}

pub(crate) struct Cursor<'buf> {
    buf: &'buf mut [u8],
    pos: usize,
}

impl<'buf> Cursor<'buf> {
    pub(crate) const fn new(buf: &'buf mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub(crate) fn written(&self) -> &[u8] {
        &self.buf[..self.pos]
    }

    pub(crate) fn write_u8(&mut self, byte: u8) -> Result<(), crate::Error> {
        self.ensure_remaining(1)?;
        self.buf[self.pos] = byte;
        self.pos += 1;

        Ok(())
    }

    fn write_u16(&mut self, value: u16) -> Result<(), crate::Error> {
        self.ensure_remaining(2)?;
        let [one, two] = value.to_be_bytes();
        self.buf[self.pos] = one;
        self.buf[self.pos + 1] = two;
        self.pos += 2;

        Ok(())
    }

    pub(crate) fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), crate::Error> {
        let len = bytes.len();
        self.ensure_remaining(len)?;

        self.buf[self.pos..self.pos + len].copy_from_slice(bytes);
        self.pos += len;

        Ok(())
    }

    pub(crate) fn write_binary_chunk(&mut self, bytes: &[u8]) -> Result<(), crate::Error> {
        self.write_u16(bytes.len() as u16)?;
        self.write_bytes(bytes)
    }

    fn write_utf8(&mut self, value: &str) -> Result<(), crate::Error> {
        self.write_binary_chunk(value.as_bytes())
    }

    fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    fn is_empty(&self) -> bool {
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

impl Encode for u16 {
    fn encode(&self, cursor: &mut Cursor) -> Result<(), crate::Error> {
        cursor.write_u16(*self)
    }

    fn required_space(&self) -> usize {
        2
    }
}

impl Encode for u8 {
    fn encode(&self, cursor: &mut Cursor) -> Result<(), crate::Error> {
        cursor.write_u8(*self)
    }

    fn required_space(&self) -> usize {
        1
    }
}

impl Encode for &str {
    fn encode(&self, cursor: &mut Cursor) -> Result<(), crate::Error> {
        cursor.write_utf8(&self)
    }

    fn required_space(&self) -> usize {
        self.as_bytes().len() + 2
    }
}

impl Encode for &[u8] {
    fn encode(&self, cursor: &mut Cursor) -> Result<(), crate::Error> {
        cursor.write_u16(self.len() as u16)?;
        cursor.write_bytes(&self)
    }

    fn required_space(&self) -> usize {
        self.len()
    }
}
