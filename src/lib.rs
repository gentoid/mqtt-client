#![no_std]
#[allow(unused)]
pub mod buffer;
pub mod client;
pub(crate) mod incoming;
pub mod packet;
pub(crate) mod packet_id_pool;
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
    TimeError,
    TimedOut,
    ProtocolViolation,
    NoPacketIdAvailable,
    SubVectorIsFull,
    WrongTopicToUnsubscribe,
    Unsubscribed,
    PingOutstanding,
    QueueRangeError,
}
