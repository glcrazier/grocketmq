use anyhow::anyhow;
use log::info;
use memmap2::MmapMut;
use prost::bytes::BufMut;
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    fs::{File, OpenOptions},
    path::Path,
};

#[derive(Debug, Clone)]
pub struct Message {
    payload: Vec<u8>,
    properties: HashMap<String, String>,
}

impl Message {
    const BYTES_SIZE: usize = 8;

    fn new(payload: Vec<u8>, properties: HashMap<String, String>) -> Self {
        Self {
            payload,
            properties,
        }
    }

    fn encode(&self) -> Vec<u8> {
        let mut frame_length = Message::BYTES_SIZE + self.payload.len();
        let mut header_length: usize = 0;
        let mut header: Option<Vec<u8>> = None;
        if !self.properties.is_empty() {
            if let Ok(data) = serde_json::to_vec(&self.properties) {
                header_length = data.len();
                header.replace(data);
            }
        }
        frame_length += header_length + Message::BYTES_SIZE;
        let mut result: Vec<u8> = Vec::with_capacity(frame_length as usize);
        result.put_slice(&frame_length.to_be_bytes());
        result.put_slice(&header_length.to_be_bytes());
        if let Some(data) = header {
            result.put_slice(&data);
        }
        if !self.payload.is_empty() {
            result.put_slice(&self.payload);
        }

        result
    }

    fn decode(content: &[u8]) -> Result<Message, anyhow::Error> {
        let length = content.len();
        let mut start_pos: usize = 0;
        let mut end_pos: usize = Message::BYTES_SIZE;
        if end_pos > length {
            return Err(anyhow!("invalid data format"));
        }
        let length_buf: [u8; Message::BYTES_SIZE] = content[start_pos..end_pos].try_into().unwrap();

        let frame_length = usize::from_be_bytes(length_buf);
        if frame_length != length {
            return Err(anyhow!("invalid frame length"));
        }
        start_pos = end_pos;
        end_pos += Message::BYTES_SIZE;

        // parse header
        let header_length_buf: [u8; Message::BYTES_SIZE] =
            content[start_pos..end_pos].try_into().unwrap();
        let header_length = usize::from_be_bytes(header_length_buf);
        start_pos = end_pos;
        end_pos += header_length;
        let properties: HashMap<String, String> = if header_length > 0 {
            serde_json::from_slice(&content[start_pos..end_pos]).unwrap()
        } else {
            HashMap::new()
        };

        start_pos = end_pos;
        end_pos = length;
        let payload = (&content[start_pos..end_pos]).to_vec();

        Ok(Message {
            payload,
            properties,
        })
    }

    fn payload(&self) -> &[u8] {
        &self.payload
    }

    fn properties(&self) -> &HashMap<String, String> {
        &self.properties
    }
}

pub struct LogFile {
    mmap_file: MmapMut,
    write_pos: usize,
}

impl LogFile {
    pub fn open<P: AsRef<Path> + Debug + Clone>(
        path: P,
        file_size: u64,
    ) -> Result<Self, anyhow::Error> {
        let clone_path = path.clone();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        file.set_len(file_size)?;
        let mmap_file = unsafe { MmapMut::map_mut(&file)? };
        let write_pos = LogFile::calcuate_write_pos(&mmap_file);
        info!("open file {:?} with pos {1}", clone_path, write_pos);
        Ok(LogFile {
            mmap_file,
            write_pos,
        })
    }

    pub fn append(&mut self, msg: &Message) -> Result<usize, anyhow::Error> {
        let raw_bytes = msg.encode();
        let len = raw_bytes.len();
        if self.mmap_file.len() < self.write_pos + len {
            return Err(anyhow!("reach file end"));
        }

        unsafe {
            let write_ptr = self.mmap_file.as_mut_ptr().add(self.write_pos);
            write_ptr.copy_from_nonoverlapping(raw_bytes.as_ptr(), len);
        }
        self.write_pos += len;
        Ok(self.write_pos)
    }

    pub fn read(&self, offset: usize) -> Result<Message, anyhow::Error> {
        unsafe {
            let read_ptr = self.mmap_file.as_ptr().add(offset);
            let mut frame_length_buf: Vec<u8> = Vec::with_capacity(Message::BYTES_SIZE);
            read_ptr.copy_to(frame_length_buf.as_mut_ptr(), Message::BYTES_SIZE);
            let frame_length = usize::from_be_bytes(frame_length_buf.try_into().unwrap());
            let mut content: Vec<u8> = Vec::with_capacity(frame_length);
            read_ptr.copy_to(content.as_mut_ptr(), frame_length);
            Message::decode(&content)
        }
    }

    pub fn flush(&self) -> Result<(), anyhow::Error> {
        self.mmap_file.flush()?;
        Ok(())
    }

    fn calcuate_write_pos(mem: &[u8]) -> usize {
        let mut pos = 0;
        loop {
            let frame_length_buf: [u8; Message::BYTES_SIZE] =
                mem[pos..pos + Message::BYTES_SIZE].try_into().unwrap();
            let frame_length = usize::from_be_bytes(frame_length_buf);
            if frame_length == 0 {
                break;
            } else {
                pos += frame_length;
            }
        }
        pos
    }
}

pub struct LogFileQueue {
    file_queue: Vec<LogFile>,
    file_size: usize,
}

impl LogFileQueue {
    pub fn create(log_dir: &impl AsRef<Path>, file_size: usize) -> Result<Self, anyhow::Error> {
        Err(anyhow!("not implemented"))
    }

    pub fn append_message(&self, msg: &Message) -> Result<(), anyhow::Error> {
        Ok(())
    }

    fn new_file() {}
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test_log::test]
    fn test_message_decode_encode() {
        let mut properties = HashMap::new();
        properties.insert("abc".to_string(), "def".to_string());
        let message = Message::new(vec![1, 2, 3, 4], properties);
        let result = message.encode();
        assert!(!result.is_empty());
        let result = Message::decode(&result);
        assert!(result.is_ok());
        let msg = result.unwrap();
        assert_eq!(vec![1, 2, 3, 4], msg.payload());
        let properties = msg.properties();
        assert_eq!(properties.get("abc").unwrap(), "def");
    }

    #[test_log::test]
    fn test_log_append() {
        let mut log_file = LogFile::open("../target/test_log", 100 * 1024 * 1024).unwrap();
        let msg1 = Message::new("Hell world".as_bytes().to_vec(), HashMap::new());
        assert!(log_file.append(&msg1).is_ok());
        let mut properties = HashMap::new();
        properties.insert("abc".to_string(), "cde".to_string());
        let msg2 = Message::new("Hello world".as_bytes().to_vec(), properties);
        assert!(log_file.append(&msg2).is_ok());

        assert!(log_file.flush().is_ok());
        drop(log_file);
    }
}
