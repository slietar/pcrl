use crate::result::*;
use crate::iterator::{CharIndex, CharIndexer, CharIterator, Marker};
use crate::span::{Span, WithSpan};


#[derive(Debug)]
enum StackItemKind<Index: CharIndex> {
    List {
        floating_handle_end_marker: Option<Marker<Index>>,
        items: Vec<ExpandedListItem<Index>>,
        next_item_context: Option<Context<Index>>,
        start_marker: Marker<Index>,
    },
    Map {
        entries: Vec<ExpandedMapEntry<Index>>,
        floating_key: Option<WithSpan<String, Index>>,
        next_entry_context: Option<Context<Index>>,
    },
    // String(String),
}

#[derive(Debug)]
struct StackItem<Index: CharIndex> {
    indent: usize,
    kind: StackItemKind<Index>,
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


// pub type Object<Index> = WithSpan<Value<Index>, Index>;
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

// #[derive(Clone, Debug)]
// pub struct Comment<Index: CharIndex> {
//     span: Span<Index>,
//     value: String,
// }

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
        object: WithSpan<ExpandedValue<Index>, Index>,
    },
    MapKey {
        handle: Option<ListHandle<Index>>,
        key: WithSpan<String, Index>,
    },
    MapEntry {
        handle: Option<ListHandle<Index>>,
        key: WithSpan<String, Index>,
        value: WithSpan<ExpandedValue<Index>, Index>,
    }
}

// #[derive(Debug)]
// struct Node<Index: CharIndex> {
//     kind: NodeKind<Index>,
// }


impl<'a, Indexer: CharIndexer> Parser<'a, Indexer> {
    // Only returns None if the first character is \n, #, or EOF.
    fn accept_expr(&mut self, break_chars: &[char]) -> Result<Option<WithSpan<CompactValue<Indexer::Index>, Indexer::Index>>, ()> {
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

                CompactValue::List {
                    items: Vec::new(),
                    item_completion_spans: Vec::new(),
                }
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

                // Value::Map(items)
                return Err(());
            },
            '+' if self.chars.pop_constant("+inf") => {
                CompactValue::Float(f64::INFINITY)
            },
            '-' if self.chars.pop_constant("-inf") => {
                CompactValue::Float(f64::NEG_INFINITY)
            },
            '+' | '-' | '0'..='9' | '.' => {
                let string = self.chars.pop_until(|ch| !break_chars.contains(&ch) && ch != '\n' && ch != '#', |ch| ch == ' ');

                if let Ok(value) = string.parse::<i64>() {
                    CompactValue::Integer(value)
                } else {
                    if let Ok(value) = string.parse::<f64>() {
                        CompactValue::Float(value)
                    } else {
                        self.errors.push(Error::new(ErrorKind::InvalidScalarLiteral, Span(start_marker, self.chars.marker())));
                        return Err(());
                    }
                }
            },
            'n' if self.chars.pop_constant("null") => {
                CompactValue::Null
            },
            't' if self.chars.pop_constant("true") => {
                CompactValue::Bool(true)
            },
            'f' if self.chars.pop_constant("false") => {
                CompactValue::Bool(false)
            },
            'i' if self.chars.pop_constant("inf") => {
                CompactValue::Float(f64::INFINITY)
            },
            'n' if self.chars.pop_constant("nan") => {
                CompactValue::Float(f64::NAN)
            },
            _ => {
                let string = self.chars.pop_until(|ch| !break_chars.contains(&ch) && ch != '\n' && ch != '#', |ch| ch == ' ');
                CompactValue::String(string.to_string())
            },
        };

        Ok(Some(WithSpan {
            span: Span(start_marker, self.chars.marker()),
            value,
        }))
    }

    fn reduce_stack(&mut self, level: usize) -> Option<WithSpan<ExpandedValue<Indexer::Index>, Indexer::Index>> {
        while self.stack.len() > level {
            let item = self.stack.pop().unwrap();

            let object = match item.kind {
                StackItemKind::List { floating_handle_end_marker, items, start_marker, .. } => {
                    match items.last() {
                        Some(item) => {
                            WithSpan {
                                span: Span(start_marker, item.value.span.1),
                                value: ExpandedValue::List {
                                    item_completion_spans: Vec::new(),
                                    items,
                                },
                            }
                        },
                        None => {
                            // The list can only be empty if there is a floating handle.
                            self.errors.push(Error::new(ErrorKind::EmptyExpandedList, Span(start_marker, floating_handle_end_marker.unwrap())));
                            continue;
                        },
                    }
                },
                StackItemKind::Map { entries, floating_key: None, .. } => {
                    WithSpan {
                        span: Span(
                            entries.first().unwrap().key.span.0,
                            entries.last().unwrap().value.span.1,
                        ),
                        value: ExpandedValue::Map {
                            entries,
                            key_completion_spans: Vec::new(),
                            value_completion_spans: Vec::new(),
                        },
                    }
                },
                StackItemKind::Map { entries, floating_key: Some(floating_key), .. } => {
                    self.errors.push(Error::new(ErrorKind::MissingExpandedMapValue, floating_key.span));

                    WithSpan {
                        span: Span(
                            entries.first().and_then(|entry| Some(entry.key.span.0)).unwrap_or(floating_key.span.0),
                            floating_key.span.1,
                        ),
                        value: ExpandedValue::Map {
                            entries,
                            key_completion_spans: Vec::new(),
                            value_completion_spans: Vec::new(),
                        },
                    }
                },
            };

            match self.stack.last_mut().and_then(|item| Some(&mut item.kind)) {
                Some(StackItemKind::List { items, next_item_context, .. }) => {
                    items.push(ExpandedListItem {
                        comment: None,
                        context: next_item_context.take().unwrap(),
                        value: object,
                    });
                },
                Some(StackItemKind::Map { entries, floating_key: key @ Some(_), next_entry_context }) => {
                    entries.push(ExpandedMapEntry {
                        comment: None,
                        context: next_entry_context.take().unwrap(),
                        key: key.take().unwrap(),
                        value: object,
                    });
                },
                None => { return Some(object); },
                _ => todo!(),
            }
        }

        None
    }

    pub fn parse(&mut self) -> Result<WithSpan<ExpandedValue<Indexer::Index>, Indexer::Index>, ()> {
        let mut comments = Vec::new();
        let mut gap = 0;

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
                    if let Some(comment) = self.accept_line_end() {
                        comments.push(StandaloneComment {
                            contents: comment,
                            gap,
                            indent,
                        });

                        gap = 0;
                    } else {
                        gap += 1;
                    }

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
                    let current_item = self.stack.iter().enumerate().find(|(_, item)| item.indent == indent);

                    if let Some((index, _)) = current_item {
                        assert!(self.reduce_stack(index + 1).is_none());
                        false
                    } else {
                        self.errors.push(Error::new(ErrorKind::InvalidIndentSize, Span(line_start_marker, content_start_marker)));
                        self.accept_line_end(); // TODO: Avoid extraneous chars error
                        comments.clear();

                        continue;
                    }
                },
                None if indent == 0 => {
                    true
                },
                _ => {
                    self.errors.push(Error::new(ErrorKind::InvalidIndentSize, Span(line_start_marker, content_start_marker)));
                    self.accept_line_end();
                    comments.clear();

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

            let node = if let Some(key) = self.accept_key() {
                match self.accept_expr(&[]) {
                    // [-] x: y
                    Ok(Some(value)) => {
                        Some(Node::MapEntry {
                            handle,
                            key,
                            value: WithSpan::new(ExpandedValue::Compact(value.value), value.span),
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
                            object: WithSpan::new(ExpandedValue::Compact(item.value), item.span),
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

            let local_comment = self.accept_line_end();
            // let content_comments = std::mem::replace(&mut comments, Vec::new());
            // let content_gap = gap;
            // gap = 0;

            let context = Context {
                comments: std::mem::replace(&mut comments, Vec::new()),
                gap,
                indent,
            };

            gap = 0;

            let node = match node {
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
                        kind: StackItemKind::List {
                            next_item_context: Some(context),
                            floating_handle_end_marker: Some(handle.end_marker),
                            items: Vec::new(),
                            start_marker: content_start_marker,
                        },
                        indent,
                    });
                },

                // - a
                // -
                (Node::ListOpen { handle }, Some(StackItemKind::List { floating_handle_end_marker, next_item_context, .. }), false) => {
                    *floating_handle_end_marker = Some(handle.end_marker);
                    *next_item_context = Some(context);

                    // ??
                    // self.stack.push(StackItem {
                    //     kind: StackItemKind::List {
                    //         next_item_context: None,
                    //         floating_handle_end_marker: Some(handle.end_marker),
                    //         items: Vec::new(),
                    //         start_marker: content_start_marker,
                    //     },
                    //     indent,
                    // });
                },

                // a:
                //   - x
                //
                // [root]
                // - x
                //
                // TODO: Relax to allow unnested
                (Node::ListItem { object, .. }, Some(StackItemKind::Map { floating_key: Some(_), .. }) | None, true) => {
                    self.stack.push(StackItem {
                        kind: StackItemKind::List {
                            floating_handle_end_marker: None,
                            items: vec![ExpandedListItem {
                                comment: local_comment,
                                context,
                                value: object,
                            }],
                            next_item_context: None,
                            start_marker: content_start_marker,
                        },
                        indent,
                    });
                },

                // -
                //   - x
                (Node::ListItem { object, .. }, Some(StackItemKind::List { floating_handle_end_marker: Some(_), .. }), true) => {
                    self.stack.push(StackItem {
                        kind: StackItemKind::List {
                            floating_handle_end_marker: None,
                            items: vec![ExpandedListItem {
                                comment: local_comment,
                                context,
                                value: object,
                            }],
                            next_item_context: None,
                            start_marker: content_start_marker,
                        },
                        indent,
                    });
                },

                // - a
                // - x
                (Node::ListItem { object, .. }, Some(StackItemKind::List { floating_handle_end_marker: None, items, .. }), false) => {
                    items.push(ExpandedListItem {
                        comment: local_comment,
                        context,
                        value: object,
                    });
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
                    let mut optional_context = Some(context);

                    if handle.is_some() {
                        self.stack.push(StackItem {
                            kind: StackItemKind::List {
                                next_item_context: optional_context.take(),
                                floating_handle_end_marker: None,
                                items: Vec::new(),
                                start_marker: content_start_marker,
                            },
                            indent,
                        });
                    }

                    let map_indent = handle
                        .as_ref()
                        .and_then(|handle| Some(handle.item_indent))
                        .unwrap_or(indent);

                    self.stack.push(StackItem {
                        kind: StackItemKind::Map {
                            entries: vec![
                                ExpandedMapEntry {
                                    comment: local_comment,
                                    context: optional_context.unwrap_or(Context::new(
                                        handle
                                            .and_then(|handle| Some(handle.item_indent))
                                            .unwrap_or(indent)
                                    )),
                                    key,
                                    value,
                                }
                            ],
                            floating_key: None,
                            next_entry_context: None,
                        },
                        indent: map_indent,
                    });
                },

                // - a
                // - x: y
                (Node::MapEntry { handle: Some(handle), key, value }, Some(StackItemKind::List { next_item_context, .. }), false) => {
                    *next_item_context = Some(context);

                    self.stack.push(StackItem {
                        kind: StackItemKind::Map {
                            entries: vec![ExpandedMapEntry {
                                comment: local_comment,
                                context: Context::new(handle.item_indent),
                                key,
                                value: WithSpan {
                                    span: value.span,
                                    value: value.value,
                                },
                            }],
                            floating_key: None,
                            next_entry_context: None,
                        },
                        indent,
                    });
                },

                // a: b
                // x: y
                (Node::MapEntry { handle: None, key, value }, Some(StackItemKind::Map { entries, floating_key, .. }), false) => {
                    debug_assert!(floating_key.is_none());

                    entries.push(ExpandedMapEntry {
                        comment: local_comment,
                        context,
                        key: WithSpan {
                            span: key.span,
                            value: key.value,
                        },
                        value: WithSpan {
                            span: value.span,
                            value: value.value,
                        },
                    });
                },

                // a: b
                // x:
                (Node::MapKey { handle: None, key }, Some(StackItemKind::Map { floating_key, next_entry_context, .. }), false) => {
                    debug_assert!(floating_key.is_none());
                    *floating_key = Some(key);
                    *next_entry_context = Some(context);
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
                    let mut optional_context = Some(context);

                    if handle.is_some() {
                        self.stack.push(StackItem {
                            kind: StackItemKind::List {
                                floating_handle_end_marker: None,
                                items: Vec::new(),
                                next_item_context: optional_context.take(),
                                start_marker: content_start_marker,
                            },
                            indent,
                        });
                    }

                    let map_indent = optional_context
                        .as_ref()
                        .and_then(|context| Some(context.indent))
                        .unwrap_or(indent);

                    self.stack.push(StackItem {
                        kind: StackItemKind::Map {
                            entries: Vec::new(),
                            floating_key: Some(key),
                            next_entry_context: optional_context.or(Some(Context::new(
                                handle
                                    .and_then(|handle| Some(handle.item_indent))
                                    .unwrap_or(indent)
                            ))),
                        },
                        indent: map_indent,
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

    fn accept_line_end(&mut self) -> Option<WithSpan<String, Indexer::Index>> {
        self.pop_whitespace();

        match self.chars.peek() {
            Some('\n') => {
                self.chars.advance();
                None
            },
            Some('#') => {
                self.chars.advance();
                self.pop_whitespace();

                let comment_start_marker = self.chars.marker();
                let value = self.chars.pop_while(|ch| ch != '\n').to_string();

                let comment_end_marker = self.chars.marker();

                self.chars.pop();

                Some(WithSpan {
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
    pub object: Option<WithSpan<ExpandedValue<Index>, Index>>,
}

// #[cfg(test)]
// impl<Index: CharIndex> ParseResult<Index> {
//     pub fn json(&self) -> Option<String> {
//         // self.object.as_ref().and_then(|obj| obj.value.json().ok())
//         self.object.and_then(|object| object.value)
//     }
// }

pub fn parse<Indexer: CharIndexer>(input: &str) -> ParseResult<Indexer::Index> {
    let mut parser = Parser::<'_, Indexer>::new(input);
    let object = parser.parse();

    ParseResult {
        errors: parser.errors,
        object: object.ok(),
    }
}
