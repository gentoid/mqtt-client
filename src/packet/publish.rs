use crate::packet::{PacketId, QoS, encode::is_full, get_bytes, parse_packet_id, parse_utf8_str};

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

    // // @todo this cannot be a topic filter unlike subscribe, so maybe chec for allowed chars
    let topic = parse_utf8_str(body, &mut offset)?;

    let packet_id = if let QoS::AtMostOnce = flags.qos {
        None
    } else {
        let packet_id = parse_packet_id(body, &mut offset)?;

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

pub(super) fn encode<const N: usize>(out: &mut heapless::Vec<u8, N>, packet: &Publish<'_>) -> Result<(), crate::Error> {
    out.extend_from_slice(packet.topic.as_bytes()).map_err(is_full)?;
    out.extend_from_slice(&packet.packet_id.map(|id| id.0).unwrap_or(0).to_be_bytes()).map_err(is_full)?;
    out.extend_from_slice(&packet.payload).map_err(is_full)?;
    Ok(())
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
