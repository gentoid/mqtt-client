#![no_std]

pub mod protocol;

pub enum Error {
    InvalidFlags,
    MalformedRemainingLength,
}
