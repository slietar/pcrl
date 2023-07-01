#[cfg(feature = "format")]
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

x:
  - b: d
    a:
      - p

u: v
";

    let result = pcrl::parse::<pcrl::indexers::Character>(input);

    // eprintln!("Result: {:#?}", result.object);
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


#[cfg(not(feature = "format"))]
fn main() {

}
