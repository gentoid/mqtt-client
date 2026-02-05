use crate::{
    buffer,
    packet::{
        QoS,
        decode::{self, CursorExt},
        encode::{self, Encode},
    },
    protocol::PacketType,
};

pub struct Options<'a> {
    pub clean_session: bool,
    pub keep_alive: u16,
    pub client_id: &'a str,
    pub will: Option<WillOptions<'a>>,
    pub username: Option<&'a str>,
    pub password: Option<&'a str>,
}

pub struct WillOptions<'a> {
    pub qos: QoS,
    pub retain: bool,
    pub topic: &'a str,
    pub payload: &'a [u8],
}

#[derive(Debug)]
pub struct Connect<'a> {
    pub clean_session: bool,
    pub keep_alive: u16,
    pub client_id: buffer::String<'a>,
    pub will: Option<Will<'a>>,
    pub username: Option<buffer::String<'a>>,
    pub password: Option<buffer::Slice<'a>>,
}

impl <'a, 'b> From<Options<'a>> for Connect<'b> {
    fn from(value: Options<'a>) -> Self {
        todo!()
    }
}

impl<'buf, P> decode::DecodePacket<'buf, P> for Connect<'buf>
where
    P: buffer::Provider<'buf>,
{
    fn decode(
        cursor: &mut decode::Cursor,
        provider: &'buf mut P,
        _: u8,
    ) -> Result<Self, crate::Error> {
        let protocol_name = cursor.read_utf8(provider)?;
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
        let len = cursor.read_u16()? as usize;
        let mut buf = provider
            .provide(len)
            .map_err(|_| crate::Error::UnexpectedEof)?;
        cursor.consume(buf.as_mut())?;

        let client_id = buffer::String::from(buf.into());

        let will = if will_flag {
            Some(Will {
                topic: cursor.read_utf8(provider)?,
                payload: cursor.read_binary(provider)?,
                qos,
                retain,
            })
        } else {
            None
        };

        let username = if username_flag {
            Some(cursor.read_utf8(provider)?)
        } else {
            None
        };

        let password = if password_flag {
            Some(cursor.read_binary(provider)?)
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

        if let Some(username) = &self.username {
            required += username.required_space();
        }

        if let Some(password) = &self.password {
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

        if let Some(username) = &self.username {
            username.encode(cursor)?;
        }

        if let Some(password) = &self.password {
            password.encode(cursor)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Will<'a> {
    pub qos: QoS,
    pub retain: bool,
    pub topic: buffer::String<'a>,
    pub payload: buffer::Slice<'a>,
}

pub struct ConnAck {
    pub session_present: bool,
    pub return_code: ConnectReturnCode,
}

impl<'buf, P> decode::DecodePacket<'buf, P> for ConnAck
where
    P: buffer::Provider<'buf>,
{
    fn decode(cursor: &mut decode::Cursor, _: &mut P, _: u8) -> Result<Self, crate::Error> {
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
#[derive(PartialEq)]
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
    use crate::{
        buffer,
        packet::{decode::DecodePacket, encode::EncodePacket},
    };

    use super::*;

    #[test]
    fn connack_accepted() {
        let body = [0x00, 0x00];
        let mut cursor = decode::Cursor::new(&body);
        let mut buf = [0u8; 16];
        let mut buf = buffer::Bump::new(&mut buf[..]);
        let packet = ConnAck::decode(&mut cursor, &mut buf, 0).unwrap();

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
        let mut buf = [0u8; 16];
        let mut buf = buffer::Bump::new(&mut buf[..]);
        assert!(ConnAck::decode(&mut cursor, &mut buf, 0).is_err());
    }

    #[test]
    fn connect_encode_flags() {
        let connect = Connect {
            client_id: buffer::String::from("Client"),
            clean_session: true,
            keep_alive: 60,
            will: None,
            username: None,
            password: None,
        };

        let mut buf = [0u8; 32];
        let mut cursor = encode::Cursor::new(&mut buf);
        (&connect).encode_body(&mut cursor).unwrap();

        // [
        //   0, 4,   77, 81, 84, 84,    // "MQTT"
        //   4,                         // MQT version
        //   2,                         // Flags
        //   0, 60,                     // keep_alive
        //   0, 6,   67, 108, 105, 101, 110, 116    // "Client"
        // ]
        assert_eq!(cursor.written().len(), 18);
        assert_eq!(&buf[2..6], b"MQTT");
        assert_eq!(buf[6], 4);
        assert_eq!(buf[7], 0b0000_0010);
        assert_eq!(u16::from_be_bytes([buf[8], buf[9]]), 60);

        let len = u16::from_be_bytes([buf[10], buf[11]]) as usize;
        assert_eq!(&buf[12..12 + len], b"Client");
    }

    #[test]
    fn connect_encode_with_will_username_password() {
        let will = Will {
            topic: buffer::String::from("topic1"),
            payload: buffer::Slice::from(b"heavy-load".as_slice()),
            qos: QoS::AtLeastOnce,
            retain: true,
        };

        let connect = Connect {
            client_id: buffer::String::from("Client 2"),
            clean_session: false,
            keep_alive: 120,
            will: Some(will),
            username: Some(buffer::String::from("user 1")),
            password: Some(buffer::Slice::from(b"long-pass".as_slice())),
        };

        let mut buf = [0u8; 64];
        let mut cursor = encode::Cursor::new(&mut buf);
        (&connect).encode_body(&mut cursor).unwrap();

        //  [
        //    0, 4,   77, 81, 84, 84,   // "MQTT"
        //    4,                        // MQTT version
        //    236,                      // Flags
        //    0, 120,                   // keep_alive
        //    0, 8,   67, 108, 105, 101, 110, 116, 32, 50,              // "Client 2"
        //    0, 6,   116, 111, 112, 105, 99, 49,                       // "topic1"
        //    0, 10,  104, 101, 97, 118, 121, 45, 108, 111, 97, 100,    // "heavy-load"
        //    0, 6,   117, 115, 101, 114, 32, 49,                       // "user 1"
        //    0, 9,   108, 111, 110, 103, 45, 112, 97, 115, 115         // "long-pass"
        //  ]

        assert_eq!(cursor.written().len(), 59);

        assert_eq!(buf[7], 0b1110_1100);
        assert_eq!(u16::from_be_bytes([buf[8], buf[9]]), 120);

        let len = u16::from_be_bytes([buf[10], buf[11]]) as usize;
        assert_eq!(&buf[12..12 + len], b"Client 2");

        let len = u16::from_be_bytes([buf[20], buf[21]]) as usize;
        assert_eq!(&buf[22..22 + len], b"topic1");

        let len = u16::from_be_bytes([buf[28], buf[29]]) as usize;
        assert_eq!(&buf[30..30 + len], b"heavy-load");

        let len = u16::from_be_bytes([buf[40], buf[41]]) as usize;
        assert_eq!(&buf[42..42 + len], b"user 1");

        let len = u16::from_be_bytes([buf[48], buf[49]]) as usize;
        assert_eq!(&buf[50..50 + len], b"long-pass");
    }

    #[test]
    fn connect_with_invalid_flags() {
        let bytes = [
            0x00,        // "MQTT"
            0x04,        // |
            b'M',        // |
            b'Q',        // |
            b'T',        // |
            b'T',        // ___
            0x04,        // MQTT version
            0b0000_0001, // Flags - invalid
            0x00,        // keep_alive = 60
            0x3C,        // ___
        ];
        let mut cursor = decode::Cursor::new(&bytes);
        let mut buf = [0u8; 32];
        let mut buf = buffer::Bump::new(&mut buf[..]);
        let err = Connect::decode(&mut cursor, &mut buf, 0).unwrap_err();

        assert!(matches!(err, crate::Error::MalformedPacket));
    }
}
