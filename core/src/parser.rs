use crate::iterator::{CharIterator, CharIndexer, CharIndex, Marker};


#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Span<Index: CharIndex>(pub Marker<Index>, pub Marker<Index>);

impl<Index: CharIndex> Span<Index> {
    fn point(marker: &Marker<Index>) -> Self {
        Self(*marker, *marker)
    }

    #[cfg(feature = "format")]
    pub fn format(&self, contents: &str, output: &mut dyn std::io::Write) -> std::io::Result<()> {
        use unicode_segmentation::UnicodeSegmentation;

        let mut iterator: CharIterator<'_, crate::indexers::CharacterLineColumn> = CharIterator::new(contents);
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

        let last_line_number_fmt = format!("{}", span_end_marker.index.line + 1);

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

            let span_width = span_end_marker.index.column - span_start_marker.index.column;

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
                let line_number = span_start_marker.index.line + relative_line_number;

                output.write_fmt(format_args!("{: >width$} | ", line_number + 1, width = line_number_width))?;

                output.write(&contents[line_start_marker.byte_offset..line_end_marker.byte_offset].as_bytes())?;
                output.write(&['\n' as u8])?;

                output.write(&[' ' as u8].repeat(line_number_width))?;
                output.write(" | ".as_bytes())?;

                if relative_line_number == 0 {
                    output.write(&[' ' as u8].repeat(span_start_marker.index.column))?;
                    output.write(&['^' as u8].repeat(line_end_marker.index.column - span_start_marker.index.column))?;
                    output.write(&['-' as u8])?;
                } else if relative_line_number == line_markers.len() - 1 {
                    output.write(&['^' as u8].repeat(span_end_marker.index.column))?;

                    if extend_last_line {
                        output.write(&['-' as u8])?;
                    }
                } else {
                    output.write(&['^' as u8].repeat(line_end_marker.index.column))?;
                    output.write(&['-' as u8])?;
                }

                output.write(&['\n' as u8])?;
            }
        }

        Ok(())
    }
}


#[derive(Debug)]
enum StackItemKind<Index: CharIndex> {
    List {
        floating_handle_end_marker: Option<Marker<Index>>,
        items: Vec<Object<Index>>,
        start_marker: Marker<Index>,
    },
    Map {
        entries: Vec<(WithSpan<String, Index>, Object<Index>)>,
        floating_key: Option<WithSpan<String, Index>>,
    },
    // String(String),
}

#[derive(Debug)]
struct StackItem<Index: CharIndex> {
    comments: Vec<Comment<Index>>,
    kind: StackItemKind<Index>,
    indent: usize,
}

#[derive(Debug)]
pub struct Parser<'a, Indexer: CharIndexer> {
    chars: CharIterator<'a, Indexer>,
    pub errors: Vec<Error<Indexer::Index>>,
    stack: Vec<StackItem<Indexer::Index>>,
}

impl<'a, Indexer: CharIndexer> Parser<'a, Indexer> {
    pub fn new(contents: &'a str) -> Self {
        Self {
            chars: CharIterator::new(contents),
            errors: Vec::new(),
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
pub enum Value<Index: CharIndex> {
    Bool(bool),
    Float(f64),
    Integer(i64),
    List(Vec<Object<Index>>),
    Map(Vec<(WithSpan<String, Index>, Object<Index>)>),
    Null,
    String(String),
}

#[cfg(test)]
impl<Index: CharIndex> Value<Index> {
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

                    Value::String::<Index>(key.clone()).format_json(output)?;
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
pub struct WithSpan<T, Index: CharIndex> {
    pub span: Span<Index>,
    pub value: T,
}

impl<T, Index: CharIndex> WithSpan<T, Index> {
    fn new(value: T, span: Span<Index>) -> Self {
        Self {
            span,
            value,
        }
    }
}

pub type Object<Index> = WithSpan<Value<Index>, Index>;

pub type Error<Index> = WithSpan<ErrorKind, Index>;

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

    // x: { a:
    MissingCompactMapValue,

    // x:
    MissingExpandedMapValue,

    // x: 3.4.5
    InvalidScalarLiteral,
}

#[derive(Clone, Debug)]
pub struct Comment<Index: CharIndex> {
    span: Span<Index>,
    value: String,
}

#[derive(Debug)]
struct ListHandle<Index: CharIndex> {
    end_marker: Marker<Index>,
    item_indent: usize,
}

#[derive(Debug)]
enum Node<Index: CharIndex> {
    ListOpen {
        handle: ListHandle<Index>,
    },
    ListItem {
        handle: ListHandle<Index>,
        object: Object<Index>,
    },
    MapKey {
        handle: Option<ListHandle<Index>>,
        key: WithSpan<String, Index>,
    },
    MapEntry {
        handle: Option<ListHandle<Index>>,
        key: WithSpan<String, Index>,
        value: Object<Index>,
    }
}


impl<'a, Indexer: CharIndexer> Parser<'a, Indexer> {
    // Only returns None if the first character is \n, #, or EOF.
    fn accept_expr(&mut self, break_chars: &[char]) -> Result<Option<Object<Indexer::Index>>, ()> {
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
                        self.errors.push(Error::new(ErrorKind::MissingCompactMapValue, Span::point(&self.chars.marker())));
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

    fn reduce_stack(&mut self, level: usize) -> Option<Object<Indexer::Index>> {
        while self.stack.len() > level {
            let item = self.stack.pop().unwrap();

            let object = match item.kind {
                StackItemKind::List { floating_handle_end_marker, items, start_marker, .. } => {
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
                StackItemKind::Map { entries, floating_key: None } => {
                    Object {
                        span: Span(
                            entries.first().unwrap().0.span.0,
                            entries.last().unwrap().1.span.1,
                        ),
                        value: Value::Map(entries),
                    }
                },
                StackItemKind::Map { entries, floating_key: Some(floating_key) } => {
                    self.errors.push(Error::new(ErrorKind::MissingExpandedMapValue, floating_key.span));

                    Object {
                        span: Span(
                            entries.first().and_then(|(key, _)| Some(key.span.0)).unwrap_or(floating_key.span.0),
                            floating_key.span.1,
                        ),
                        value: Value::Map(entries),
                    }
                },
            };

            match self.stack.last_mut().and_then(|item| Some(&mut item.kind)) {
                Some(StackItemKind::List { items, .. }) => {
                    items.push(object);
                },
                Some(StackItemKind::Map { entries, floating_key: ref mut key @ Some(_) }) => {
                    let key = std::mem::replace(key, None).unwrap();
                    entries.push((key, object));
                },
                None => return Some(object),
                _ => todo!(),
            }
        }

        None
    }

    pub fn parse(&mut self) -> Result<Object<Indexer::Index>, ()> {
        let mut preceding_lines = Vec::new();

        loop {
            // eprintln!("{:?}", std::str::from_utf8(&self.chars.bytes[self.chars.byte_offset..]).unwrap());

            let line_start_marker = self.chars.marker();

            if self.chars.peek().is_none() {
                break;
            }

            let indent = self.chars.pop_while(|ch| ch == ' ').len();

            match self.chars.peek() {
                // Whitespace-only line
                Some('\n' | '#') | None => {
                    let comment = self.accept_line_end();
                    preceding_lines.push((indent, comment));

                    continue;
                },
                _ => (),
            }

            let content_start_marker = self.chars.marker();

            let nested = match self.stack.last() {
                Some(last_item) if indent > last_item.indent => {
                    true
                },
                Some(_) => {
                    let current_item = self.stack.iter().enumerate().find(|(index, item)| item.indent == indent);

                    if let Some((index, item)) = current_item {
                        assert!(self.reduce_stack(index + 1).is_none());
                        false
                    } else {
                        self.errors.push(Error::new(ErrorKind::InvalidIndentSize, Span(line_start_marker, content_start_marker)));
                        self.accept_line_end(); // TODO: Avoid extraneous chars error
                        preceding_lines.clear();

                        continue;
                    }
                },
                None if indent == 0 => {
                    true
                },
                _ => {
                    self.errors.push(Error::new(ErrorKind::InvalidIndentSize, Span(line_start_marker, content_start_marker)));
                    self.accept_line_end();
                    preceding_lines.clear();

                    continue;
                },
            };

            // eprintln!("{} {} {}", indent_level, current_level, std::str::from_utf8(&self.chars.bytes[self.chars.byte_offset..]).unwrap());

            let handle = match self.chars.peek() {
                Some('-') => {
                    self.chars.advance();
                    let handle_end_marker = self.chars.marker();

                    self.pop_whitespace();

                    Some(ListHandle {
                        end_marker: handle_end_marker,
                        item_indent: self.chars.byte_offset - line_start_marker.byte_offset,
                    })
                },
                _ => None,
            };

            let line_item = if let Some(key) = self.accept_key() {
                match self.accept_expr(&[]) {
                    // [-] x: y
                    Ok(Some(value)) => {
                        Some(Node::MapEntry {
                            handle,
                            key,
                            value,
                        })
                    },

                    // [-] x:
                    Ok(None) => {
                        Some(Node::MapKey {
                            handle,
                            key,
                        })
                    },

                    Err(_) => {
                        None
                    },
                }
            } else if let Some(handle) = handle {
                match self.accept_expr(&[]) {
                    // - x
                    Ok(Some(item)) => {
                        Some(Node::ListItem {
                            handle,
                            object: item,
                        })
                    },

                    // -
                    Ok(None) => {
                        Some(Node::ListOpen {
                            handle,
                        })
                    },

                    Err(_) => {
                        None
                    },
                }
            } else {
                None
            };

            let content_end_marker = self.chars.marker();

            self.pop_whitespace();

            let comment = self.accept_line_end();
            // let mut line_comments = std::mem::replace(&mut preceding_comments, Vec::new());

            let node = match line_item {
                Some(node) => node,
                None => {
                    continue;
                },
            };

            match (node, self.stack.last_mut().and_then(|item| Some(&mut item.kind)), nested) {
                // [root]
                // -
                (Node::ListOpen { handle }, None, true) => {
                    self.stack.push(StackItem {
                        comments: preceding_lines
                            .iter()
                            .filter_map(|(_, comment)| comment.clone())
                            .collect(),
                        kind: StackItemKind::List {
                            floating_handle_end_marker: Some(handle.end_marker),
                            items: Vec::new(),
                            start_marker: content_start_marker,
                        },
                        indent,
                    });
                },

                // - a
                // -
                // (LineItem::ListOpen, Some(StackItem { kind: StackItemKind::List { .. }, .. }), false) => {
                (Node::ListOpen { handle }, Some(StackItemKind::List { .. }), false) => {
                    self.stack.push(StackItem {
                        comments: Vec::new(),
                        kind: StackItemKind::List {
                            floating_handle_end_marker: Some(handle.end_marker),
                            items: Vec::new(),
                            start_marker: content_start_marker,
                        },
                        indent,
                    });
                },

                // a:
                //   - x
                //
                // [root]
                // - x
                (Node::ListItem { handle, object }, Some(StackItemKind::Map { floating_key: Some(_), .. }) | None, true) => {
                    self.stack.push(StackItem {
                        comments: Vec::new(),
                        kind: StackItemKind::List {
                            floating_handle_end_marker: None,
                            items: vec![object],
                            start_marker: content_start_marker,
                        },
                        indent,
                    });
                },

                // -
                //   - x
                (Node::ListItem { object, .. }, Some(StackItemKind::List { floating_handle_end_marker: Some(_), .. }), true) => {
                    self.stack.push(StackItem {
                        comments: Vec::new(),
                        kind: StackItemKind::List {
                            floating_handle_end_marker: None,
                            items: vec![object],
                            start_marker: content_start_marker,
                        },
                        indent,
                    });
                },

                // - a
                // - x
                (Node::ListItem { object, .. }, Some(StackItemKind::List { floating_handle_end_marker: None, items, .. }), false) => {
                    items.push(object);
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
                (Node::MapEntry { handle, key, value }, Some(StackItemKind::Map { floating_key: Some(_), .. }) | None, true) => {
                    if handle.is_some() {
                        self.stack.push(StackItem {
                            comments: Vec::new(),
                            kind: StackItemKind::List {
                                floating_handle_end_marker: None,
                                items: Vec::new(),
                                start_marker: content_start_marker,
                            },
                            indent,
                        });
                    }

                    self.stack.push(StackItem {
                        comments: Vec::new(),
                        kind: StackItemKind::Map {
                            entries: vec![(key, value)],
                            floating_key: None,
                        },
                        indent: handle.and_then(|handle| Some(handle.item_indent)).unwrap_or(indent),
                    });
                },

                // - a
                // - x: y
                (Node::MapEntry { handle: Some(handle), key, value }, Some(StackItemKind::List { .. }), false) => {
                    self.stack.push(StackItem {
                        comments: Vec::new(),
                        kind: StackItemKind::Map {
                            entries: vec![(key, value)],
                            floating_key: None,
                        },
                        indent: handle.item_indent,
                    });
                },

                // a: b
                // x: y
                (Node::MapEntry { handle: None, key, value }, Some(StackItemKind::Map { entries, floating_key, .. }), false) => {
                    debug_assert!(floating_key.is_none());
                    entries.push((key, value));
                },

                // a: b
                // x:
                (Node::MapKey { handle: None, key }, Some(StackItemKind::Map { ref mut floating_key, .. }), false) => {
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
                (Node::MapKey { handle, key }, Some(StackItemKind::Map { floating_key: Some(_), .. }) | None, true) => {
                    if handle.is_some() {
                        self.stack.push(StackItem {
                            comments: Vec::new(),
                            kind: StackItemKind::List {
                                floating_handle_end_marker: None,
                                items: Vec::new(),
                                start_marker: content_start_marker,
                            },
                            indent,
                        });
                    }

                    self.stack.push(StackItem {
                        comments: Vec::new(),
                        kind: StackItemKind::Map {
                            entries: Vec::new(),
                            floating_key: Some(key),
                        },
                        indent: handle.and_then(|handle| Some(handle.item_indent)).unwrap_or(indent),
                    });
                },

                (node, _, _) => {
                    eprintln!("Missing: {:#?} {:#?} {:#?}", &node, self.stack.last(), nested);
                    self.errors.push(Error::new(ErrorKind::InvalidIndent, Span(content_start_marker, content_end_marker)));
                },
            }

            // eprintln!("Comment: {:#?}", comment);
        }

        // eprintln!("Stack: {:#?}", self.stack);
        self.reduce_stack(0).ok_or(())
    }

    fn accept_line_end(&mut self) -> Option<Comment<Indexer::Index>> {
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

    fn accept_key(&mut self) -> Option<WithSpan<String, Indexer::Index>>{
        match self.chars.peek() {
            Some('A'..='Z' | 'a'..='z' | '_') => {
                let key_start_offset = self.chars.byte_offset;
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
                        self.chars.byte_offset = key_start_offset;
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
pub struct ParseResult<Index: CharIndex> {
    pub errors: Vec<Error<Index>>,
    pub object: Option<Object<Index>>,
}

#[cfg(test)]
impl<Index: CharIndex> ParseResult<Index> {
    pub fn json(&self) -> Option<String> {
        self.object.as_ref().and_then(|obj| obj.value.json().ok())
    }
}

pub fn parse<Indexer: CharIndexer>(input: &str) -> ParseResult<Indexer::Index> {
    let mut parser = Parser::<'_, Indexer>::new(input);
    let object = parser.parse();

    ParseResult {
        errors: parser.errors,
        object: object.ok(),
    }
}
