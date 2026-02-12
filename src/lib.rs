#![no_std]

pub mod buffer;
pub mod client;
pub(crate) mod incoming;
pub(crate) mod keep_alive;
pub mod packet;
pub(crate) mod packet_id_pool;
pub mod parser;
pub mod protocol;
pub(crate) mod session;
#[cfg(feature = "embassy")]
pub mod time;

pub use client::Client;
pub use packet::connect::Options as ConnectOptions;
pub use packet::publish::Msg as PublishMsg;
pub use session::Event;

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
    RemoteClosed,
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
