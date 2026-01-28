#![no_std]

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
    TooSmallSubAckVector,
    InvalidQoS,
    InvalidUtf8,
}
