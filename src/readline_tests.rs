use std::vec::Vec;

use embedded_io_async as eia;

use crate::{readline, Buffers};

struct TestUart {
    data_to_read: &'static [u8],
    data_to_write: Vec<u8>,
    pos: usize,
}
impl TestUart {
    fn new(data: &'static [u8]) -> Self {
        Self {
            data_to_read: data,
            data_to_write: Vec::new(),
            pos: 0,
        }
    }
}
impl eia::ErrorType for TestUart {
    type Error = eia::ErrorKind;
}
impl eia::Read for TestUart {
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
impl eia::Write for TestUart {
    async fn write(&mut self, bytes: &[u8]) -> Result<usize, Self::Error> {
        self.data_to_write.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[tokio::test]
async fn test_read_line() {
    let mut uart = TestUart::new(b"hello\nworld\n");
    let mut buffers: Buffers<10, 1> = Buffers::default();
    let result = readline(&mut uart, &mut buffers).await;
    assert_eq!(result, Ok(&b"hello"[..]));
    assert_eq!(uart.data_to_write, b"hello"[..]);

    let result = readline(&mut uart, &mut buffers).await;
    assert_eq!(result, Ok(&b"world"[..]));
    assert_eq!(uart.data_to_write, b"helloworld"[..]);
}
