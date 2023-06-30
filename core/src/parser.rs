use crate::iterator::{CharIterator, CharIteratorMarker, CharCounter};


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span<T: CharCounter>(pub CharIteratorMarker<T>, pub CharIteratorMarker<T>);

impl<T: CharCounter> Span<T> {
    fn point(marker: &CharIteratorMarker<T>) -> Self {
        Self(*marker, *marker)
    }

    #[cfg(feature = "format")]
    pub fn format(&self, contents: &str, output: &mut dyn std::io::Write) -> std::io::Result<()> {
        use unicode_segmentation::UnicodeSegmentation;

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

            let whitespace_width = UnicodeSegmentation::graphemes(&contents[line_start_marker.byte_offset..span_start_marker.byte_offset], true).count();
            output.write(&[' ' as u8].repeat(whitespace_width))?;

            let span_width = span_end_marker.counter.column - span_start_marker.counter.column;

            if span_width > 0 {
                let highlight_width = UnicodeSegmentation::graphemes(&contents[span_start_marker.byte_offset..span_end_marker.byte_offset], true).count();
                output.write(&['^' as u8].repeat(highlight_width))?;

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
        floating_handle_end_marker: Option<CharIteratorMarker<T>>,
        items: Vec<Object<T>>,
        start_marker: CharIteratorMarker<T>,
    },
    Map {
        entries: Vec<(WithSpan<String, T>, Object<T>)>,
        floating_key: Option<WithSpan<String, T>>,
    },
    // String(String),
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

pub type Object<T> = WithSpan<Value<T>, T>;

pub type Error<T> = WithSpan<ErrorKind, T>;

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
    fn accept_expr(&mut self, break_chars: &[char]) -> Result<Option<Object<T>>, ()> {
        self.pop_whitespace();

        let start_marker = self.chars.marker();
        let ch = match self.chars.peek() {
            Some(ch) => ch,
            None => return Ok(None),
        };

        let value = match ch {
            _ if break_chars.contains(&ch) => return Ok(None),
            '\n' => return Ok(None),
            '[' => {
                self.chars.advance();
                self.pop_whitespace();

                let mut items = Vec::new();

                if let Some(first_item) = self.accept_expr(&[',', ']'])? {
                    items.push(first_item);

                    loop {
                        self.pop_whitespace();

                        if !self.chars.pop_char(',') {
                            break;
                        }

                        if let Some(next_item) = self.accept_expr(&[',', ']'])? {
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

                    if let Some(value) = self.accept_expr(&[',', '}'])? {
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
                let string = self.chars.pop_until(|ch| !break_chars.contains(&ch) && ch != '\n' && ch != '#', |ch| ch == ' ');

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
                let string = self.chars.pop_until(|ch| !break_chars.contains(&ch) && ch != '\n' && ch != '#', |ch| ch == ' ');
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
                StackItem::List { floating_handle_end_marker, items, start_marker, .. } => {
                    match items.last() {
                        Some(item) => {
                            Object {
                                span: Span(start_marker, item.span.1),
                                value: Value::List(items),
                            }
                        },
                        None => {
                            // The list can only be empty if there is a floating handle.
                            self.errors.push(Error::new(ErrorKind::EmptyExpandedList, Span(start_marker, floating_handle_end_marker.unwrap())));
                            continue;
                        },
                    }
                },
                StackItem::Map { entries, floating_key: None, } => {
                    Object {
                        span: Span(
                            entries.first().unwrap().0.span.0,
                            entries.last().unwrap().1.span.1,
                        ),
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
            // eprintln!("{:?}", std::str::from_utf8(&self.chars.bytes[self.chars.byte_offset..]).unwrap());

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
                Some(_) => None,
                None => {
                    break;
                },
            };

            match self.chars.peek() {
                // Whitespace-only line
                Some('\n' | '#') | None => {
                    let _comment = self.accept_line_end();
                },

                // Non-whitespace line
                Some(_) => {
                    // indent_level = max number of items in stack before processing
                    let indent_level = match indent {
                        Some(indent) => {
                            match &self.indent {
                                Some(first_indent) => {
                                    match first_indent.calc(&indent) {
                                        Some(level) => level,
                                        None => {
                                            self.errors.push(Error::new(ErrorKind::InvalidIndentSize, Span(line_start_marker, self.chars.marker())));
                                            continue;
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

                    // eprintln!("{} {} {}", indent_level, current_level, std::str::from_utf8(&self.chars.bytes[self.chars.byte_offset..]).unwrap());

                    let item_start_marker = self.chars.marker();
                    let handle_end_marker = match self.chars.peek() {
                        Some('-') => {
                            self.chars.advance();
                            let handle_end_marker = self.chars.marker();

                            self.pop_whitespace();

                            Some(handle_end_marker)
                        },
                        _ => None
                    };

                    let line_item = if let Some(key) = self.accept_key() {
                        match self.accept_expr(&[]) {
                            // [-] x: y
                            Ok(Some(value)) => {
                                Some(LineItem::MapEntry {
                                    list: handle_end_marker.is_some(),
                                    key,
                                    value,
                                })
                            },

                            // [-] x:
                            Ok(None) => {
                                Some(LineItem::MapKey {
                                    list: handle_end_marker.is_some(),
                                    key,
                                })
                            },

                            Err(_) => {
                                None
                            },
                        }
                    } else if handle_end_marker.is_some() {
                        match self.accept_expr(&[]) {
                            // - x
                            Ok(Some(item)) => {
                                Some(LineItem::ListItem(item))
                            },

                            // -
                            Ok(None) => {
                                Some(LineItem::ListOpen)
                            },

                            Err(_) => {
                                None
                            },
                        }
                    } else {
                        None
                    };

                    let item_end_marker = self.chars.marker();

                    self.pop_whitespace();

                    let comment = self.accept_line_end();

                    let line_item = match line_item {
                        Some(line_item) => line_item,
                        None => {
                            continue;
                        }
                    };

                    match (line_item, self.stack.last_mut(), level_diff) {
                        // [root]
                        // -
                        (LineItem::ListOpen, None, 1) => {
                            debug_assert!(handle_end_marker.is_some());

                            self.stack.push(StackItem::List {
                                floating_handle_end_marker: handle_end_marker,
                                items: Vec::new(),
                                start_marker: item_start_marker,
                            });
                        },

                        // - a
                        // -
                        (LineItem::ListOpen, Some(StackItem::List { .. }), 0) => {
                            debug_assert!(handle_end_marker.is_some());

                            self.stack.push(StackItem::List {
                                floating_handle_end_marker: handle_end_marker,
                                items: Vec::new(),
                                start_marker: item_start_marker,
                            });
                        },

                        // a:
                        //   - x
                        //
                        // [root]
                        // - x
                        (LineItem::ListItem(item), Some(StackItem::Map { floating_key: Some(_), .. }) | None, 1) => {
                            self.stack.push(StackItem::List {
                                floating_handle_end_marker: None,
                                items: vec![item],
                                start_marker: item_start_marker,
                            });
                        },

                        // -
                        //   - x
                        (LineItem::ListItem(item), Some(StackItem::List { floating_handle_end_marker: Some(_), .. }), 1) => {
                            self.stack.push(StackItem::List {
                                floating_handle_end_marker: None,
                                items: vec![item],
                                start_marker: item_start_marker,
                            });
                        },

                        // - a
                        // - x
                        (LineItem::ListItem(item), Some(StackItem::List { floating_handle_end_marker: None, items, .. }), 0) => {
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
                                    floating_handle_end_marker: None,
                                    items: Vec::new(),
                                    start_marker: item_start_marker,
                                });
                            }

                            self.stack.push(StackItem::Map {
                                entries: vec![(key, value)],
                                floating_key: None,
                            });
                        },

                        // - a
                        // - x: y
                        (LineItem::MapEntry { key, list: true, value }, Some(StackItem::List { .. }), 0) => {
                            self.stack.push(StackItem::Map {
                                entries: vec![(key, value)],
                                floating_key: None,
                            });
                        },

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
                                    floating_handle_end_marker: None,
                                    items: Vec::new(),
                                    start_marker: item_start_marker,
                                });
                            }

                            self.stack.push(StackItem::Map {
                                entries: Vec::new(),
                                floating_key: Some(key),
                            });
                        },

                        (line_item, _, _) => {
                            eprintln!("Missing: {:#?} {:#?} {:#?}", &line_item, self.stack.last(), level_diff);
                            self.errors.push(Error::new(ErrorKind::InvalidIndent, Span(item_start_marker, item_end_marker)));
                        },
                    }
                },
            }

            // eprintln!("Comment: {:#?}", comment);
        }

        // eprintln!("Stack: {:#?}", self.stack);
        self.reduce_stack(0).ok_or(())
    }

    fn accept_line_end(&mut self) -> Option<Comment<T>> {
        self.pop_whitespace();

        match self.chars.peek() {
            Some('\n') => {
                self.chars.advance();
                None
            },
            Some('#') => {
                let comment_start_marker = self.chars.marker();

                self.pop_whitespace();
                let value = self.chars.pop_while(|ch| ch != '\n').to_string();

                let comment_end_marker = self.chars.marker();

                self.chars.pop();

                Some(Comment {
                    span: Span(comment_start_marker, comment_end_marker),
                    value,
                })
            },
            Some(_) => {
                let extraneous_chars_start_marker = self.chars.marker();
                let extraneous_chars = self.chars.pop_until(|ch| ch != '\n' && ch != '#', |ch| ch == ' ');

                if !extraneous_chars.is_empty() {
                    self.errors.push(Error::new(ErrorKind::ExtraneousChars, Span(extraneous_chars_start_marker, self.chars.marker())));
                }

                self.pop_whitespace();
                self.accept_line_end()
            },
            None => None,
        }
    }

    fn accept_key(&mut self) -> Option<WithSpan<String, T>>{
        match self.chars.peek() {
            Some('A'..='Z' | 'a'..='z' | '_') => {
                let key_start_marker = self.chars.marker();

                let key = self.chars.pop_while(|ch| ch.is_alphanumeric() || ch == '_');
                // let key = self.contents[key_start_offset..self.offset].to_string();

                self.pop_whitespace();

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
