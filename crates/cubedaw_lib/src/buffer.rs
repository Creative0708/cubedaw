use std::io::{Write, Read, Result, ErrorKind};

pub struct WriteBuffer {
    bytes: Vec<u8>,
}

impl WriteBuffer {
    pub fn new() -> Self {
        WriteBuffer { bytes: Vec::new() }
    }
    pub fn get_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
}

impl Write for WriteBuffer {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.write_all(buf)?;
        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.bytes.extend(buf);
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct ReadBuffer {
    bytes: Box<[u8]>,
    index: usize,
}

impl ReadBuffer {
    pub fn new(bytes: Box<[u8]>) -> Self {
        ReadBuffer { bytes, index: 0 }
    }
}

impl From<WriteBuffer> for ReadBuffer {
    fn from(value: WriteBuffer) -> Self {
        Self::new(value.bytes.into_boxed_slice())
    }
}

impl Read for ReadBuffer {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let copyable_len = usize::min(buf.len(), self.bytes.len() - self.index);
        if copyable_len == 0 { return Ok(0); }
        buf[..copyable_len].copy_from_slice(&self.bytes[self.index..self.index + copyable_len]);
        self.index += copyable_len;
        Ok(copyable_len)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        if self.bytes.len() - self.index < buf.len() {
            return Err(ErrorKind::UnexpectedEof.into());
        }
        buf.copy_from_slice(&self.bytes[self.index..self.index + buf.len()]);
        self.index += buf.len();
        Ok(())
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let remaining = self.bytes.len() - self.index;
        buf.extend_from_slice(&self.bytes[self.index..]);
        Ok(remaining)
    }
}