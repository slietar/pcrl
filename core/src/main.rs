use std::str::Chars;


fn main() -> Result<(), ()> {
    // let mut parser = Parser::new("[5, 6,foo, null]");
    // let result = parser.accept_expr(['\n', '\n']);
    let mut parser = Parser::new("- 3 # Hello\n -4");

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
pub struct Span(pub usize, pub usize);

impl Span {
    pub fn len(&self) -> usize {
        self.1 - self.0
    }

    fn point(point: usize) -> Self {
        Self(point, point)
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
    chars: Chars<'a>,
    contents: &'a str,
    errors: Vec<Error>,
    indent: Option<Indent>,
    offset: usize,
    stack: Vec<StackItem>,
}

impl<'a> Parser<'a> {
    pub fn new(contents: &'a str) -> Parser<'a> {
        Parser {
            chars: contents.chars(),
            contents,
            errors: Vec::new(),
            indent: None,
            offset: 0,
            stack: Vec::new(),
        }
    }

    pub fn pop(&mut self) -> Option<char> {
        match self.chars.next() {
            Some(ch) => {
                self.offset += 1;
                Some(ch)
            },
            None => None
        }
    }

    pub fn advance(&mut self, distance: usize) {
        self.offset += distance;
        self.chars.nth(distance - 1);
    }

    pub fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        loop {
            match self.peek() {
                Some(ch) if !predicate(ch) => break,
                None => break,
                _ => ()
            }

            self.pop();
        }
    }

    // pub fn is_eof(&self) -> bool {
    //     self.chars.as_str().is_empty()
    // }

    pub fn peek(&self) -> Option<char> {
        self.chars.clone().next()
    }
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
        self.eat_whitespace();

        let start_offset = self.offset;
        let ch = match self.peek() {
            Some(ch) => ch,
            None => return Ok(None),
        };

        let value = match ch {
            _ if ch == break_chars[0] => return Ok(None),
            _ if ch == break_chars[1] => return Ok(None),
            '\n' => return Ok(None),
            'n' if &self.contents[self.offset..(self.offset + 4)] == "null" => {
                self.advance(4);

                Value::Null
            },
            '[' => {
                let mut items = Vec::new();

                self.pop();
                self.eat_whitespace();

                if let Some(first_item) = self.accept_expr([',', ']'])? {
                    items.push(first_item);

                    loop {
                        self.eat_whitespace();

                        if let Some(',') = self.peek() {
                            self.pop();
                        } else {
                            break;
                        }

                        if let Some(next_item) = self.accept_expr([',', ']'])? {
                            items.push(next_item);
                        } else {
                            break;
                        }
                    }
                }

                if self.peek() == Some(']') {
                    self.pop();
                } else {
                    self.errors.push(Error::MissingListClose(Span::point(self.offset - 1)));
                    return Err(());
                }

                Value::List(items)
            },
            _ if ch.is_digit(10) => {
                self.eat_while(|ch| ch.is_digit(10));
                Value::Integer(self.contents[start_offset..self.offset].parse().unwrap())
            },
            _ => {
                self.eat_while(|ch| ch != break_chars[0] && ch != break_chars[1]);
                Value::String(self.contents[start_offset..self.offset].to_string())
            },
        };

        let end_offset = self.offset;

        Ok(Some(Object {
            span: Span(start_offset, end_offset),
            value,
        }))
    }

    fn accept_line(&mut self) -> Result<(), ()> {
        let start_offset = self.offset;

        let indent = match self.peek() {
            Some(' ') => {
                self.eat_while(|ch| ch == ' ');
                Some(Indent {
                    kind: IndentKind::Spaces,
                    size: self.offset - start_offset,
                })
            },
            Some('\t') => {
                self.eat_while(|ch| ch == '\t');
                Some(Indent {
                    kind: IndentKind::Tabs,
                    size: self.offset - start_offset,
                })
            },
            _ => None,
        };

        match self.peek() {
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
                                        self.errors.push(Error::InvalidIndentSize(Span(start_offset, self.offset)));
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

                match self.peek() {
                    Some('A'..='Z' | 'a'..='z' | '_') => {
                        let key_start_offset = self.offset;
                        self.eat_while(|ch| ch.is_alphanumeric() || ch == '_');
                        let key = self.contents[key_start_offset..self.offset].to_string();

                        match self.peek() {
                            Some(':') => {
                                self.pop();
                                self.eat_whitespace();

                                let expr = self.accept_expr(['\x00', '\x00'])?;
                            },
                            _ => (),
                        }
                    },
                    Some('-') => {
                        self.pop();
                        self.eat_whitespace();

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
                                    self.errors.push(Error::InvalidIndentSize(Span(self.offset, self.offset)));
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

        self.eat_whitespace();

        let mut comment = None;

        match self.peek() {
            Some('\n') => {
                self.pop();
            },
            Some('#') => {
                let comment_start_offset = self.offset;

                self.eat_whitespace();
                self.eat_while(|ch| ch != '\n');
                comment = Some(self.contents[comment_start_offset..self.offset].to_string());

                self.pop();
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

    fn eat_whitespace(&mut self) {
        self.eat_while(|ch| ch == ' ' || ch == '\t');
    }
}
