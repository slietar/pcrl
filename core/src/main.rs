use pcrl::CharCounter;


fn main() -> Result<(), ()> {
    let input = "foo
bar
baz";
    // let iterator = pcrl::CharIterator::new(input);
    let span = pcrl::Span(
        pcrl::CharIteratorMarker {
            byte_offset: 0,
            counter: pcrl::counters::Empty::new(),
        },
        pcrl::CharIteratorMarker {
            byte_offset: 9,
            counter: pcrl::counters::Empty::new(),
        },
    );

    span.format(input, &mut std::io::stdout()).unwrap();

    // let result = pcrl::parse::<pcrl::counters::Character>(input);

    // eprintln!("Result: {:#?}", result.object);
    // eprintln!("Errors: {:#?}", result.errors);

    // result.object.unwrap().span.format(input, &mut std::io::stdout()).unwrap();

    // if let Some(result) = result.object {
    //     eprintln!("JSON: {}", result.value.json());
    // }

    Ok(())
}
