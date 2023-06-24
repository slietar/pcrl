mod counters;
mod iterator;

use iterator::{CharIterator, CharIteratorMarker, CharCounter};


fn main() -> Result<(), ()> {
    // let mut parser: Parser<'_, counters::Character> = Parser::new("x\ny");
    // let result = parser.accept_expr(['\n', '\n']);

    // eprintln!("{:#?}", result);
    // return Ok(());

    let mut parser: Parser<'_, counters::Character> = Parser::new("
-
    - z
    -
        - w # foo
    - { a: b }
    - [5, 6]
");

    let result = parser.accept();

    eprintln!("Result: {:#?}", result);
    eprintln!("Errors: {:#?}", parser.errors);
    // eprintln!("{:#?}", &parser.contents[parser.offset..]);

    if let Ok(result) = result {
        let mut output = Vec::new();
        result.value.json(&mut output).unwrap();

        eprintln!("JSON: {}", std::str::from_utf8(&output).unwrap());
    }

    Ok(())
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span<T: CharCounter>(pub CharIteratorMarker<T>, pub CharIteratorMarker<T>);

impl<T: CharCounter> Span<T> {
    fn point(marker: &CharIteratorMarker<T>) -> Self {
        Self(*marker, *marker)
    }
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
enum StackItem<T: CharCounter> {
    List {
        items: Vec<Object<T>>,
        start_marker: Option<CharIteratorMarker<T>>, // None -> the list was pre-allocated
    },
    Map {
        entries: Vec<(String, Object<T>)>,
        key: Option<String>,
    },
    String(String),
}

#[derive(Debug)]
struct Parser<'a, T: CharCounter> {
    chars: CharIterator<'a, T>,
    errors: Vec<Error<T>>,
    indent: Option<Indent>,
    stack: Vec<StackItem<T>>,
}

impl<'a, T: CharCounter> Parser<'a, T> {
    fn new(contents: &'a str) -> Self {
        Self {
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
pub enum Value<T: CharCounter> {
    Integer(i64),
    List(Vec<Object<T>>),
    Map(Vec<(WithSpan<String, T>, Object<T>)>),
    Null,
    String(String),
}

impl<T: CharCounter> Value<T> {
    fn json(&self, output: &mut dyn std::io::Write) -> std::io::Result<()> {
        use Value::*;

        match self {
            Integer(value) => {
                let mut buffer = itoa::Buffer::new();
                output.write(buffer.format(*value).as_bytes())?;
            },
            List(items) => {
                output.write("[".as_bytes())?;

                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        output.write(", ".as_bytes())?;
                    }

                    item.value.json(output)?;
                }

                output.write("]".as_bytes())?;
            },
            Map(items) => {
                output.write("{".as_bytes())?;

                for (index, (WithSpan { value: key, .. }, Object { value, .. })) in items.iter().enumerate() {
                    if index > 0 {
                        output.write(", ".as_bytes())?;
                    }

                    Value::String::<T>(key.clone()).json(output)?;
                    output.write(":".as_bytes())?;
                    value.json(output)?;
                }

                output.write("}".as_bytes())?;
            },
            Null => {
                output.write("null".as_bytes())?;
            },
            String(value) => {
                output.write_fmt(format_args!("\"{}\"", value.replace("\"", "\\\"")))?;
            },
            _ => todo!(),
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Object<T: CharCounter> {
    pub span: Span<T>,
    pub value: Value<T>,
}

#[derive(Debug)]
pub struct WithSpan<T, S: CharCounter> {
    pub span: Span<S>,
    pub value: T,
}

#[derive(Debug)]
pub enum Error<T: CharCounter> {
    InvalidIndent(Span<T>),
    InvalidIndentSize(Span<T>),
    MissingListClose(Span<T>),
    MissingMapClose(Span<T>),
    MissingMapSemicolon(Span<T>),
    MissingMapValue(Span<T>),
}

#[derive(Debug)]
pub struct Comment<T: CharCounter> {
    span: Span<T>,
    value: String,
}

impl<'a, T: CharCounter> Parser<'a, T> {
    // Only returns None if the first character is \n, #, or EOF.
    fn accept_expr(&mut self, break_chars: [char; 2]) -> Result<Option<Object<T>>, ()> {
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
                self.chars.advance();
                self.pop_whitespace();

                let mut items = Vec::new();

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
            '{' => {
                self.chars.advance();
                self.pop_whitespace();

                let mut items = Vec::new();

                loop {
                    let key_start_marker = self.chars.marker();
                    let key = self.chars.pop_while(|ch| ch != ':' && ch != '}');

                    if key.is_empty() {
                        break;
                    }

                    let key_span = Span(key_start_marker, self.chars.marker());

                    self.pop_whitespace();

                    if !self.chars.pop_char(':') {
                        self.errors.push(Error::MissingMapSemicolon(Span::point(&self.chars.marker())));
                        return Err(());
                    }

                    self.pop_whitespace();

                    if let Some(value) = self.accept_expr([',', '}'])? {
                        items.push((WithSpan { span: key_span, value: key.to_string() }, value));
                    } else {
                        self.errors.push(Error::MissingMapValue(Span::point(&self.chars.marker())));
                        return Err(());
                    }

                    if !self.chars.pop_char(',') {
                        break;
                    }
                }

                if !self.chars.pop_char('}') {
                    self.errors.push(Error::MissingMapClose(Span::point(&self.chars.marker())));
                    return Err(());
                }

                Value::Map(items)
            },
            _ if ch.is_digit(10) => {
                let string = self.chars.pop_while(|ch| ch.is_digit(10));
                Value::Integer(string.parse().unwrap())
            },
            _ => {
                let string = self.chars.pop_while(|ch| ch != break_chars[0] && ch != break_chars[1] && ch != '\n' && ch != '#');
                Value::String(string.to_string())
            },
        };

        Ok(Some(Object {
            span: Span(start_marker, self.chars.marker()),
            value,
        }))
    }

    fn reduce_stack(&mut self, level: usize) -> Option<Object<T>> {
        while self.stack.len() > level {
            let item = self.stack.pop().unwrap();

            let object = match item {
                StackItem::List { items, start_marker } => {
                    Object {
                        span: Span(start_marker.unwrap(), items.last().and_then(|item| Some(item.span.1)).unwrap_or(start_marker.unwrap())),
                        value: Value::List(items),
                    }
                },
                _ => todo!(),
            };

            match self.stack.last_mut() {
                Some(StackItem::List { items, .. }) => {
                    items.push(object);
                },
                None => return Some(object),
                _ => todo!(),
            }
        }

        None
    }

    fn accept(&mut self) -> Result<Object<T>, ()> {
        loop {
            let line_start_marker = self.chars.marker();

            let indent = match self.chars.peek() {
                Some(' ') => {
                    let spaces = self.chars.pop_while(|ch| ch == ' ');

                    Some(Indent {
                        kind: IndentKind::Spaces,
                        size: spaces.len(),
                    })
                },
                Some('\t') => {
                    let spaces = self.chars.pop_while(|ch| ch == '\t');

                    Some(Indent {
                        kind: IndentKind::Tabs,
                        size: spaces.len(),
                    })
                },
                _ => None,
            };

            match self.chars.peek() {
                // Empty line
                Some('\n' | '#') | None => (),

                // Non-empty line
                _ => {
                    // indent_level = max number of items in stack before processing
                    let indent_level = match indent {
                        Some(indent) => {
                            match &self.indent {
                                Some(first_indent) => {
                                    match first_indent.calc(&indent) {
                                        Some(level) => level,
                                        None => {
                                            self.errors.push(Error::InvalidIndentSize(Span(line_start_marker, self.chars.marker())));
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

                    assert!(self.reduce_stack(indent_level).is_none());

                    let current_level = self.stack.len();

                    // if self.stack.is_empty() {
                    //     if let Some(expr) = self.accept_expr(['\x00', '\x00'])? {
                    //         eprintln!("{:?}", expr);
                    //     }
                    // } else {
                    let level_diff = indent_level - current_level;

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
                            let item_start_marker = self.chars.marker();

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

                            // eprintln!("{:?}", std::str::from_utf8(&self.chars.bytes[self.chars.byte_offset..]));

                            if let Some(expr) = self.accept_expr(['\x00', '\x00'])? {
                                match (self.stack.last_mut(), level_diff) {
                                    // -
                                    //   - y
                                    //
                                    // x:
                                    //   - y
                                    // (Some(StackItem::List { start_marker: ref mut start_marker @ None, .. }), 1) => { // | (Some(StackItem::Map { key: Some(_), .. }) | None, 1) => {
                                    //     *start_marker = Some(item_start_marker);

                                    //     self.stack.push(StackItem::List {
                                    //         items: vec![expr],
                                    //         start_marker: Some(item_start_marker),
                                    //     });
                                    // },

                                    // - x
                                    // - y
                                    //
                                    // -
                                    //   - x
                                    (Some(StackItem::List { items, ref mut start_marker }), 0) => {
                                        *start_marker = start_marker.or(Some(item_start_marker));
                                        items.push(expr);
                                    },

                                    _ => {
                                        self.errors.push(Error::InvalidIndent(Span::point(&self.chars.marker())));
                                        return Err(());
                                    },
                                }
                            } else {
                                match (self.stack.last_mut(), level_diff) {
                                    // -
                                    //   -
                                    //     - y [not reached yet]
                                    (Some(StackItem::List { start_marker: ref mut start_marker @ None, .. }), 1) => {
                                        *start_marker = Some(item_start_marker);

                                        self.stack.push(StackItem::List {
                                            items: Vec::new(),
                                            start_marker: None,
                                        });
                                    },

                                    // - x
                                    // -
                                    //   - y [not reached yet]
                                    (Some(StackItem::List { .. }), 0) => {
                                        self.stack.push(StackItem::List {
                                            items: Vec::new(),
                                            start_marker: None,
                                        });
                                    },

                                    // x:
                                    //   -
                                    //     - y [not reached yet]
                                    // (Some(StackItem::Map { key: Some(_), .. }) | None, 1) => {
                                    //

                                    // -
                                    //   - x [not reached yet]
                                    (None, 1) => {
                                        self.stack.push(StackItem::List {
                                            items: Vec::new(),
                                            start_marker: Some(item_start_marker),
                                        });

                                        self.stack.push(StackItem::List {
                                            items: Vec::new(),
                                            start_marker: None,
                                        });
                                    },

                                    _ => {
                                        self.errors.push(Error::InvalidIndent(Span::point(&self.chars.marker())));
                                        return Err(());
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
                    let value = self.chars.pop_while(|ch| ch != '\n').to_string();

                    comment = Some(Comment {
                        span: Span(comment_start_marker, self.chars.marker()),
                        value,
                    });

                    self.chars.pop();
                },
                None => {
                    break;
                },
                _ => {
                    return Err(());
                },
            }

            // eprintln!("Comment: {:#?}", comment);
        }

        Ok(self.reduce_stack(0).unwrap())
    }

    // fn accept_value(&mut self) -> Result<Object, ()> {
    //     if let Some(expr) = self.accept_expr(['\x00', '\x00'])? {
    //         Ok(expr)
    //     } else {
    //         Err(())
    //     }
    // }

    fn accept_key(&mut self) -> Option<(String, Span<T>)>{
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
