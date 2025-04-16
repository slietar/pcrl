use std::collections::HashMap;
use crate::iterator::CharIndex;
use crate::span::{Span, WithSpan};


#[derive(Debug)]
pub struct MultilineStringLine<Index: CharIndex> {
    pub comment: Option<String>,
    pub context: Context<Index>,
    pub text: String,
}


#[derive(Debug)]
pub struct StandaloneComment<Index: CharIndex> {
    pub contents: WithSpan<String, Index>,
    pub indent: usize,
    pub gap: usize,
}


#[derive(Debug)]
pub struct Context<Index: CharIndex> {
    pub comments: Vec<StandaloneComment<Index>>,
    pub gap: usize,
    pub indent: usize,
}

impl<Index: CharIndex> Context<Index> {
    pub fn new(indent: usize) -> Self {
        Self {
            comments: Vec::new(),
            gap: 0,
            indent,
        }
    }
}

#[derive(Debug)]
pub struct ExpandedListItem<Index: CharIndex> {
    pub comment: Option<WithSpan<String, Index>>,
    pub context: Context<Index>,
    pub value: WithSpan<ExpandedValue<Index>, Index>,
}

#[derive(Debug)]
pub struct ExpandedMapEntry<Index: CharIndex> {
    pub comment: Option<WithSpan<String, Index>>,
    pub context: Context<Index>,
    pub key: WithSpan<String, Index>,
    pub value: WithSpan<ExpandedValue<Index>, Index>,
}

#[derive(Debug)]
pub struct CompactMapEntry<Index: CharIndex> {
    pub key: WithSpan<String, Index>,
    pub value: WithSpan<CompactValue<Index>, Index>,
}


// #[derive(Debug)]
// pub enum CompactOrExpandedValue<Index: CharIndex> {
//     Compact(CompactValue<Index>),
//     Expanded(ExpandedValue<Index>),
// }


#[derive(Debug)]
pub enum ExpandedValue<Index: CharIndex> {
    Compact(CompactValue<Index>),
    List {
        items: Vec<ExpandedListItem<Index>>,
        item_completion_spans: Vec<Span<Index>>,
    },
    Map {
        entries: Vec<ExpandedMapEntry<Index>>,
        key_completion_spans: Vec<Span<Index>>,
        value_completion_spans: Vec<Span<Index>>,
    },
    String {
        lines: Vec<MultilineStringLine<Index>>,
        string: String,
    },
}

impl<Index: CharIndex> std::convert::From<ExpandedValue<Index>> for RegularValue {
    fn from(value: ExpandedValue<Index>) -> Self {
        use ExpandedValue::*;

        match value {
            Compact(value) =>
                value.into(),
            List { items, .. } =>
                RegularValue::List(
                    items
                        .into_iter()
                        .map(|item| item.value.value.into())
                        .collect()
                ),
            Map { entries, .. } =>
                RegularValue::Map(
                    entries
                        .into_iter()
                        .map(|entry| (entry.key.value, entry.value.value.into()))
                        .collect()
                ),
            String { string, .. } =>
                RegularValue::String(string),
        }
    }
}


#[derive(Debug)]
pub enum CompactValue<Index: CharIndex> {
    Bool(bool),
    Float(f64),
    Integer(i64),
    List {
        items: Vec<WithSpan<CompactValue<Index>, Index>>,
        item_completion_spans: Vec<Span<Index>>,
    },
    Map {
        entries: Vec<CompactMapEntry<Index>>,
        key_completion_spans: Vec<Span<Index>>,
        value_completion_spans: Vec<Span<Index>>,
    },
    Null,
    String(String),
}

impl<Index: CharIndex> std::convert::From<CompactValue<Index>> for RegularValue {
    fn from(value: CompactValue<Index>) -> Self {
        use CompactValue::*;

        match value {
            Bool(value) =>
                RegularValue::Bool(value),
            Float(value) =>
                RegularValue::Float(value),
            Integer(value) =>
                RegularValue::Integer(value),
            List { items, .. } =>
                RegularValue::List(
                    items
                        .into_iter()
                        .map(|item| item.value.into())
                        .collect()
                ),
            Map { entries, .. } =>
                RegularValue::Map(
                    entries
                        .into_iter()
                        .map(|entry| (entry.key.value, entry.value.value.into()))
                        .collect()
                ),
            Null =>
                RegularValue::Null,
            String(value) =>
                RegularValue::String(value),
        }
    }
}


#[derive(Debug)]
pub enum RegularValue {
    Bool(bool),
    Float(f64),
    Integer(i64),
    List(Vec<RegularValue>),
    Map(HashMap<String, RegularValue>),
    Null,
    String(String),
}

impl std::convert::From<RegularValue> for serde_json::Value {
    fn from(value: RegularValue) -> Self {
        use RegularValue::*;

        match value {
            Bool(value) =>
                Self::Bool(value),
            Float(value) =>
                Self::Number(serde_json::Number::from_f64(value).unwrap()),
            Integer(value) =>
                Self::Number(value.into()),
            List(items) =>
                Self::Array(
                    items
                        .into_iter()
                        .map(Self::from)
                        .collect()
                ),
            Map(entries) =>
                Self::Object(
                    entries
                        .into_iter()
                        .map(|(key, value)| (key, Self::from(value)))
                        .collect()
                ),
            Null => Self::Null,
            String(value) =>
                Self::String(value),
        }
    }
}
