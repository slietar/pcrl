use crate::parser::*;
use crate::iterator::CharIndex;


#[derive(Debug)]
pub enum FindResult<'a, Index: CharIndex> {
    MapKey {
        entry: &'a MapEntry<Index>,
    },
    Value {
        object: &'a Object<Index>,
    },
}

pub fn find<'a, Index: CharIndex>(result: &'a ParseResult<Index>, index: Index) -> Option<FindResult<'a, Index>> {
    // match result.object
    let mut current_object = result.object.as_ref().unwrap();

    if !current_object.span.contains_index(index) {
        return None;
    }

    loop {
        match &current_object.value {
            Value::Map { entries, .. } => {
                for entry in entries {
                    if entry.key.span.contains_index(index) {
                        return Some(FindResult::MapKey {
                            entry,
                        });
                    }

                    if entry.value.span.contains_index(index) {
                        current_object = &entry.value;
                        continue;
                    }
                }
            },
            Value::Integer(_) | Value::Float(_) | Value::String(_) | Value::Bool(_) | Value::Null => {
                return Some(FindResult::Value {
                    object: current_object,
                });
            },
            _ => (),
        }

        return None;
    }
}
