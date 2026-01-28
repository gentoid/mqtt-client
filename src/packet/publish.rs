use crate::packet::{PacketId, QoS};

pub struct Publish<'a> {
    pub flags: Flags,
    pub topic: &'a str,
    pub packet_id: Option<PacketId>,
    pub payload: &'a [u8],
}

pub struct Flags {
    pub dup: bool,
    pub qos: QoS,
    pub retain: bool,
}

impl TryFrom<u8> for Flags {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let dup = value & 0b1000 == 1;
        let qos = QoS::try_from((value >> 1) & 0b11)?;
        let retain = value & 0b0001 == 1;

        Ok(Self { dup, qos, retain })
    }
}

pub(super) fn parse<'a>(flags: u8, body: &'a [u8]) -> Result<Publish<'a>, crate::Error> {
    let flags = Flags::try_from(flags)?;

    let mut offset = 0;

    let mut get_bytes = |len: usize| {
        let (bytes, new_offset) = super::get_bytes(body, offset, len)?;
        offset = new_offset;
        Ok(bytes)
    };

    let bytes = get_bytes(2)?;
    let topic_len = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;
    let topic_bytes = get_bytes(topic_len)?;

    // @todo this cannot be a topic filter unlike subscribe, so maybe chec for allowed chars
    let topic = core::str::from_utf8(topic_bytes).map_err(|_| crate::Error::InvalidUtf8)?;

    let packet_id = if let QoS::AtMostOnce = flags.qos {
        None
    } else {
        let bytes = get_bytes(2)?;
        let packet_id = PacketId::try_from(bytes)?;

        Some(packet_id)
    };

    let payload = &body[offset..];

    Ok(Publish {
        flags,
        topic,
        packet_id,
        payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_packet() {
        let flags = 0b0000_0000;
        let body = [
            0x00, 0x05, b't', b'o', b'p', b'i', b'c', b'p', b'a', b'y', b'l', b'o', b'a', b'd',
        ];

        let packet = parse(flags, &body).unwrap();

        assert!(matches!(
            packet.flags,
            Flags {
                dup: false,
                qos: QoS::AtMostOnce,
                retain: false
            }
        ));
        assert_eq!(packet.packet_id, None);
        assert_eq!(packet.topic, "topic");
        assert_eq!(packet.payload, "payload".as_bytes());
    }
}
