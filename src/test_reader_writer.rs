use std::vec::Vec;

use embedded_io_async as eia;

use crate::{readline, Buffers};

pub struct TestReaderWriter<'a> {
    pub data_to_read: &'a [u8],
    pub data_to_write: Vec<u8>,
    pub pos: usize,
}
impl<'a> TestReaderWriter<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data_to_read: data,
            data_to_write: Vec::new(),
            pos: 0,
        }
    }
    pub fn totally_consumed(&self) -> bool {
        self.pos == self.data_to_read.len()
    }
}
impl<'a> eia::ErrorType for TestReaderWriter<'a> {
    type Error = eia::ErrorKind;
}
impl<'a> eia::Read for TestReaderWriter<'a> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        if self.pos >= self.data_to_read.len() {
            return Ok(0);
        }
        let len = buf.len().min(self.data_to_read.len() - self.pos);
        buf[..len].copy_from_slice(&self.data_to_read[self.pos..self.pos + len]);
        self.pos += len;
        Ok(len)
    }
}
impl<'a> eia::Write for TestReaderWriter<'a> {
    async fn write(&mut self, bytes: &[u8]) -> Result<usize, Self::Error> {
        self.data_to_write.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
