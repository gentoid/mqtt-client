// use crate::buffer;

// pub trait Decode<'buf>: Sized
// // where
//     // P: buffer::Provider<'buf>,
// {
//     fn decode(cursor: &mut Cursor) -> Result<Self, crate::Error>;
// }

// pub trait DecodePacket<'buf, P>: Sized
// where
//     P: buffer::Provider<'buf>,
// {
//     fn decode(cursor: &mut Cursor, provider: &'buf mut P, flags: u8) -> Result<Self, crate::Error>;
// }

// pub struct Cursor<'a> {
//     buf: &'a [u8],
//     pos: usize,
// }

// impl<'a> Cursor<'a> {
//     pub const fn new(buf: &'a [u8]) -> Self {
//         Self { buf, pos: 0 }
//     }

//     pub fn read_u8(&mut self) -> Result<u8, crate::Error> {
//         self.ensure_remaining(1)?;
//         let res = self.buf[self.pos];
//         self.pos += 1;

//         Ok(res)
//     }

//     pub fn read_u16(&mut self) -> Result<u16, crate::Error> {
//         self.ensure_remaining(2)?;
//         let res = u16::from_be_bytes([self.buf[self.pos], self.buf[self.pos + 1]]);
//         self.pos += 2;

//         Ok(res)
//     }

//     pub fn consume<'buf>(&mut self, slice: &'buf mut [u8]) -> Result<(), crate::Error> {
//         let len = slice.len();
//         self.ensure_remaining(len)?;

//         let start = self.pos;
//         self.pos += len;

//         slice.copy_from_slice(&self.buf[start..self.pos]);

//         Ok(())
//     }

//     // pub fn read_binary_chunk(&mut self) -> Result<&'a [u8], crate::Error> {
//     //     let len = self.read_u16()? as usize;
//     //     self.consume(len)
//     // }

//     // pub fn read_utf8(&mut self) -> Result<&'a str, crate::Error> {
//     //     let len = self.read_u16()? as usize;
//     //     let bytes = self.consume(len)?;

//     //     core::str::from_utf8(bytes).map_err(|_| crate::Error::InvalidUtf8)
//     // }

//     pub fn remaining(&self) -> usize {
//         self.buf.len() - self.pos
//     }

//     pub fn is_empty(&self) -> bool {
//         self.remaining() == 0
//     }

//     pub fn expect_empty(&self) -> Result<(), crate::Error> {
//         if !self.is_empty() {
//             Err(crate::Error::MalformedPacket)
//         } else {
//             Ok(())
//         }
//     }

//     fn ensure_remaining(&self, n: usize) -> Result<(), crate::Error> {
//         if self.remaining() < n {
//             Err(crate::Error::UnexpectedEof)
//         } else {
//             Ok(())
//         }
//     }
// }

// pub(crate) trait CursorExt<'buf, P>
// where
//     P: buffer::Provider<'buf>,
// {
//     fn read_utf8(&mut self, provider: &mut P) -> Result<buffer::String<'buf>, crate::Error>;
//     fn read_binary(&mut self, provider: &mut P) -> Result<buffer::Slice<'buf>, crate::Error>;
//     fn read_all(&mut self, provider: &mut P) -> Result<buffer::Slice<'buf>, crate::Error>;
// }

// impl<'buf, P> CursorExt<'buf, P> for Cursor<'_>
// where
//     P: buffer::Provider<'buf>,
// {
//     fn read_utf8(&mut self, provider: &mut P) -> Result<buffer::String<'buf>, crate::Error> {
//         Ok(buffer::String::from(self.read_binary(provider)?))
//     }

//     fn read_binary(&mut self, provider: &mut P) -> Result<buffer::Slice<'buf>, crate::Error> {
//         let len = self.read_u16()? as usize;
//         let mut buf = provider
//             .provide(len)
//             .map_err(|_| crate::Error::UnexpectedEof)?;

//         self.consume(buf.as_mut())?;
//         Ok(buf.into())
//     }

//     fn read_all(&mut self, provider: &mut P) -> Result<buffer::Slice<'buf>, crate::Error> {
//         let len = self.remaining();
//         let mut buf = provider
//             .provide(len)
//             .map_err(|_| crate::Error::UnexpectedEof)?;

//         self.consume(buf.as_mut())?;
//         Ok(buf.into())
//     }
// }
