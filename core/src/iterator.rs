use std::cell::Cell;


pub trait CharIndexer: Clone + std::fmt::Debug {
    type Index: CharIndex;

    fn new() -> Self;
    fn consume(&mut self, ch: char);
    fn export(&mut self, string: &str) -> Self::Index;
}

pub trait CharIndex: Clone + Copy + std::fmt::Debug { }
impl<T: Clone + Copy + std::fmt::Debug> CharIndex for T {}


#[derive(Debug)]
pub struct CharIterator<'a, Indexer: CharIndexer> {
    pub bytes: &'a [u8],
    pub byte_offset: usize,

    indexer: Indexer,
    indexer_byte_offset: usize,

    last_char: Cell<Option<(char, usize)>>,
}

impl<'a, Indexer: CharIndexer> CharIterator<'a, Indexer> {
    pub fn new(string: &'a str) -> Self {
        Self {
            bytes: string.as_bytes(),
            byte_offset: 0,
            indexer: Indexer::new(),
            indexer_byte_offset: 0,
            last_char: Cell::new(None),
        }
    }

    fn next(&self) -> Option<(char, usize)> {
        const fn utf8_first_byte(byte: u8, width: u32) -> u32 {
            (byte & (0x7F >> width)) as u32
        }

        const CONT_MASK: u8 = 0b0011_1111;

        const fn utf8_acc_cont_byte(ch: u32, byte: u8) -> u32 {
            (ch << 6) | (byte & CONT_MASK) as u32
        }

        if self.byte_offset < self.bytes.len() {
            // Taken from std::str::validations::next_code_point
            // https://github.com/rust-lang/rust/blob/master/library/core/src/str/validations.rs#L36

            // Assumes that the input is valid UTF-8

            let x = self.bytes[self.byte_offset];
            if x < 128 {
                return Some((x as char, 1));
            }

            let init = utf8_first_byte(x, 2);
            let y = self.bytes[self.byte_offset + 1];
            let mut code = utf8_acc_cont_byte(init, y);
            let mut size = 2;

            if x >= 0xE0 {
                let z = self.bytes[self.byte_offset + 2];
                let y_z = utf8_acc_cont_byte((y & CONT_MASK) as u32, z);
                code = init << 12 | y_z;
                size += 1;

                if x >= 0xF0 {
                    let w = self.bytes[self.byte_offset + 3];
                    code = (init & 7) << 18 | utf8_acc_cont_byte(y_z, w);
                    size += 1;
                }
            }

            Some((unsafe { char::from_u32_unchecked(code) }, size))
        } else {
            None
        }
    }

    pub fn advance(&mut self) {
        let (_, size) = self.last_char.get().unwrap();
        self.byte_offset += size;
        self.last_char.set(None);
    }

    pub fn marker(&mut self) -> Marker<Indexer::Index> {
        let string = unsafe { std::str::from_utf8_unchecked(&self.bytes[self.indexer_byte_offset..self.byte_offset]) };

        for ch in string.chars() {
            self.indexer.consume(ch);
        }

        self.indexer_byte_offset = self.byte_offset;

        Marker {
            byte_offset: self.byte_offset,
            index: self.indexer.export(string),
        }
    }

    pub fn peek(&self) -> Option<char> {
        if let Some((ch, _)) = self.last_char.get() {
            Some(ch)
        } else {
            match self.next() {
                Some((ch, size)) => {
                    self.last_char.set(Some((ch, size)));
                    Some(ch)
                },
                None => None,
            }
        }
    }

    pub fn pop(&mut self) -> Option<char> {
        match self.last_char.get().or_else(|| self.next()) {
            Some((ch, size)) => {
                self.byte_offset += size;
                self.last_char.set(None);
                Some(ch)
            },
            None => None
        }
    }

    pub fn pop_while(&mut self, predicate: impl Fn(char) -> bool) -> &'a str {
        self.last_char.set(None);

        let start_byte_offset = self.byte_offset;

        loop {
            match self.next() {
                Some((ch, size)) => {
                    if predicate(ch) {
                        self.byte_offset += size;
                    } else {
                        break;
                    }
                },
                None => {
                    break;
                },
            }
        }

        unsafe { std::str::from_utf8_unchecked(&self.bytes[start_byte_offset..self.byte_offset]) }
    }

    pub fn pop_until(&mut self, predicate_while: impl Fn(char) -> bool, predicate_until: impl Fn(char) -> bool) -> &'a str {
        self.last_char.set(None);

        let start_byte_offset = self.byte_offset;
        let mut end_byte_offset = self.byte_offset;

        loop {
            match self.next() {
                Some((ch, size)) => {
                    if !predicate_while(ch) {
                        break;
                    }

                    self.byte_offset += size;

                    if !predicate_until(ch) {
                        end_byte_offset = self.byte_offset;
                    }
                },
                None => {
                    break;
                },
            }
        }

        self.byte_offset = end_byte_offset;

        unsafe { std::str::from_utf8_unchecked(&self.bytes[start_byte_offset..self.byte_offset]) }
    }

    pub fn pop_constant(&mut self, constant: &str) -> bool {
        let start_offset = self.byte_offset;

        for constant_ch in constant.chars() {
            match self.pop() {
                Some(ch) => {
                    if ch != constant_ch {
                        self.byte_offset = start_offset;
                        return false;
                    }
                },
                None => {
                    self.byte_offset = start_offset;
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
}


#[derive(Clone, Copy, Debug)]
pub struct Marker<Index: CharIndex> {
    pub byte_offset: usize,
    pub index: Index,
}

impl<Index: CharIndex> PartialEq for Marker<Index> {
    fn eq(&self, other: &Self) -> bool {
        self.byte_offset == other.byte_offset
    }
}
impl<Index: CharIndex> Eq for Marker<Index> {

}
