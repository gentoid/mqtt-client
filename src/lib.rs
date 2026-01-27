#![no_std]

pub mod parser;
pub mod protocol;

pub enum Error {
    InvalidFlags,
    MalformedRemainingLength,
    InvalidPacketType,
}
