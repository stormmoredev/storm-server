use std::io::Read;

pub struct StringReader {
    content: String,
    idx: usize,
}

impl StringReader {
    pub fn new(content: String) -> StringReader {
        StringReader { content, idx: 0 }
    }
}

impl Read for StringReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = buf.len();
        let start = self.idx;
        let end = start + len - 1;

        let mut read = 0;
        let mut buff_idx = 0;
        for i in start..end {
            if i > self.content.len() - 1 {
                break;
            }
            buf[buff_idx] = self.content.as_bytes()[i];
            read += 1;
            self.idx += 1;
            buff_idx += 1;
        }

        Ok(read)
    }
}
