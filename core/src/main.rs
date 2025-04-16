#[cfg(feature = "format")]
fn main() -> Result<(), ()> {
    let input = "
a:
  - b: c
";

    // use std::time::Instant;
    // let now = Instant::now();

    // for i in 0..1000 {
    //   let result = pcrl::parse::<pcrl::indexers::Character>(input);
    // }

    // let elapsed = now.elapsed();
    // println!("Elapsed: {:.2?}", (elapsed.as_micros() as f64) / 1000.0f64);

    let result = pcrl::parse::<pcrl::indexers::Character>(input);

    let x = result.object.as_ref().unwrap();
    x.span.format(input, &mut std::io::stdout()).unwrap();
    println!("");

    match &x.value {
/*       pcrl::ExpandedValue::List { items, .. } => {
        for item in items.iter() {
          item.value.span.format(input, &mut std::io::stdout()).unwrap();
          println!("");
        }
      }, */
      pcrl::ExpandedValue::Map { entries, .. } => {
        for entry in entries.iter() {
          for comment in &entry.context.comments {
            println!("Comment: {:?}", comment.contents.value);
            comment.contents.span.format(input, &mut std::io::stdout()).unwrap();
            println!("");
          }

          if let Some(comment) = &entry.comment {
            println!("Local comment: {:?}", comment.value);
            comment.span.format(input, &mut std::io::stdout()).unwrap();
            println!("");
          }

          entry.key.span.format(input, &mut std::io::stdout()).unwrap();
          println!("");
          entry.value.span.format(input, &mut std::io::stdout()).unwrap();
          println!("");
        }
      },
      _ => (),
    }

    eprintln!("Result: {:#?}", result.object);

    let regular_value: pcrl::RegularValue = result.object.unwrap().value.into();
    let json_value: serde_json::Value = regular_value.into();

    eprintln!("{}", serde_json::to_string(&json_value).unwrap());


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
