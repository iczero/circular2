use std::io::{Read, Write};

// change: do NOT derive PartialEq (it does not make sense)
#[derive(Debug, Clone)]
pub struct Buffer {
    /// backing buffer
    buf: Vec<u8>,
    /// pointer to start of data
    start: usize,
    /// pointer to end of data
    end: usize,
    // change: no capacity! the vec has one already
}

// TODO: generic?
impl Buffer {
    /// create buffer with given capacity
    pub fn with_capacity(capacity: usize) -> Buffer {
        // change: use vec macro to construct zeroed vec
        let buf = vec![0; capacity];
        Buffer {
            buf,
            start: 0,
            end: 0,
        }
    }

    /// create buffer by copying slice
    pub fn from_slice(data: &[u8]) -> Buffer {
        Buffer {
            buf: data.to_owned(),
            start: 0,
            end: data.len(),
        }
    }

    /// resize buffer to a larger size
    pub fn grow(&mut self, new_size: usize) -> bool {
        if new_size <= self.buf.capacity() {
            false
        } else {
            self.buf.resize(new_size, 0);
            true
        }
    }

    /// return data currently available to consume
    pub fn available_data(&self) -> usize {
        self.end - self.start
    }

    /// return available space for new data
    pub fn available_space(&self) -> usize {
        self.buf.capacity() - self.end
    }

    /// return capacity of backing buffer
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    /// return if buffer is empty (no data to read)
    pub fn empty(&self) -> bool {
        self.end == self.start
    }

    /// mark data as having been consumed
    pub fn consume(&mut self, count: usize) -> usize {
        if count > self.available_data() {
            // change: panics if you try to consume() too much data
            panic!("attempted to consume more data than available");
        }
        self.start += count;
        // change: does not shift()
        count
    }

    /// mark data as having been consumed, compat method
    pub fn consume_noshift(&mut self, count: usize) -> usize {
        // forward to other
        self.consume(count)
    }

    /// inform buffer that new data has been written to available space
    pub fn fill(&mut self, count: usize) -> usize {
        if count > self.available_space() {
            // change: panics when you try to fill() too much data
            panic!("attempted to write more data than available space");
        }
        self.end += count;
        count
        // change: does not shift() (not sure why it was there originally)
    }

    /// return pointer to start of data
    pub fn position(&self) -> usize {
        self.start
    }

    /// reset start/end pointers to beginning of backing buffer, will not modify data
    pub fn reset(&mut self) {
        self.start = 0;
        self.end = 0;
    }

    /// returns slice with data available to read
    pub fn data(&self) -> &[u8] {
        &self.buf[self.start..self.end]
    }

    /// returns slice with space available to write
    pub fn space(&mut self) -> &mut [u8] {
        &mut self.buf[self.end..]
    }

    /// move remaining data to beginning of buffer and reset position() to 0
    pub fn shift(&mut self) {
        if self.start == 0 {
            return;
        }

        let len = self.end - self.start;
        // change: no unsafe!
        self.buf.copy_within(self.start..self.end, 0);
        self.start = 0;
        self.end = len;
    }

    // the following methods were originally #[doc(hidden)] and they probably
    // need more tests

    /// delete `len` elements `start` elements from the read position
    pub fn delete_slice(&mut self, start: usize, len: usize) -> Option<usize> {
        if start + len >= self.available_data() {
            return None;
        }

        // change: not unsafe
        // copy elements after deleted range to start of delete position
        let copy_from = self.start + start + len;
        let copy_to = self.start + start;
        self.buf.copy_within(copy_from..self.end, copy_to);
        self.end -= len;
        Some(self.available_data())
    }

    /// insert a slice at `start` elements from the read position
    pub fn insert_slice(&mut self, data: &[u8], start: usize) -> Option<usize> {
        if start >= self.available_data() {
            return None;
        }
        if self.available_space() + data.len() > self.buf.capacity() {
            // could not possibly fit new data
            return None;
        } else if self.start + self.available_data() + data.len() > self.buf.capacity() {
            // cannot fit new data as is, but can if we shift()
            // note: could reduce some copying by not shifting everything in all cases
            self.shift();
            // just in case
            debug_assert!(self.start + self.available_data() + data.len() > self.buf.capacity());
        }

        // copy elements after start position to end of new slice
        let remain_start = self.start + start;
        let remain_end = self.end;
        let remain_copy_to = remain_start + data.len();
        self.buf
            .copy_within(remain_start..remain_end, remain_copy_to);

        // copy new elements into buffer
        let insert_at = self.start + start;
        self.buf[insert_at..insert_at + data.len()].copy_from_slice(data);
        self.end += data.len();
        Some(self.available_data())
    }

    /// replace range `start..start + len` with `data`
    pub fn replace_slice(&mut self, mut data: &[u8], start: usize, len: usize) -> Option<usize> {
        match len.cmp(&data.len()) {
            std::cmp::Ordering::Greater => {
                if self.start + start + len > self.end {
                    return None;
                }
                // delete excess elements
                let delete_from = start + data.len();
                let delete_len = len - data.len();
                self.delete_slice(delete_from, delete_len)
                    .expect("logic error in replace_slice (greater)");
            }
            std::cmp::Ordering::Less => {
                if self.start + start + data.len() > self.end {
                    return None;
                }
                // insert extra elements before overwriting
                let (fits, extra) = data.split_at(len);
                data = fits;
                let insert_at = start + len;
                self.insert_slice(extra, insert_at)
                    .expect("logic error in replace_slice (less)");
            }
            std::cmp::Ordering::Equal => {
                if self.start + start + len > self.end {
                    return None;
                }
            }
        }

        // copy remaining region into buffer
        let copy_start = self.start + start;
        let copy_end = copy_start + data.len();
        self.buf[copy_start..copy_end].copy_from_slice(data);
        Some(self.available_data())
    }
}

impl Read for Buffer {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read_from = self.data();
        let read_len = read_from.len().min(buf.len());
        buf[..read_len].copy_from_slice(&read_from[..read_len]);
        self.start += read_len;
        Ok(read_len)
    }
}

impl Write for Buffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let write_to = self.space();
        let write_len = write_to.len().min(buf.len());
        write_to[..write_len].copy_from_slice(&buf[..write_len]);
        self.end += write_len;
        // change: write() will not shift(), it must be done manually (no surprises)
        Ok(write_len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// note: these tests are copied exactly from the original `circular` crate
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn fill_and_consume() {
        let mut b = Buffer::with_capacity(10);
        assert_eq!(b.available_data(), 0);
        assert_eq!(b.available_space(), 10);
        let res = b.write(&b"abcd"[..]);
        assert_eq!(res.ok(), Some(4));
        assert_eq!(b.available_data(), 4);
        assert_eq!(b.available_space(), 6);

        assert_eq!(b.data(), &b"abcd"[..]);

        b.consume(2);
        assert_eq!(b.available_data(), 2);
        assert_eq!(b.available_space(), 6);
        assert_eq!(b.data(), &b"cd"[..]);

        b.shift();
        assert_eq!(b.available_data(), 2);
        assert_eq!(b.available_space(), 8);
        assert_eq!(b.data(), &b"cd"[..]);

        assert_eq!(b.write(&b"efghijklmnop"[..]).ok(), Some(8));
        assert_eq!(b.available_data(), 10);
        assert_eq!(b.available_space(), 0);
        assert_eq!(b.data(), &b"cdefghijkl"[..]);
        b.shift();
        assert_eq!(b.available_data(), 10);
        assert_eq!(b.available_space(), 0);
        assert_eq!(b.data(), &b"cdefghijkl"[..]);
    }

    #[test]
    fn delete() {
        let mut b = Buffer::with_capacity(10);
        let _ = b.write(&b"abcdefgh"[..]);
        assert_eq!(b.available_data(), 8);
        assert_eq!(b.available_space(), 2);

        assert_eq!(b.delete_slice(2, 3), Some(5));
        assert_eq!(b.available_data(), 5);
        assert_eq!(b.available_space(), 5);
        assert_eq!(b.data(), &b"abfgh"[..]);

        assert_eq!(b.delete_slice(5, 2), None);
        assert_eq!(b.delete_slice(4, 2), None);
    }

    #[test]
    fn replace() {
        let mut b = Buffer::with_capacity(10);
        let _ = b.write(&b"abcdefgh"[..]);
        assert_eq!(b.available_data(), 8);
        assert_eq!(b.available_space(), 2);

        assert_eq!(b.replace_slice(&b"ABC"[..], 2, 3), Some(8));
        assert_eq!(b.available_data(), 8);
        assert_eq!(b.available_space(), 2);
        assert_eq!(b.data(), &b"abABCfgh"[..]);

        assert_eq!(b.replace_slice(&b"XYZ"[..], 8, 3), None);
        assert_eq!(b.replace_slice(&b"XYZ"[..], 6, 3), None);

        assert_eq!(b.replace_slice(&b"XYZ"[..], 2, 4), Some(7));
        assert_eq!(b.available_data(), 7);
        assert_eq!(b.available_space(), 3);
        assert_eq!(b.data(), &b"abXYZgh"[..]);

        assert_eq!(b.replace_slice(&b"123"[..], 2, 2), Some(8));
        assert_eq!(b.available_data(), 8);
        assert_eq!(b.available_space(), 2);
        assert_eq!(b.data(), &b"ab123Zgh"[..]);
    }

    #[test]
    fn set_position() {
        let mut output = [0; 5];
        let mut b = Buffer::with_capacity(10);
        let _ = b.write(&b"abcdefgh"[..]);
        let _ = b.read(&mut output);
        assert_eq!(b.available_data(), 3);
        println!("{:?}", b.position());
    }

    #[test]
    fn consume_without_shift() {
        let mut b = Buffer::with_capacity(10);
        let _ = b.write(&b"abcdefgh"[..]);
        b.consume_noshift(6);
        assert_eq!(b.position(), 6);
    }
}
