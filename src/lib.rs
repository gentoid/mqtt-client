#![no_std]
#[allow(unused)]

pub mod buffer;
pub mod packet;
pub mod parser;
pub mod protocol;

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
}
