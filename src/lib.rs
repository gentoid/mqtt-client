#![no_std]
#[allow(unused)]

pub mod buffer;
pub mod client;
pub mod packet;
pub mod parser;
pub mod protocol;
pub(crate) mod session;

#[derive(Debug)]
pub enum Error {
    InvalidFlags,
    MalformedRemainingLength,
    InvalidPacketType,
    MalformedPacket,
    InvalidConnectReturnCode,
    VectorIsFull,
    InvalidQoS,
    InvalidUtf8,
    EncodeNotImplemented,
    UnexpectedEof,
    TransportError,
    UnsupportedIncomingPacket,
    BufferTooSmall,
}
