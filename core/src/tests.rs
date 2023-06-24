#![cfg(test)]

use super::{counters, parse};


#[test]
fn entries() {
    let entries = [
        ("- a", r#"["a"]"#),
        ("- a\n-  b\n-c", r#"["a", "b", "c"]"#),
        ("-\n  - a\n", r#"[["a"]]"#),

        ("- [3, 4, 5]", r#"[[3, 4, 5]]"#),
        ("- [3, 4, 5, ]", r#"[[3, 4, 5]]"#),
        ("- [a, b ]", r#"[["a", "b"]]"#),
    ];

    for (input, expected) in &entries {
        let result = parse::<counters::Empty>(input);

        assert!(result.errors.is_empty());
        assert_eq!(&result.json().unwrap(), expected);
    }
}

// #[test]
// fn list1() {
//     let result = parse::<counters::Empty>("- a");

//     // eprintln!("{:#?}", result);
//     assert!(result.errors.is_empty());
//     assert_eq!(&result.json().unwrap(), r#"["a"]"#);
// }

// #[test]
// fn list2() {
//     let result = parse::<counters::Empty>("- a\n-  b\n-c");

//     assert!(result.errors.is_empty());
//     assert_eq!(&result.json().unwrap(), r#"["a", "b", "c"]"#);
// }

// #[test]
// fn list3() {
//     let result = parse::<counters::Empty>("-\n  - a\n");

//     assert!(result.errors.is_empty());
//     assert_eq!(&result.json().unwrap(), r#"["a", "b", "c"]"#);
// }
