use std::cell::Cell;


pub trait CharCounter: Clone + Copy + std::fmt::Debug {
    fn new() -> Self;
    fn consume(&mut self, ch: char);
}


#[derive(Debug)]
pub struct CharIterator<'a, T: CharCounter> {
    pub bytes: &'a [u8],
    pub byte_offset: usize,
    counter: T,
    last_byte_size: Cell<Option<usize>>,
}

impl<'a, T: CharCounter> CharIterator<'a, T> {
    pub fn new(string: &'a str) -> CharIterator<'a, T> {
        CharIterator {
            bytes: string.as_bytes(),
            byte_offset: 0,
            counter: T::new(),
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
        self.pop();

        // self.byte_offset += self.last_byte_size.get().unwrap();
        // self.char_offset += 1;

        self.last_byte_size.set(None);
    }

    pub fn bytes_from_marker(&self, marker: CharIteratorMarker<T>) -> &'a str {
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
                self.counter.consume(ch);

                Some(ch)
            },
            None => None
        }
    }

    pub fn pop_while(&mut self, predicate: impl Fn(char) -> bool) -> &'a str {
        let start_byte_offset = self.byte_offset;

        loop {
            match self.next() {
                Some((ch, size)) => {
                    if predicate(ch) {
                        self.byte_offset += size;
                        self.counter.consume(ch);
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
        let start_byte_offset = self.byte_offset;
        let mut end_marker = self.marker();

        loop {
            match self.next() {
                Some((ch, size)) => {
                    if !predicate_while(ch) {
                        break;
                    }

                    self.byte_offset += size;
                    self.counter.consume(ch);

                    if !predicate_until(ch) {
                        end_marker = self.marker();
                    }
                },
                None => {
                    break;
                },
            }
        }

        self.restore(&end_marker);

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

    pub fn marker(&self) -> CharIteratorMarker<T> {
        CharIteratorMarker {
            byte_offset: self.byte_offset,
            counter: self.counter
        }
    }

    pub fn restore(&mut self, marker: &CharIteratorMarker<T>) {
        self.byte_offset = marker.byte_offset;
        self.counter = marker.counter;
    }
}


#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CharIteratorMarker<T: CharCounter> {
    pub byte_offset: usize,
    pub counter: T,
}

// impl<T: CharCounter> CharIteratorMarker<T> {
//     fn convert<S: CharCounter>(&self, contents: &str) -> CharIteratorMarker<S> {
//         let mut marker = CharIteratorMarker {
//             byte_offset: 0,
//             counter: S::new(),
//         };

//         while marker.byte_offset < self.byte_offset {
//             let ch = contents[marker.byte_offset].chars().next().unwrap();
//             marker.counter.consume(ch);
//         }

//         marker
//     }
// }

// impl<Src: CharCounter, Dest: CharCounter> std::convert::From<CharIteratorMarker<Src>> for CharIteratorMarker<Dest> {
//     fn from(value: CharIteratorMarker<Src>) -> Self {
//         Self { byte_offset: value.byte_offset, counter: Dest::new() }
//     }
// }
