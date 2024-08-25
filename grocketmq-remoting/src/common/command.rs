use std::{collections::HashMap, sync::atomic::{AtomicUsize, Ordering}};

use serde::{Deserialize, Serialize};

use crate::util;

static REQUEST_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Serialize, Deserialize)]
pub struct Header {
    code: u8,
    flag: u8,
    language: u8,
    opaque: usize,
    remark: String,
    ext_fields: HashMap<String, String>,
}

pub struct Command {
    header: Header,
    body: Option<Vec<u8>>,
}

impl Command {
    pub fn new(code: u8) -> Self {
        Self {
            header: Header {
                code,
                flag: 0,
                language: 12,
                opaque: REQUEST_ID.fetch_add(1, Ordering::Relaxed),
                remark: "".to_string(),
                ext_fields: HashMap::new(),
            },
            body: None,
        }
    }

    pub fn code(&self) -> u8 {
        self.header.code
    }

    pub fn opaque(&self) -> usize {
        self.header.opaque
    }

    pub fn add_property(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.header.ext_fields.insert(key.into(), value.into());
    }

    pub fn get_property(&self, key: &str) -> Option<&String> {
        self.header.ext_fields.get(key)
    }

    pub fn set_body(&mut self, body: Vec<u8>) {
        self.body = Some(body);
    }

    pub fn body(&self) -> Option<&[u8]> {
        self.body.as_ref().map(|v| v.as_ref())
    }

    pub fn encode(self) -> Vec<u8> {
        let mut length: u32 = 8;

        let header_data = serde_json::to_vec(&self.header).unwrap();
        length = length + header_data.len() as u32;

        let ref_body = self.body.as_ref();
        if let Some(body) = ref_body {
            length = length + body.len() as u32;
        }
        let mut result = Vec::with_capacity(4 + length as usize);
        result.extend(Self::u32_to_vec(length));
        result.extend(Self::u32_to_vec(header_data.len() as u32));
        result.extend(header_data);
        if let Some(body) = self.body {
            result.extend(body);
        }

        result
    }

    fn u32_to_vec(data: u32) -> Vec<u8> {
        let mut result = Vec::with_capacity(4);
        result.push((data >> 24) as u8);
        result.push((data >> 16) as u8);
        result.push((data >> 8) as u8);
        result.push(data as u8);
        result
    }

    pub fn decode(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let length = util::vec_to_u32(&data);
        let header_length = util::vec_to_u32(&data[4..8]);
        let header: Header = serde_json::from_slice(&data[8..8 + header_length as usize])?;
        Ok(Self {
            header,
            body: Some(data[8 + header_length as usize..length as usize].to_vec()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let mut command = Command::new(1);
        command.add_property("test-key", "value");
        command.set_body(vec![1, 2, 3]);

        let encoded = command.encode();
        let decoded = Command::decode(&encoded).unwrap();
        assert_eq!(1, decoded.code());
        assert_eq!(0, decoded.opaque());
        assert_eq!("value", decoded.get_property("test-key").unwrap());
        assert_eq!(vec![1, 2, 3], decoded.body().unwrap());
    }    
}
