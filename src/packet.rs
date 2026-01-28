use crate::protocol::{FixedHeader, PacketType};

pub enum Packet {
    PingReq,
    PingResp,
    Disconnect,
    ConnAck(ConnAck),
}

pub struct ConnAck {
    pub session_present: bool,
    pub return_code: ConnectReturnCode,
}

// @note: for MQTT 5.0 it is a whole another story
#[repr(u8)]
pub enum ConnectReturnCode {
    Accepted = 0,
    UnacceptableProtocolVersion = 1,
    IdentifierRejected = 2,
    ServerUnavailable = 3,
    BadUserNameOrPassword = 4,
    NotAuthorized = 5,
}

impl TryFrom<u8> for ConnectReturnCode {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let code = match value {
            0 => Self::Accepted,
            1 => Self::UnacceptableProtocolVersion,
            2 => Self::IdentifierRejected,
            3 => Self::ServerUnavailable,
            4 => Self::BadUserNameOrPassword,
            5 => Self::NotAuthorized,
            _ => return Err(crate::Error::InvalidConnectReturnCode),
        };

        Ok(code)
    }
}

fn parse<'a>(header: FixedHeader, body: &'a [u8]) -> Result<Packet, crate::Error> {
    if header.remaining_len as usize != body.len() {
        return Err(crate::Error::MalformedPacket);
    }

    match header.packet_type {
        PacketType::Connect => todo!(),
        PacketType::ConnAck => todo!(),
        PacketType::Publish => todo!(),
        PacketType::PubAck => todo!(),
        PacketType::PubRec => todo!(),
        PacketType::PubRel => todo!(),
        PacketType::PubComp => todo!(),
        PacketType::Subscribe => todo!(),
        PacketType::SubAck => todo!(),
        PacketType::Unsubscribe => todo!(),
        PacketType::UnsubAck => todo!(),
        PacketType::PingReq => expect_body_len(body, 0).map(|_| Packet::PingReq),
        PacketType::PingResp => expect_body_len(body, 0).map(|_| Packet::PingResp),
        PacketType::Disconnect => expect_body_len(body, 0).map(|_| Packet::Disconnect),
    }
}

fn expect_body_len(body: &[u8], len: usize) -> Result<(), crate::Error> {
    if body.len() == len {
        return Ok(());
    }

    Err(crate::Error::MalformedPacket)
}

fn parse_connack(body: &[u8]) -> Result<Packet, crate::Error> {
    expect_body_len(body, 2)?;

    let flags = body[0];
    let return_code = ConnectReturnCode::try_from(body[1])?;

    if flags & 0b1111_1110 != 0 {
        return Err(crate::Error::MalformedPacket);
    }

    let session_present = (flags & 0b0000_0001) == 1;

    Ok(Packet::ConnAck(ConnAck {
        return_code,
        session_present,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connack_accepted() {
        let body = [0x00, 0x00];
        let packet = parse_connack(&body).unwrap();

        assert!(matches!(
            packet,
            Packet::ConnAck(ConnAck {
                session_present: false,
                return_code: ConnectReturnCode::Accepted
            })
        ));
    }

    #[test]
    fn connack_invalid_flags() {
        let body = [0b0000_0010, 0x00];
        assert!(parse_connack(&body).is_err());
    }
}
