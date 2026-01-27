// #![no_std]

pub mod parser;
pub mod protocol;

#[derive(Debug)]
pub enum Error {
    InvalidFlags,
    MalformedRemainingLength,
    InvalidPacketType,
}
