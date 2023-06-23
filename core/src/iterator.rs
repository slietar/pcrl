use std::cell::Cell;


#[derive(Debug)]
pub struct CharIterator<'a> {
    bytes: &'a [u8],
    pub byte_offset: usize,
    pub char_offset: usize,
    last_byte_size: Cell<Option<usize>>,
}

impl<'a> CharIterator<'a> {
    pub fn new(string: &'a str) -> CharIterator<'a> {
        CharIterator {
            bytes: string.as_bytes(),
            byte_offset: 0,
            char_offset: 0,
            last_byte_size: Cell::new(None),
        }
    }

    fn next(&self) -> Option<(char, usize)> {
        if self.byte_offset < self.bytes.len() {
            let byte = self.bytes[self.byte_offset];

            if byte < 128 {
                Some((byte as char, 1))
            } else {
                todo!()
            }
        } else {
            None
        }

        // unsafe { std::validations::next_code_point(&mut self.iter).map(|ch| char::from_u32_unchecked(ch)) }
    }

    pub fn advance(&mut self) {
        self.byte_offset += self.last_byte_size.get().unwrap();
        self.char_offset += 1;

        self.last_byte_size.set(None);
    }

    pub fn bytes_from_marker(&self, marker: CharIteratorMarker) -> &'a str {
        unsafe { std::str::from_utf8_unchecked(&self.bytes[marker.byte_offset..self.byte_offset]) }
    }

    pub fn peek(&self) -> Option<char> {
        // self.next().and_then(|(ch, _)| Some(ch))

        match self.next() {
            Some((ch, size)) => {
                self.last_byte_size.set(Some(size));
                Some(ch)
            },
            None => None,
        }
    }

    pub fn pop(&mut self) -> Option<char> {
        match self.next() {
            Some((ch, size)) => {
                self.byte_offset += size;
                self.char_offset += 1;
                Some(ch)
            },
            None => None
        }
    }

    pub fn pop_while(&mut self, predicate: impl Fn(char) -> bool) -> &'a str {
        // let result = String::new();
        let start_byte_offset = self.byte_offset;

        loop {
            match self.next() {
                Some((ch, size)) => {
                    if predicate(ch) {
                        self.byte_offset += size;
                        self.char_offset += 1;
                    } else {
                        break;
                    }
                },
                None => {
                    break;
                }
            }
        }

        unsafe { std::str::from_utf8_unchecked(&self.bytes[start_byte_offset..self.byte_offset]) }
    }

    pub fn pop_constant(&mut self, constant: &str) -> bool {
        let marker = self.marker();

        for constant_ch in constant.chars() {
            match self.pop() {
                Some(ch) => {
                    if ch != constant_ch {
                        self.restore(&marker);
                        return false;
                    }
                },
                None => {
                    self.restore(&marker);
                    return false;
                },
            }
        }

        true
    }

    pub fn pop_char(&mut self, ch: char) -> bool {
        if self.peek() == Some(ch) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn marker(&self) -> CharIteratorMarker {
        CharIteratorMarker {
            byte_offset: self.byte_offset,
            char_offset: self.char_offset
        }
    }

    pub fn restore(&mut self, marker: &CharIteratorMarker) {
        self.byte_offset = marker.byte_offset;
        self.char_offset = marker.char_offset;
    }
}


#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CharIteratorMarker {
    pub byte_offset: usize,
    pub char_offset: usize,
}
