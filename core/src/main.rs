fn main() -> Result<(), ()> {
    let result = pcrl::parse::<pcrl::counters::Character>("
-
    - z
    -
        - w # foo
    - { a: b }
    - [5, 6]
");

    eprintln!("Result: {:#?}", result.object);
    eprintln!("Errors: {:#?}", result.errors);
    // eprintln!("{:#?}", &parser.contents[parser.offset..]);

    // if let Some(result) = result.object {
    //     eprintln!("JSON: {}", result.value.json());
    // }

    Ok(())
}
