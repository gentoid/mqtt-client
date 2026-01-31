use crate::{
    packet::{
        QoS, decode,
        encode::{self, Encode},
    },
    protocol::PacketType,
};

pub struct Connect<'a> {
    pub clean_session: bool,
    pub keep_alive: u16,
    pub client_id: &'a str,
    pub will: Option<Will<'a>>,
    pub username: Option<&'a str>,
    pub password: Option<&'a [u8]>,
}

impl<'buf> decode::DecodePacket<'buf> for Connect<'buf> {
    fn decode<'cursor>(
        flags: u8,
        cursor: &'cursor mut decode::Cursor<'buf>,
    ) -> Result<Self, crate::Error> {
        let protocol_name = cursor.read_utf8()?;
        if protocol_name != "MQTT" {
            return Err(crate::Error::MalformedPacket);
        }

        // @note: MQTT v3.1.1
        let level = cursor.read_u8()?;
        if level != 4 {
            return Err(crate::Error::MalformedPacket);
        }

        let flags = cursor.read_u8()?;
        if flags & 0b0000_0001 != 0 {
            return Err(crate::Error::MalformedPacket);
        }

        let clean_session = flags & 0b0000_0010 == 1;
        let will_flag = flags & 0b0000_0100 == 1;
        let qos = QoS::try_from((flags >> 3) & 0b11)?;
        let retain = flags & 0b0010_0000 == 1;
        let password_flag = flags & 0b0100_0000 == 1;
        let username_flag = flags & 0b1000_0000 == 1;

        let keep_alive = cursor.read_u16()?;
        // @todo: validate client id (see 3.1.3.1 Client Identifier of the MQTT 3.1.1 spec)
        let client_id = cursor.read_utf8()?;

        let will = if will_flag {
            Some(Will {
                topic: cursor.read_utf8()?,
                payload: cursor.read_binary_chunk()?,
                qos,
                retain,
            })
        } else {
            None
        };

        let username = if username_flag {
            Some(cursor.read_utf8()?)
        } else {
            None
        };

        let password = if password_flag {
            Some(cursor.read_binary_chunk()?)
        } else {
            None
        };

        Ok(Connect {
            clean_session,
            keep_alive,
            client_id,
            will,
            username,
            password,
        })
    }
}

impl<'buf> encode::EncodePacket for &Connect<'buf> {
    const PACKET_TYPE: PacketType = PacketType::Connect;

    fn flags(&self) -> u8 {
        0
    }

    fn required_space(&self) -> usize {
        let mut required = "MQTT".required_space()
            + 4u8.required_space()
            + 0u8.required_space()
            + self.keep_alive.required_space()
            + self.client_id.required_space();

            
        if let Some(will) = &self.will {
            required += will.topic.required_space();
            required += will.payload.required_space();
        }

        if let Some(username) = self.username {
            required += username.required_space();
        }

        if let Some(password) = self.password {
            required += password.required_space();
        }

        required
    }

    fn encode_body(&self, cursor: &mut encode::Cursor) -> Result<(), crate::Error> {
        "MQTT".encode(cursor)?;
        4u8.encode(cursor)?;

        let flags = (self.username.is_some() as u8) << 7
            | (self.password.is_some() as u8) << 6
            | (self.will.as_ref().map(|w| w.retain).unwrap_or(false) as u8) << 5
            | self.will.as_ref().map(|w| w.qos as u8).unwrap_or(0) << 3 // 2 bits
            | (self.will.is_some() as u8) << 2
            | (self.clean_session as u8) << 1;

        flags.encode(cursor)?;
        self.keep_alive.encode(cursor)?;
        self.client_id.encode(cursor)?;

        if let Some(will) = &self.will {
            will.topic.encode(cursor)?;
            will.payload.encode(cursor)?;
        }

        if let Some(username) = self.username {
            username.encode(cursor)?;
        }

        if let Some(password) = self.password {
            password.encode(cursor)?;
        }

        Ok(())
    }
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

impl<'buf> decode::DecodePacket<'buf> for ConnAck {
    fn decode<'cursor>(
        flags: u8,
        cursor: &'cursor mut decode::Cursor<'buf>,
    ) -> Result<Self, crate::Error> {
        let flags = cursor.read_u8()?;

        if flags & 0b1111_1110 != 0 {
            return Err(crate::Error::MalformedPacket);
        }

        let return_code = ConnectReturnCode::try_from(cursor.read_u8()?)?;

        let session_present = (flags & 0b0000_0001) == 1;

        cursor.expect_empty()?;

        Ok(ConnAck {
            return_code,
            session_present,
        })
    }
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

#[cfg(test)]
mod tests {
    use crate::{packet::decode::DecodePacket, protocol::PacketType};

    use super::*;

    #[test]
    fn connack_accepted() {
        let body = [0x00, 0x00];
        let mut cursor = decode::Cursor::new(&body);
        let packet = ConnAck::decode(0, &mut cursor).unwrap();

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
        let mut cursor = decode::Cursor::new(&body);
        assert!(ConnAck::decode(0, &mut cursor).is_err());
    }
}
