use crate::packet::{QoS, expect_body_len, get_bytes, parse_binary_data, parse_u16, parse_utf8_str};

pub struct Connect<'a> {
    pub clean_session: bool,
    pub keep_alive: u16,
    pub client_id: &'a str,
    pub will: Option<Will<'a>>,
    pub username: Option<&'a str>,
    pub password: Option<&'a [u8]>,
}
pub struct Will<'a> {
    pub qos: QoS,
    pub retain: bool,
    pub topic: &'a str,
    pub payload: &'a [u8],
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

pub(super) fn parse<'a>(body: &'a [u8]) -> Result<Connect<'a>, crate::Error> {
    let mut offset = 0;

    let protocol_name = parse_utf8_str(body, &mut offset)?;
    if protocol_name != "MQTT" {
        return Err(crate::Error::MalformedPacket);
    }

    // @note: MQTT v3.1.1
    let level = get_bytes(body, &mut offset)(1)?[0];
    if level != 4 {
        return Err(crate::Error::MalformedPacket);
    }

    let flags = get_bytes(body, &mut offset)(1)?[0];

    if flags & 0b0000_0001 != 0 {
        return Err(crate::Error::MalformedPacket);
    }

    let clean_session = flags & 0b0000_0010 == 1;
    let will_flag = flags & 0b0000_0100 == 1;
    let qos = QoS::try_from((flags >> 3) & 0b11)?;
    let retain = flags & 0b0010_0000 == 1;
    let password_flag = flags & 0b0100_0000 == 1;
    let username_flag = flags & 0b1000_0000 == 1;

    let keep_alive = parse_u16(body, &mut offset)?;
    // @todo: validate client id (see 3.1.3.1 Client Identifier of the MQTT 3.1.1 spec)
    let client_id = parse_utf8_str(body, &mut offset)?;

    let will = if will_flag {
        Some(Will {
            topic: parse_utf8_str(body, &mut offset)?,
            payload: parse_binary_data(body, &mut offset)?,
            qos,
            retain,
        })
    } else {
        None
    };

    let username = if username_flag {
        Some(parse_utf8_str(body, &mut offset)?)
    } else {
        None
    };

    let password = if password_flag {
        Some(parse_binary_data(body, &mut offset)?)
    } else {
        None
    };


    Ok(Connect { clean_session, keep_alive, client_id, will, username, password })
}

pub(super) fn parse_connack(body: &[u8]) -> Result<ConnAck, crate::Error> {
    expect_body_len(body, 2)?;

    let flags = body[0];

    if flags & 0b1111_1110 != 0 {
        return Err(crate::Error::MalformedPacket);
    }

    let return_code = ConnectReturnCode::try_from(body[1])?;

    let session_present = (flags & 0b0000_0001) == 1;

    Ok(ConnAck {
        return_code,
        session_present,
    })
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
            ConnAck {
                session_present: false,
                return_code: ConnectReturnCode::Accepted
            }
        ));
    }

    #[test]
    fn connack_invalid_flags() {
        let body = [0b0000_0010, 0x00];
        assert!(parse_connack(&body).is_err());
    }
}
