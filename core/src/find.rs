use crate::parser::*;
use crate::iterator::CharIndex;
use crate::result::*;
use crate::span::WithSpan;


#[derive(Debug)]
pub enum FindResult<'a, Index: CharIndex> {
    MapKey {
        entry: &'a ExpandedMapEntry<Index>,
        path: FindPath<'a>,
    },
    Value {
        object: &'a WithSpan<ExpandedValue<Index>, Index>,
        path: FindPath<'a>,
    },
}


#[derive(Debug)]
pub enum FindPathItem<'a> {
    ListIndex(usize),
    MapKey(&'a str),
}

type FindPath<'a> = Vec<FindPathItem<'a>>;


pub fn find<'a, Index: CharIndex>(result: &'a ParseResult<Index>, index: Index, include_end: bool) -> Option<FindResult<'a, Index>> {
    let mut current_object = result.object.as_ref().unwrap();
    let mut path = FindPath::new();

    if !current_object.span.contains_index(index, include_end) {
        return None;
    }

    'b: loop {
        match &current_object.value {
            ExpandedValue::List { items, .. } => {
                for (item_index, item) in items.iter().enumerate() {
                    if item.value.span.contains_index(index, include_end) {
                        current_object = &item.value;
                        path.push(FindPathItem::ListIndex(item_index));
                        continue 'b;
                    }
                }
            },
            ExpandedValue::Map { entries, .. } => {
                for entry in entries {
                    if entry.key.span.contains_index(index, include_end) {
                        return Some(FindResult::MapKey {
                            entry,
                            path,
                        });
                    }

                    if entry.value.span.contains_index(index, include_end) {
                        current_object = &entry.value;
                        path.push(FindPathItem::MapKey(&entry.key.value));

                        continue 'b;
                    }
                }
            },
            ExpandedValue::Compact(CompactValue::Integer(_) | CompactValue::Float(_) | CompactValue::String(_) | CompactValue::Bool(_) | CompactValue::Null) => {
                return Some(FindResult::Value {
                    object: current_object,
                    path,
                });
            },
            _ => todo!(),
        }

        return None;
    }
}
