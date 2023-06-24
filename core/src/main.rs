use pcrl::CharCounter;


fn main() -> Result<(), ()> {
//     let input = "foo
// bar
// baz";

    // let span = pcrl::Span(
    //     pcrl::CharIteratorMarker {
    //         byte_offset: 4,
    //         counter: pcrl::counters::Empty::new(),
    //     },
    //     pcrl::CharIteratorMarker {
    //         byte_offset: 5,
    //         counter: pcrl::counters::Empty::new(),
    //     },
    // );

    // span.format(input, &mut std::io::stdout()).unwrap();

    let input = "
a: b
c:
    d: e
    f: g
    # d: e
";

    let result = pcrl::parse::<pcrl::counters::Character>(input);

    eprintln!("Result: {:#?}", result.object);
    // eprintln!("Errors: {:#?}", result.errors);

    for error in result.errors {
        eprintln!("Error: {:#?}", error.value);
        error.span.format(input, &mut std::io::stdout()).unwrap();
    }

    // result.object.unwrap().span.format(input, &mut std::io::stdout()).unwrap();

    // if let Some(result) = result.object {
    //     eprintln!("JSON: {}", result.value.json());
    // }

    Ok(())
}
