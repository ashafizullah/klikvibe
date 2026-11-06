//! The encrypted channel to a phone: Noise NNpsk0 (X25519, ChaCha20-Poly1305,
//! SHA-256) with the key the phone got from the pairing QR code. The phone
//! starts the handshake; a phone without the key can't finish it, and nothing
//! in between (the Wi-Fi, the relay) can read or change the messages.

use snow::{Builder, HandshakeState, TransportState};

pub const PATTERN: &str = "Noise_NNpsk0_25519_ChaChaPoly_SHA256";
/// The largest Noise message, and the largest plaintext that fits in one.
pub const MAX_MESSAGE: usize = 65535;
pub const MAX_PLAINTEXT: usize = MAX_MESSAGE - 16;

#[derive(Debug)]
pub struct NoiseError;

impl From<snow::Error> for NoiseError {
    fn from(_: snow::Error) -> Self {
        NoiseError
    }
}

fn builder(psk: &[u8; 32]) -> Result<Builder<'_>, NoiseError> {
    Ok(Builder::new(PATTERN.parse()?).psk(0, psk)?)
}

/// The desktop's side: reads the phone's first message and returns the answer
/// to send back, and the channel.
pub fn respond(psk: &[u8; 32], first: &[u8]) -> Result<(Vec<u8>, Channel), NoiseError> {
    let mut hs: HandshakeState = builder(psk)?.build_responder()?;
    let mut buf = vec![0u8; MAX_MESSAGE];
    hs.read_message(first, &mut buf)?;
    let n = hs.write_message(&[], &mut buf)?;
    buf.truncate(n);
    Ok((buf, Channel(hs.into_transport_mode()?)))
}

pub struct Channel(TransportState);

impl Channel {
    pub fn seal(&mut self, plain: &[u8]) -> Result<Vec<u8>, NoiseError> {
        if plain.len() > MAX_PLAINTEXT {
            return Err(NoiseError);
        }
        let mut out = vec![0u8; plain.len() + 16];
        let n = self.0.write_message(plain, &mut out)?;
        out.truncate(n);
        Ok(out)
    }

    pub fn open(&mut self, sealed: &[u8]) -> Result<Vec<u8>, NoiseError> {
        let mut out = vec![0u8; sealed.len()];
        let n = self.0.read_message(sealed, &mut out)?;
        out.truncate(n);
        Ok(out)
    }
}

/// The phone's side, for tests (and for anyone writing a client in Rust).
#[cfg(test)]
pub fn initiate(psk: &[u8; 32]) -> Result<(Vec<u8>, HandshakeState), NoiseError> {
    let mut hs = builder(psk)?.build_initiator()?;
    let mut buf = vec![0u8; MAX_MESSAGE];
    let n = hs.write_message(&[], &mut buf)?;
    buf.truncate(n);
    Ok((buf, hs))
}

#[cfg(test)]
pub fn finish(mut hs: HandshakeState, answer: &[u8]) -> Result<Channel, NoiseError> {
    let mut buf = vec![0u8; MAX_MESSAGE];
    hs.read_message(answer, &mut buf)?;
    Ok(Channel(hs.into_transport_mode()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_round_trip() {
        let psk = [7u8; 32];
        let (first, hs) = initiate(&psk).unwrap();
        let (answer, mut desktop) = respond(&psk, &first).unwrap();
        let mut phone = finish(hs, &answer).unwrap();
        let sealed = phone.seal(br#"{"t":"hello"}"#).unwrap();
        assert_eq!(desktop.open(&sealed).unwrap(), br#"{"t":"hello"}"#);
        let back = desktop.seal(b"pong").unwrap();
        assert_eq!(phone.open(&back).unwrap(), b"pong");
        // Tampering is caught.
        let mut bad = desktop.seal(b"x").unwrap();
        bad[0] ^= 1;
        assert!(phone.open(&bad).is_err());
    }

    #[test]
    fn the_wrong_key_fails() {
        let (first, _) = initiate(&[1u8; 32]).unwrap();
        assert!(respond(&[2u8; 32], &first).is_err());
    }

    #[test]
    fn too_large_is_refused() {
        let psk = [7u8; 32];
        let (first, _) = initiate(&psk).unwrap();
        let (_, mut desktop) = respond(&psk, &first).unwrap();
        assert!(desktop.seal(&vec![0u8; MAX_PLAINTEXT + 1]).is_err());
        assert!(desktop.seal(&vec![0u8; MAX_PLAINTEXT]).is_ok());
    }

    /// Fixed keys, so the phone app's own Noise code can be checked against
    /// this one (cloud/mobile/test/noise_test.dart uses the same values).
    #[test]
    fn known_answer() {
        let psk: [u8; 32] = std::array::from_fn(|i| i as u8);
        let ie: [u8; 32] = std::array::from_fn(|i| 0x20 + i as u8);
        let re: [u8; 32] = std::array::from_fn(|i| 0x40 + i as u8);
        let p = PATTERN.parse().unwrap();
        let mut init = Builder::new(p).psk(0, &psk).unwrap().fixed_ephemeral_key_for_testing_only(&ie).build_initiator().unwrap();
        let p = PATTERN.parse().unwrap();
        let mut resp = Builder::new(p).psk(0, &psk).unwrap().fixed_ephemeral_key_for_testing_only(&re).build_responder().unwrap();
        let mut m1 = [0u8; 128];
        let n1 = init.write_message(&[], &mut m1).unwrap();
        let mut tmp = [0u8; 128];
        resp.read_message(&m1[..n1], &mut tmp).unwrap();
        let mut m2 = [0u8; 128];
        let n2 = resp.write_message(&[], &mut m2).unwrap();
        init.read_message(&m2[..n2], &mut tmp).unwrap();
        let mut i = init.into_transport_mode().unwrap();
        let mut out = [0u8; 128];
        let n3 = i.write_message(b"hello", &mut out).unwrap();
        let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
        println!("m1 {}\nm2 {}\nt1 {}", hex(&m1[..n1]), hex(&m2[..n2]), hex(&out[..n3]));
        assert_eq!(n1, 48);
        assert_eq!(n2, 48);
        assert_eq!(n3, 5 + 16);
    }
}
