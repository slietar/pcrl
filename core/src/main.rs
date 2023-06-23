mod iterator;

use iterator::CharIteratorMarker;

use crate::iterator::CharIterator;


fn main() -> Result<(), ()> {
    // let mut parser = Parser::new("[5, 6,foo, null]");
    // let result = parser.accept_expr(['\n', '\n']);
    let mut parser = Parser::new("- 3 # Hello\n-4");

    parser.accept_line()?;
    parser.accept_line()?;

    // eprintln!("{:#?}", result);
    eprintln!("Errors: {:#?}", parser.errors);
    // eprintln!("{:#?}", &parser.contents[parser.offset..]);

    Ok(())
}


#[derive(Debug)]
pub enum TokenKind {
    Whitespace {
        length: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span(pub CharIteratorMarker, pub CharIteratorMarker);

impl Span {
    fn point(marker: &CharIteratorMarker) -> Self {
        Self(*marker, *marker)
    }
}


#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug)]
struct Indent {
    kind: IndentKind,
    size: usize,
}

impl Indent {
    fn calc(&self, other: &Indent) -> Option<usize> {
        if self.kind == other.kind && other.size % self.size == 0 {
            Some(other.size / self.size)
        } else {
            None
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum IndentKind {
    Spaces,
    Tabs,
}


#[derive(Debug)]
enum StackItem {
    List(Vec<Object>),
    Map {
        entries: Vec<(String, Object)>,
        key: Option<String>,
    },
    String(String),
}

#[derive(Debug)]
struct Parser<'a> {
    chars: CharIterator<'a>,
    errors: Vec<Error>,
    indent: Option<Indent>,
    stack: Vec<StackItem>,
}

impl<'a> Parser<'a> {
    fn new(contents: &'a str) -> Parser<'a> {
        Parser {
            chars: CharIterator::new(contents),
            errors: Vec::new(),
            indent: None,
            stack: Vec::new(),
        }
    }

    // fn save(&self) -> Point<'a> {
    //     Point {
    //         chars: self.chars.clone(),
    //         offset: self.offset
    //     }
    // }

    // fn restore(&mut self, point: Point<'a>) {
    //     self.chars = point.chars;
    //     self.offset = point.offset;
    // }
}


#[derive(Debug)]
pub enum Value {
    Integer(i64),
    List(Vec<Object>),
    Null,
    String(String),
}

#[derive(Debug)]
pub struct Object {
    pub span: Span,
    pub value: Value,
}

#[derive(Debug)]
pub enum Error {
    InvalidIndent(Span),
    InvalidIndentSize(Span),
    MissingListClose(Span),
}

impl Parser<'_> {
    // Only returns None if the end of the file is reached
    fn accept_expr(&mut self, break_chars: [char; 2]) -> Result<Option<Object>, ()> {
        self.pop_whitespace();

        let start_marker = self.chars.marker();
        let ch = match self.chars.peek() {
            Some(ch) => ch,
            None => return Ok(None),
        };

        let value = match ch {
            _ if ch == break_chars[0] => return Ok(None),
            _ if ch == break_chars[1] => return Ok(None),
            '\n' => return Ok(None),
            'n' if self.chars.pop_constant("null") => {
                Value::Null
            },
            '[' => {
                let mut items = Vec::new();

                self.chars.advance();
                self.pop_whitespace();

                if let Some(first_item) = self.accept_expr([',', ']'])? {
                    items.push(first_item);

                    loop {
                        self.pop_whitespace();

                        if !self.chars.pop_char(',') {
                            break;
                        }

                        if let Some(next_item) = self.accept_expr([',', ']'])? {
                            items.push(next_item);
                        } else {
                            break;
                        }
                    }
                }

                if !self.chars.pop_char(']') {
                    self.errors.push(Error::MissingListClose(Span::point(&self.chars.marker())));
                    return Err(());
                }

                Value::List(items)
            },
            _ if ch.is_digit(10) => {
                let string = self.chars.pop_while(|ch| ch.is_digit(10));
                Value::Integer(string.parse().unwrap())
            },
            _ => {
                let string = self.chars.pop_while(|ch| ch != break_chars[0] && ch != break_chars[1]);
                Value::String(string.to_string())
            },
        };

        Ok(Some(Object {
            span: Span(start_marker, self.chars.marker()),
            value,
        }))
    }

    fn accept_line(&mut self) -> Result<(), ()> {
        let start_marker = self.chars.marker();

        let indent = match self.chars.peek() {
            Some(' ') => {
                self.chars.pop_while(|ch| ch == ' ');
                Some(Indent {
                    kind: IndentKind::Spaces,
                    size: self.chars.char_offset - start_marker.char_offset,
                })
            },
            Some('\t') => {
                self.chars.pop_while(|ch| ch == '\t');
                Some(Indent {
                    kind: IndentKind::Tabs,
                    size: self.chars.char_offset - start_marker.char_offset,
                })
            },
            _ => None,
        };

        match self.chars.peek() {
            Some('\n' | '#') | None => (),
            _ => {
                // indent_level = max number of items in stack before processing
                let indent_level = match indent {
                    Some(indent) => {
                        match &self.indent {
                            Some(first_indent) => {
                                match first_indent.calc(&indent) {
                                    Some(level) => level,
                                    None => {
                                        self.errors.push(Error::InvalidIndentSize(Span(start_marker, self.chars.marker())));
                                        return Err(());
                                    },
                                }
                            },
                            None => {
                                self.indent = Some(indent);
                                1
                            },
                        }
                    },
                    None => 0,
                } + 1;

                // TODO: Handle extraneous stack items

                let current_level = self.stack.len();

                // if self.stack.is_empty() {
                //     if let Some(expr) = self.accept_expr(['\x00', '\x00'])? {
                //         eprintln!("{:?}", expr);
                //     }
                // } else {
                let extra = match indent_level - current_level {
                    1 => true,
                    0 => false,
                    _ => {
                        return Err(());
                    }
                };

                // eprintln!("! {}", &self.contents[self.offset..]);

                match self.chars.peek() {
                    // Some('A'..='Z' | 'a'..='z' | '_') => {
                    //     let key_start_offset = self.offset;
                    //     let point = self.save();

                    //     self.eat_while(|ch| ch.is_alphanumeric() || ch == '_');
                    //     let key = self.contents[key_start_offset..self.offset].to_string();

                    //     match self.peek() {
                    //         Some(':') => {
                    //             self.pop();
                    //             self.pop_whitespace();

                    //             let expr = self.accept_expr(['\x00', '\x00'])?;
                    //         },
                    //         _ => {
                    //             self.restore(point);
                    //         },
                    //     }
                    // },
                    Some('-') => {
                        self.chars.advance();
                        self.pop_whitespace();

                        // match self.peek() {
                        //     Some('A'..='Z' | 'a'..='z' | '_') => {
                        //         let key_start_offset = self.offset;
                        //         self.eat_while(|ch| ch.is_alphanumeric() || ch == '_');
                        //         let key = self.contents[key_start_offset..self.offset].to_string();
                        //     },
                        //     _ => (),
                        // }

                        if let Some(expr) = self.accept_expr(['\x00', '\x00'])? {
                            match (self.stack.last_mut(), extra) {
                                (Some(StackItem::Map { key: Some(_), .. }) | None, true) => {
                                    self.stack.push(StackItem::List(vec![expr]));
                                },
                                (Some(StackItem::List(list)), false) => {
                                    list.push(expr);
                                },
                                _ => {
                                    self.errors.push(Error::InvalidIndentSize(Span::point(&self.chars.marker())));
                                    return Ok(());
                                },
                            }
                        }
                    },
                    Some(_) => todo!(),
                    None => todo!(),
                }

                eprintln!("Stack: {:#?}", self.stack);
            },
        }

        // match self.stack.last_mut().unwrap() {
        //     StackItem::List(items) => {
        //         if let Some(expr) = self.accept_expr([',', '\n'])? {
        //             items.push(expr);
        //         }
        //     },
        //     _ => todo!()
        // }

        // match self.peek() { }

        self.pop_whitespace();

        let mut comment = None;

        match self.chars.peek() {
            Some('\n') => {
                self.chars.advance();
            },
            Some('#') => {
                let comment_start_marker = self.chars.marker();

                self.pop_whitespace();
                comment = Some(self.chars.pop_while(|ch| ch != '\n'));

                self.chars.pop();
            },
            None => {

            },
            _ => {
                return Err(());
            },
        }

        eprintln!("Comment: '{}'", comment.unwrap_or_default());

        Ok(())
    }

    // fn accept_value(&mut self) -> Result<Object, ()> {
    //     if let Some(expr) = self.accept_expr(['\x00', '\x00'])? {
    //         Ok(expr)
    //     } else {
    //         Err(())
    //     }
    // }

    fn accept_key(&mut self) -> Option<(String, Span)>{
        match self.chars.peek() {
            Some('A'..='Z' | 'a'..='z' | '_') => {
                let key_start_marker = self.chars.marker();

                let key = self.chars.pop_while(|ch| ch.is_alphanumeric() || ch == '_');
                // let key = self.contents[key_start_offset..self.offset].to_string();

                match self.chars.peek() {
                    Some(':') => {
                        let key_end_marker = self.chars.marker();
                        self.chars.advance();

                        Some((key.to_string(), Span(key_start_marker, key_end_marker)))
                    },
                    _ => {
                        self.chars.restore(&key_start_marker);
                        None
                    },
                }
            },
            _ => None,
        }
    }

    fn pop_whitespace(&mut self) {
        self.chars.pop_while(|ch| ch == ' ' || ch == '\t');
    }
}
