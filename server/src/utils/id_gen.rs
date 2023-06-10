use std::fmt::{Debug, Display};

use base64::{prelude::BASE64_URL_SAFE_NO_PAD, DecodeSliceError, Engine};
use rand::RngCore;

const TIMESTAMP_LENGTH: usize = u64::BITS as usize / 8;
const RANDOM_DATA_LENGTH: usize = 128;
const ID_LENGTH: usize = TIMESTAMP_LENGTH + RANDOM_DATA_LENGTH;

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id {
    timestamp: u64,
    data: [u8; RANDOM_DATA_LENGTH],
}

impl Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.as_string())
    }
}
impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.as_string())
    }
}

impl serde::Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.as_string())
    }
}

struct IdVisitor;

impl<'de> serde::de::Visitor<'de> for IdVisitor {
    type Value = Id;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("BASE64 id value")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        from_str(v).map_err(|err| {
            serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(v),
                &format!("{err}").as_str(),
            )
        })
    }
}

impl<'de> serde::Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(IdVisitor)
    }
}

impl Id {
    pub fn generate(rng: &mut impl RngCore) -> Self {
        let data = generate_data::<RANDOM_DATA_LENGTH>(rng);
        let timestamp = std::time::UNIX_EPOCH
            .elapsed()
            .expect("Time went backwards")
            .as_secs();
        Self { timestamp, data }
    }
    pub fn as_bytes(&self) -> [u8; ID_LENGTH] {
        let mut bytes = [0u8; ID_LENGTH];
        bytes[0..TIMESTAMP_LENGTH].copy_from_slice(&self.timestamp.to_be_bytes()[..]);
        bytes[TIMESTAMP_LENGTH..ID_LENGTH].copy_from_slice(&self.data);
        bytes
    }

    pub fn as_string(&self) -> String {
        let bytes = self.as_bytes();
        let mut string = String::with_capacity(ID_LENGTH);
        BASE64_URL_SAFE_NO_PAD.encode_string(bytes, &mut string);
        string
    }
}

fn generate_data<const N: usize>(rng: &mut impl RngCore) -> [u8; N] {
    let mut bytes = [0u8; N];
    rng.fill_bytes(&mut bytes);
    bytes
}

fn from_str(s: &str) -> Result<Id, DecodeSliceError> {
    let bytes = s.as_bytes();
    let len = base64::decoded_len_estimate(bytes.len());
    let mut buffer = Vec::with_capacity(len);
    BASE64_URL_SAFE_NO_PAD.decode_vec(bytes, &mut buffer)?;
    let slice = &buffer[..];
    let id = from_bytes_unchecked(slice);
    Ok(id)
}

#[derive(Debug)]
pub enum IdFromBytesError {
    InvalidLength(usize),
}

impl<'a> TryFrom<&'a [u8]> for Id {
    type Error = IdFromBytesError;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        from_bytes(value)
    }
}

impl From<Id> for [u8; ID_LENGTH] {
    fn from(value: Id) -> Self {
        value.as_bytes()
    }
}
impl<'a> From<&'a Id> for [u8; ID_LENGTH] {
    fn from(value: &'a Id) -> Self {
        value.as_bytes()
    }
}

fn from_bytes(bytes: &[u8]) -> Result<Id, IdFromBytesError> {
    if bytes.len() != ID_LENGTH {
        return Err(IdFromBytesError::InvalidLength(bytes.len()));
    }
    Ok(from_bytes_unchecked(bytes))
}

fn from_bytes_unchecked(bytes: &[u8]) -> Id {
    debug_assert_eq!(bytes.len(), ID_LENGTH);
    let mut time_bytes = [0u8; TIMESTAMP_LENGTH];
    time_bytes.copy_from_slice(&bytes[0..TIMESTAMP_LENGTH]);
    let timestamp = u64::from_be_bytes(time_bytes);
    Id {
        timestamp,
        data: bytes[TIMESTAMP_LENGTH..ID_LENGTH].try_into().unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::{from_bytes, from_bytes_unchecked, from_str, Id, ID_LENGTH, RANDOM_DATA_LENGTH};

    #[test]
    fn check_byte_conversion() {
        let id = Id {
            timestamp: 123400000004321,
            data: [8; RANDOM_DATA_LENGTH],
        };
        let bytes = id.as_bytes();

        let res = from_bytes_unchecked(&bytes);
        assert_eq!(id, res);
    }

    #[test]
    #[should_panic]
    fn test_length_mismatch() {
        let bytes = [0u8; ID_LENGTH + 2];
        let _ = from_bytes(&bytes).expect("Mismatch");
    }

    #[test]
    fn check_string_conversion() {
        let id = Id {
            timestamp: 123400000004321,
            data: [8; RANDOM_DATA_LENGTH],
        };
        let string = id.as_string();
        let res = from_str(&string).expect("Base64 encoding error");
        assert_eq!(id, res);
    }
}
