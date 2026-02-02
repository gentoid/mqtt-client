use crate::packet::encode::{self, RequiredSize};

pub(crate) trait Provider<'buf> {
    type Buffer: AsMut<[u8]> + Into<Slice<'buf>>;
    type Error: core::fmt::Debug;

    fn provide(&mut self, len: usize) -> Result<Self::Buffer, Self::Error>;
}

#[derive(Debug)]
pub(crate) struct Slice<'buf> {
    inner: &'buf [u8],
}

impl<'buf> Slice<'buf> {
    pub fn as_bytes(&self) -> &[u8] {
        self.inner
    }
}

impl<'buf> Slice<'buf> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<'buf> encode::Encode for Slice<'buf> {
    fn encode(&self, cursor: &mut encode::Cursor) -> Result<(), crate::Error> {
        todo!()
    }

    fn required_space(&self) -> usize {
        self.inner.len() + 2
    }
}

impl<'buf> From<&'buf mut [u8]> for Slice<'buf> {
    fn from(value: &'buf mut [u8]) -> Self {
        Self { inner: value }
    }
}

impl<'buf> From<&'buf [u8]> for Slice<'buf> {
    fn from(value: &'buf [u8]) -> Self {
        Self { inner: value }
    }
}

impl<'buf> From<&'buf str> for Slice<'buf> {
    fn from(value: &'buf str) -> Self {
        Self {
            inner: value.as_bytes(),
        }
    }
}

impl<'buf> PartialEq<&[u8]> for Slice<'buf> {
    fn eq(&self, other: &&[u8]) -> bool {
        self.inner == *other
    }
}

pub(crate) struct Bump<'buf> {
    buf: &'buf mut [u8],
    index: usize,
}

impl<'buf> Bump<'buf> {
    pub(crate) fn new(buf: &'buf mut [u8]) -> Self {
        Self { buf, index: 0 }
    }
}

impl<'buf> Provider<'buf> for Bump<'buf> {
    type Buffer = &'buf mut [u8];
    type Error = crate::Error;

    fn provide(&mut self, len: usize) -> Result<Self::Buffer, Self::Error> {
        if self.index + len > self.buf.len() {
            return Err(crate::Error::UnexpectedEof);
        }

        // SAFETY:
        // - self.buf is an exclusive &mut [u8]
        // - self.index is monotonically increasing
        // - returned slices never overlap
        // - slices do not outlive 'buf

        let start = self.index;
        let ptr = unsafe { self.buf.as_mut_ptr().add(start) };
        self.index += len;

        let slice = unsafe { core::slice::from_raw_parts_mut(ptr, len) };
        Ok(slice)
    }
}

#[derive(Debug)]
pub struct String<'buf> {
    inner: Slice<'buf>,
}

impl<'buf> From<Slice<'buf>> for String<'buf> {
    fn from(value: Slice<'buf>) -> Self {
        Self { inner: value }
    }
}

impl<'buf> From<&'buf mut [u8]> for String<'buf> {
    fn from(value: &'buf mut [u8]) -> Self {
        Self {
            inner: Slice { inner: value },
        }
    }
}

#[cfg(test)]
impl<'buf> From<&'buf str> for String<'buf> {
    fn from(value: &'buf str) -> Self {
        Self {
            inner: Slice::from(value),
        }
    }
}

impl<'buf> PartialEq<&str> for String<'buf> {
    fn eq(&self, other: &&str) -> bool {
        self.inner.as_bytes() == other.as_bytes()
    }
}

impl<'buf> encode::Encode for String<'buf> {
    fn encode(&self, cursor: &mut encode::Cursor) -> Result<(), crate::Error> {
        todo!()
    }

    fn required_space(&self) -> usize {
        self.inner.required_space()
    }
}
