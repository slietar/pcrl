use crate::iterator::CharIndexer;


#[derive(Clone, Debug)]
pub struct Empty {

}

impl CharIndexer for Empty {
    type Index = ();

    fn new() -> Self {
        Self { }
    }

    fn consume(&mut self, _: char) {

    }

    fn export(&mut self, _: &str) -> Self::Index {
        ()
    }
}


#[derive(Clone, Debug)]
pub struct Character {
    pub position: usize,
}

impl Character {
}

impl CharIndexer for Character {
    type Index = usize;

    fn new() -> Self {
        Self { position: 0 }
    }

    fn consume(&mut self, ch: char) {

    }

    fn export(&mut self, string: &str) -> Self::Index {
        self.position += string.len();
        self.position
    }
}


#[derive(Clone, Debug)]
pub struct CharacterLineColumn {
    column: usize,
    line: usize,
}

impl CharIndexer for CharacterLineColumn {
    type Index = LineColumnIndex;

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

    fn export(&mut self, _: &str) -> Self::Index {
        LineColumnIndex {
            column: self.column,
            line: self.line,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LineColumnIndex {
    pub column: usize,
    pub line: usize,
}


#[derive(Clone, Debug)]
pub struct UTF16 {
    pub position: usize,
}

impl CharIndexer for UTF16 {
    type Index = usize;

    fn new() -> Self {
        Self { position: 0 }
    }

    fn consume(&mut self, _: char) {
        // self.position += ch.len_utf16();
    }

    fn export(&mut self, string: &str) -> Self::Index {
        self.position += string.encode_utf16().count();
        self.position
    }
}


// #[derive(Clone, Copy, Debug)]
// pub struct LspUtf16 {
//     carriage_return: bool,
//     pub column: usize,
//     pub line: usize,
// }

// impl CharIndexer for LspUtf16 {
//     fn new() -> Self {
//         Self {
//             carriage_return: false,
//             column: 0,
//             line: 0,
//         }
//     }

//     fn consume(&mut self, ch: char) {
//         match (ch, self.carriage_return) {
//             ('\n', false) => {
//                 self.column = 0;
//                 self.line += 1;
//             },
//             ('\n', true) => {
//                 self.carriage_return = false;
//             },
//             ('\r', _) => {
//                 self.carriage_return = true;
//                 self.column = 0;
//                 self.line += 1;
//             },
//             _ => {
//                 let mut buf = [0u16; 2];
//                 let result = ch.encode_utf16(&mut buf);
//                 self.column += result.len();
//             }
//         }
//     }
// }


// #[cfg(feature = "format")]
// #[derive(Clone, Debug)]
// pub struct Grapheme {
//     pub current_grapheme: String,
//     pub position: usize,
// }

// #[cfg(feature = "format")]
// impl CharIndexer<usize> for Grapheme {
//     fn new() -> Self {
//         Self {
//             current_grapheme: String::new(),
//             position: 0,
//         }
//     }

//     fn consume(&mut self, ch: char) {
//         use unicode_segmentation::UnicodeSegmentation;

//         self.current_grapheme.push(ch);

//         let mut graphemes = UnicodeSegmentation::graphemes(&self.current_grapheme[..], true);
//         let grapheme1 = graphemes.next();
//         let grapheme2 = graphemes.next();
//         let grapheme3 = graphemes.next();

//         match (grapheme1, grapheme2, grapheme3) {
//             (Some(_), Some(_), None) => {
//                 self.position += 1;
//                 self.current_grapheme.clear();
//             },
//             (Some(_), None, None) => (),
//             _ => panic!("Grapheme iterator returned an unexpected result"),
//         }
//     }

    // fn export(&self) -> Self::Index {
//         self.position
//     }
// }


// // #[cfg(feature = "format")]
// // impl std::clone::Clone for Grapheme {
// //     fn clone(&self) -> Self {
// //         Self {
// //             current_grapheme: self.current_grapheme.clone(),
// //             position: self.position,
// //         }
// //     }
// // }
