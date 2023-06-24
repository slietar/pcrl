use crate::iterator::{CharIterator, CharIteratorMarker, CharCounter};


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span<T: CharCounter>(pub CharIteratorMarker<T>, pub CharIteratorMarker<T>);

impl<T: CharCounter> Span<T> {
    fn point(marker: &CharIteratorMarker<T>) -> Self {
        Self(*marker, *marker)
    }

    pub fn format(&self, contents: &str, output: &mut dyn std::io::Write) -> std::io::Result<()> {
        let mut iterator: CharIterator<'_, crate::counters::CharacterLineColumn> = CharIterator::new(contents);
        let mut current_line_start_marker = iterator.marker();

        while iterator.byte_offset < self.0.byte_offset {
            if iterator.pop().unwrap() == '\n' {
                current_line_start_marker = iterator.marker();
            }
        }

        let span_start_marker = iterator.marker();
        let mut line_markers = Vec::new();
        let mut span_end_marker = span_start_marker;
        let mut extend_last_line = false;

        while iterator.byte_offset < self.1.byte_offset {
            match iterator.peek().unwrap() {
                '\n' => {
                    let current_marker = iterator.marker();
                    line_markers.push((current_line_start_marker, current_marker));

                    iterator.advance();

                    if iterator.byte_offset == self.1.byte_offset {
                        extend_last_line = true;
                        span_end_marker = current_marker;
                    }

                    current_line_start_marker = iterator.marker();
                },
                _ => {
                    iterator.advance();

                    if iterator.byte_offset == self.1.byte_offset {
                        extend_last_line = false;
                        span_end_marker = iterator.marker();
                    }
                },
            }
        }

        if !extend_last_line || line_markers.is_empty() {
            loop {
                match iterator.peek() {
                    Some('\n') | None => {
                        line_markers.push((current_line_start_marker, iterator.marker()));
                        break;
                    },
                    Some(_) => {
                        iterator.advance();
                    },
                }
            }
        }

        let last_line_number_fmt = format!("{}", span_end_marker.counter.line + 1);

        // Using .len() is ok because digits are all ASCII.
        let line_number_width = last_line_number_fmt.len();

        if line_markers.len() == 1 {
            let (line_start_marker, line_end_marker) = line_markers[0];

            output.write_fmt(format_args!("{} | ", last_line_number_fmt))?;

            output.write(&contents[line_start_marker.byte_offset..line_end_marker.byte_offset].as_bytes())?;
            output.write(&['\n' as u8])?;

            output.write(&[' ' as u8].repeat(line_number_width))?;
            output.write(" | ".as_bytes())?;

            output.write(&[' ' as u8].repeat(span_start_marker.counter.column))?;

            let span_width = span_end_marker.counter.column - span_start_marker.counter.column;

            if span_width > 0 {
                output.write(&['^' as u8].repeat(span_end_marker.counter.column - span_start_marker.counter.column))?;

                if extend_last_line {
                    output.write(&['-' as u8])?;
                }
            } else {
                output.write(&['~' as u8])?;
            }

            output.write(&['\n' as u8])?;
        } else {
            for (relative_line_number, (line_start_marker, line_end_marker)) in line_markers.iter().enumerate() {
                let line_number = span_start_marker.counter.line + relative_line_number;

                output.write_fmt(format_args!("{: >width$} | ", line_number + 1, width = line_number_width))?;

                output.write(&contents[line_start_marker.byte_offset..line_end_marker.byte_offset].as_bytes())?;
                output.write(&['\n' as u8])?;

                output.write(&[' ' as u8].repeat(line_number_width))?;
                output.write(" | ".as_bytes())?;

                if relative_line_number == 0 {
                    output.write(&[' ' as u8].repeat(span_start_marker.counter.column))?;
                    output.write(&['^' as u8].repeat(line_end_marker.counter.column - span_start_marker.counter.column))?;
                    output.write(&['-' as u8])?;
                } else if relative_line_number == line_markers.len() - 1 {
                    output.write(&['^' as u8].repeat(span_end_marker.counter.column))?;

                    if extend_last_line {
                        output.write(&['-' as u8])?;
                    }
                } else {
                    output.write(&['^' as u8].repeat(line_end_marker.counter.column))?;
                    output.write(&['-' as u8])?;
                }

                output.write(&['\n' as u8])?;
            }
        }

        Ok(())
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
        entries: Vec<(WithSpan<String, T>, Object<T>)>,
        floating_key: Option<WithSpan<String, T>>,
    },
    String(String),
}

#[derive(Debug)]
pub struct Parser<'a, T: CharCounter> {
    chars: CharIterator<'a, T>,
    pub errors: Vec<Error<T>>,
    indent: Option<Indent>,
    stack: Vec<StackItem<T>>,
}

impl<'a, T: CharCounter> Parser<'a, T> {
    pub fn new(contents: &'a str) -> Self {
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
    Bool(bool),
    Float(f64),
    Integer(i64),
    List(Vec<Object<T>>),
    Map(Vec<(WithSpan<String, T>, Object<T>)>),
    Null,
    String(String),
}

#[cfg(test)]
impl<T: CharCounter> Value<T> {
    pub fn json(&self) -> std::io::Result<String> {
        let mut buffer = Vec::new();
        self.format_json(&mut buffer)?;
        Ok(std::str::from_utf8(&buffer).unwrap().to_owned())
    }

    fn format_json(&self, output: &mut dyn std::io::Write) -> std::io::Result<()> {
        use Value::*;

        match self {
            Bool(value) => {
                if *value {
                    output.write("true".as_bytes())?;
                } else {
                    output.write("false".as_bytes())?;
                }
            },
            Integer(value) => {
                let mut buffer = itoa::Buffer::new();
                output.write(buffer.format(*value).as_bytes())?;
            },
            Float(value) => {
                if *value == f64::INFINITY {
                    output.write("Infinity".as_bytes())?;
                } else if *value == f64::NEG_INFINITY {
                    output.write("-Infinity".as_bytes())?;
                } else if *value == f64::NAN {
                    output.write("NaN".as_bytes())?;
                } else {
                    // let mut buffer = ryu::Buffer::new();
                    // output.write(buffer.format(*value).as_bytes())?;

                    output.write_fmt(format_args!("{}", value))?;
                }
            },
            List(items) => {
                output.write("[".as_bytes())?;

                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        output.write(", ".as_bytes())?;
                    }

                    item.value.format_json(output)?;
                }

                output.write("]".as_bytes())?;
            },
            Map(items) => {
                output.write("{ ".as_bytes())?;

                for (index, (WithSpan { value: key, .. }, Object { value, .. })) in items.iter().enumerate() {
                    if index > 0 {
                        output.write(", ".as_bytes())?;
                    }

                    Value::String::<T>(key.clone()).format_json(output)?;
                    output.write(": ".as_bytes())?;
                    value.format_json(output)?;
                }

                output.write(" }".as_bytes())?;
            },
            Null => {
                output.write("null".as_bytes())?;
            },
            String(value) => {
                output.write_fmt(format_args!("\"{}\"", value.replace("\"", "\\\"")))?;
            },
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct WithSpan<T, S: CharCounter> {
    pub span: Span<S>,
    pub value: T,
}

impl<T, S: CharCounter> WithSpan<T, S> {
    fn new(value: T, span: Span<S>) -> Self {
        Self {
            span,
            value,
        }
    }
}

type Object<T> = WithSpan<Value<T>, T>;

type Error<T> = WithSpan<ErrorKind, T>;

#[derive(Debug)]
pub enum ErrorKind {
    // -
    // - a
    EmptyExpandedList,

    // x: [3, 4] a
    ExtraneousChars,

    // [...]
    InvalidIndent,

    // x: 3
    //   y: 4
    //  z: 5
    InvalidIndentSize,

    // x: [3, 4
    MissingListClose,

    // x: { a: 3
    MissingMapClose,

    // x: { a
    MissingMapSemicolon,

    // x:
    // y:
    MissingMapValue,

    // x: 3.4.5
    InvalidScalarLiteral,

    // $
    InvalidValue,
}

#[derive(Debug)]
pub struct Comment<T: CharCounter> {
    span: Span<T>,
    value: String,
}

#[derive(Debug)]
enum LineItem<T: CharCounter> {
    ListOpen,
    ListItem(Object<T>),
    MapKey {
        key: WithSpan<String, T>,
        list: bool,
    },
    MapEntry {
        key: WithSpan<String, T>,
        list: bool,
        value: Object<T>,
    }
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
                    self.errors.push(Error::new(ErrorKind::MissingListClose, Span::point(&self.chars.marker())));
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
                        self.errors.push(Error::new(ErrorKind::MissingMapSemicolon, Span::point(&self.chars.marker())));
                        return Err(());
                    }

                    self.pop_whitespace();

                    if let Some(value) = self.accept_expr([',', '}'])? {
                        items.push((WithSpan { span: key_span, value: key.to_string() }, value));
                    } else {
                        self.errors.push(Error::new(ErrorKind::MissingMapValue, Span::point(&self.chars.marker())));
                        return Err(());
                    }

                    if !self.chars.pop_char(',') {
                        break;
                    }
                }

                if !self.chars.pop_char('}') {
                    self.errors.push(Error::new(ErrorKind::MissingMapClose, Span::point(&self.chars.marker())));
                    return Err(());
                }

                Value::Map(items)
            },
            '+' if self.chars.pop_constant("+inf") => {
                Value::Float(f64::INFINITY)
            },
            '-' if self.chars.pop_constant("-inf") => {
                Value::Float(f64::NEG_INFINITY)
            },
            '+' | '-' | '0'..='9' | '.' => {
                let string = self.chars.pop_until(|ch| ch != break_chars[0] && ch != break_chars[1] && ch != '\n' && ch != '#', |ch| ch == ' ');

                if let Ok(value) = string.parse::<i64>() {
                    Value::Integer(value)
                } else {
                    if let Ok(value) = string.parse::<f64>() {
                        Value::Float(value)
                    } else {
                        self.errors.push(Error::new(ErrorKind::InvalidScalarLiteral, Span(start_marker, self.chars.marker())));
                        return Err(());
                    }
                }
            },
            'n' if self.chars.pop_constant("null") => {
                Value::Null
            },
            't' if self.chars.pop_constant("true") => {
                Value::Bool(true)
            },
            'f' if self.chars.pop_constant("false") => {
                Value::Bool(false)
            },
            'i' if self.chars.pop_constant("inf") => {
                Value::Float(f64::INFINITY)
            },
            'n' if self.chars.pop_constant("nan") => {
                Value::Float(f64::NAN)
            },
            _ => {
                let string = self.chars.pop_until(|ch| ch != break_chars[0] && ch != break_chars[1] && ch != '\n' && ch != '#', |ch| ch == ' ');
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
                StackItem::List { items, start_marker: Some(start_marker) } => {
                    Object {
                        span: Span(start_marker, items.last().and_then(|item| Some(item.span.1)).unwrap_or(start_marker)),
                        value: Value::List(items),
                    }
                },
                StackItem::List { .. } => {
                    let head = self.stack.last().unwrap();

                    match head {
                        StackItem::List { start_marker: Some(start_marker), .. } => {
                            self.errors.push(Error::new(ErrorKind::EmptyExpandedList, Span(*start_marker, self.chars.marker())));
                        },
                        _ => unreachable!(),
                    }

                    continue;
                },
                StackItem::Map { entries, floating_key: None, } => {
                    Object {
                        span: Span(self.chars.marker(), self.chars.marker()), // TODO: fix
                        value: Value::Map(entries),
                    }
                },
                _ => todo!(),
            };

            match self.stack.last_mut() {
                Some(StackItem::List { items, .. }) => {
                    items.push(object);
                },
                Some(StackItem::Map { entries, floating_key: ref mut key @ Some(_) }) => {
                    let key = std::mem::replace(key, None).unwrap();
                    entries.push((key, object));
                },
                None => return Some(object),
                _ => todo!(),
            }
        }

        None
    }

    pub fn parse(&mut self) -> Result<Object<T>, ()> {
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
                                            self.errors.push(Error::new(ErrorKind::InvalidIndentSize, Span(line_start_marker, self.chars.marker())));
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
                    let level_diff = indent_level - current_level;

                    let item_start_marker = self.chars.marker();

                    let is_list_item = match self.chars.peek() {
                        Some('-') => {
                            self.chars.advance();
                            self.pop_whitespace();

                            true
                        },
                        _ => false
                    };

                    let line_item = if let Some(key) = self.accept_key() {
                        // [-] x: y
                        if let Some(value) = self.accept_expr(['\x00', '\x00'])? {
                            LineItem::MapEntry {
                                key,
                                list: is_list_item,
                                value,
                            }
                        // [-] x:
                        } else {
                            LineItem::MapKey {
                                key,
                                list: is_list_item,
                            }
                        }
                    } else if is_list_item {
                        // - x
                        if let Some(expr) = self.accept_expr(['\x00', '\x00'])? {
                            LineItem::ListItem(expr)
                        // -
                        } else {
                            assert!(is_list_item);
                            LineItem::ListOpen
                        }
                    } else {
                        self.errors.push(Error::new(ErrorKind::InvalidValue, Span::point(&item_start_marker)));
                        return Err(());
                    };

                    self.pop_whitespace();

                    let extraneous_chars_start_marker = self.chars.marker();
                    let extraneous_chars = self.chars.pop_while(|ch| ch != '\n' && ch != '#');

                    if !extraneous_chars.is_empty() {
                        self.errors.push(Error::new(ErrorKind::ExtraneousChars, Span(extraneous_chars_start_marker, self.chars.marker())));
                    }

                    match (line_item, self.stack.last_mut(), level_diff) {
                        // [root]
                        // -
                        (LineItem::ListOpen, None, 1) => {
                            self.stack.push(StackItem::List {
                                items: Vec::new(),
                                start_marker: Some(item_start_marker),
                            });

                            self.stack.push(StackItem::List {
                                items: Vec::new(),
                                start_marker: None,
                            });
                        },

                        // - a
                        // -
                        (LineItem::ListOpen, Some(StackItem::List { .. }), 0) => {
                            self.stack.push(StackItem::List {
                                items: Vec::new(),
                                start_marker: None,
                            });
                        },

                        // a:
                        //   - x
                        //
                        // [root]
                        // - x
                        (LineItem::ListItem(item), Some(StackItem::Map { floating_key: Some(_), .. }) | None, 1) => {
                            self.stack.push(StackItem::List {
                                items: vec![item],
                                start_marker: Some(item_start_marker),
                            });
                        },

                        // -
                        //   - x
                        //
                        // - a
                        // - b
                        (LineItem::ListItem(item), Some(StackItem::List { items, ref mut start_marker }), 0) => {
                            // debug_assert!(start_marker.is_none());
                            *start_marker = Some(item_start_marker);

                            items.push(item);
                        },

                        // a:
                        //   x: y
                        //
                        // [root]
                        // x: y
                        //
                        // a:
                        //   - x: y
                        //
                        // [root]
                        // - x: y
                        (LineItem::MapEntry { key, list, value }, Some(StackItem::Map { floating_key: Some(_), .. }) | None, 1) => {
                            if list {
                                self.stack.push(StackItem::List {
                                    items: Vec::new(),
                                    start_marker: Some(item_start_marker),
                                });
                            }

                            self.stack.push(StackItem::Map {
                                entries: vec![(key, value)],
                                floating_key: None,
                            });
                        },

                        // - a
                        // - x: y
                        (LineItem::MapEntry { key, list: true, value }, Some(StackItem::List { items, .. }), 0) => {
                            self.stack.push(StackItem::Map {
                                entries: vec![(key, value)],
                                floating_key: None,
                            });
                        },

                        // (LineItem::ListItem(item), Some(StackItem::Map { floating_key: Some(_), .. }) | None, 1) => {

                        // },

                        // // [root]
                        // // - x:
                        // (LineItem::MapKey { key, list: true }, None, 1) => {
                        //     self.stack.push(StackItem::List {
                        //         items: Vec::new(),
                        //         start_marker: Some(item_start_marker),
                        //     });

                        //     self.stack.push(StackItem::Map {
                        //         entries: Vec::new(),
                        //         floating_key: Some(key),
                        //     });
                        // },

                        // a: b
                        // x: y
                        (LineItem::MapEntry { key, list: false, value }, Some(StackItem::Map { entries, floating_key, .. }), 0) => {
                            debug_assert!(floating_key.is_none());
                            entries.push((key, value));
                        },

                        // a: b
                        // x:
                        (LineItem::MapKey { key, list: false }, Some(StackItem::Map { ref mut floating_key, .. }), 0) => {
                            debug_assert!(floating_key.is_none());
                            *floating_key = Some(key);
                        },

                        // // a:
                        // //   - x:
                        // (LineItem::MapKey { key, list: true }, Some(StackItem::Map { floating_key: Some(_), .. }), 1) => {
                        //     if true {
                        //         self.stack.push(StackItem::List {
                        //             items: Vec::new(),
                        //             start_marker: Some(item_start_marker),
                        //         });
                        //     }

                        //     self.stack.push(StackItem::Map {
                        //         entries: Vec::new(),
                        //         floating_key: Some(key),
                        //     })
                        // },

                        // a:
                        //   - x:
                        //
                        // a:
                        //   x:
                        //
                        // [root]
                        // x:
                        //
                        // [root]
                        // - x:
                        (LineItem::MapKey { key, list }, Some(StackItem::Map { floating_key: Some(_), .. }) | None, 1) => {
                            if list {
                                self.stack.push(StackItem::List {
                                    items: Vec::new(),
                                    start_marker: Some(item_start_marker),
                                });
                            }

                            self.stack.push(StackItem::Map {
                                entries: Vec::new(),
                                floating_key: Some(key),
                            });
                        },

                        (line_item, _, _) => {
                            eprintln!("Missing: {:#?} {:#?} {:#?}", &line_item, self.stack.last(), level_diff);
                            todo!()
                        },
                    }
                }
            }


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

        // eprintln!("Stack: {:#?}", self.stack);
        Ok(self.reduce_stack(0).unwrap())
    }

    // fn accept_value(&mut self) -> Result<Object, ()> {
    //     if let Some(expr) = self.accept_expr(['\x00', '\x00'])? {
    //         Ok(expr)
    //     } else {
    //         Err(())
    //     }
    // }

    fn accept_key(&mut self) -> Option<WithSpan<String, T>>{
        match self.chars.peek() {
            Some('A'..='Z' | 'a'..='z' | '_') => {
                let key_start_marker = self.chars.marker();

                let key = self.chars.pop_while(|ch| ch.is_alphanumeric() || ch == '_');
                // let key = self.contents[key_start_offset..self.offset].to_string();

                match self.chars.peek() {
                    Some(':') => {
                        let key_end_marker = self.chars.marker();
                        self.chars.advance();

                        Some(WithSpan {
                            span: Span(key_start_marker, key_end_marker),
                            value: key.to_string(),
                        })
                    },
                    _ => {
                        self.chars.restore(&key_start_marker);
                        None
                    },
                }
            },
            // For completion
            // Some(':') => {},
            _ => None,
        }
    }

    fn pop_whitespace(&mut self) {
        self.chars.pop_while(|ch| ch == ' ' || ch == '\t');
    }
}


#[derive(Debug)]
pub struct ParseResult<T: CharCounter> {
    pub errors: Vec<Error<T>>,
    pub object: Option<Object<T>>,
}

#[cfg(test)]
impl<T: CharCounter> ParseResult<T> {
    pub fn json(&self) -> Option<String> {
        self.object.as_ref().and_then(|obj| obj.value.json().ok())
    }
}

pub fn parse<T: CharCounter>(input: &str) -> ParseResult<T> {
    let mut parser = Parser::new(input);
    let object = parser.parse();

    ParseResult {
        errors: parser.errors,
        object: object.ok(),
    }
}
