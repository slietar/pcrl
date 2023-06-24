use crate::iterator::CharCounter;


#[derive(Clone, Copy, Debug)]
pub struct Empty {

}

impl CharCounter for Empty {
    fn new() -> Self {
        Self { }
    }

    fn consume(&mut self, _: char) {

    }
}


#[derive(Clone, Copy, Debug)]
pub struct Character {
    pub position: usize,
}

impl Character {
}

impl CharCounter for Character {
    fn new() -> Self {
        Self { position: 0 }
    }

    fn consume(&mut self, _: char) {
        self.position += 1;
    }
}


#[derive(Clone, Copy, Debug)]
pub struct CharacterLineColumn {
    pub column: usize,
    pub line: usize,
}

impl CharCounter for CharacterLineColumn {
    fn new() -> Self {
        Self {
            column: 0,
            line: 0,
        }
    }

    fn consume(&mut self, ch: char) {
        if ch == '\n' {
            self.column = 0;
            self.line += 1;
        } else {
            self.column += 1;
        }
    }
}


#[derive(Clone, Copy, Debug)]
pub struct UTF16 {
    pub position: usize,
}

impl CharCounter for UTF16 {
    fn new() -> Self {
        Self { position: 0 }
    }

    fn consume(&mut self, ch: char) {
        let mut buf = [0u16; 2];
        let result = ch.encode_utf16(&mut buf);
        self.position += result.len();
    }
}


#[derive(Clone, Copy, Debug)]
pub struct LspUtf16 {
    carriage_return: bool,
    pub column: usize,
    pub line: usize,
}

impl CharCounter for LspUtf16 {
    fn new() -> Self {
        Self {
            carriage_return: false,
            column: 0,
            line: 0,
        }
    }

    fn consume(&mut self, ch: char) {
        match (ch, self.carriage_return) {
            ('\n', false) => {
                self.column = 0;
                self.line += 1;
            },
            ('\n', true) => {
                self.carriage_return = false;
            },
            ('\r', _) => {
                self.carriage_return = true;
                self.column = 0;
                self.line += 1;
            },
            _ => {
                let mut buf = [0u16; 2];
                let result = ch.encode_utf16(&mut buf);
                self.column += result.len();
            }
        }
    }
}
