use crate::iterator::{CharIndex, CharIterator, Marker};


#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Span<Index: CharIndex>(pub Marker<Index>, pub Marker<Index>);

impl<Index: CharIndex> Span<Index> {
    pub fn point(marker: &Marker<Index>) -> Self {
        Self(*marker, *marker)
    }

    pub fn contains_index(&self, index: Index) -> bool {
        (index >= self.0.index) && (index < self.1.index)
    }

    #[cfg(feature = "format")]
    pub fn format(&self, contents: &str, output: &mut dyn std::io::Write) -> std::io::Result<()> {
        use unicode_segmentation::UnicodeSegmentation;

        let mut iterator: CharIterator<'_, crate::indexers::CharacterLineColumn> = CharIterator::new(contents);
        let mut current_line_start_marker = iterator.marker();

        while iterator.byte_offset < self.0.byte_offset {
            if iterator.pop().unwrap() == '\n' {
                current_line_start_marker = iterator.marker();
            }
        }

        let span_start_marker = iterator.marker();
        let mut line_markers = Vec::new();
        let mut span_end_marker = span_start_marker;
        let mut extend_last_line = false;

        while iterator.byte_offset < self.1.byte_offset {
            match iterator.peek().unwrap() {
                '\n' => {
                    let current_marker = iterator.marker();
                    line_markers.push((current_line_start_marker, current_marker));

                    iterator.advance();

                    if iterator.byte_offset == self.1.byte_offset {
                        extend_last_line = true;
                        span_end_marker = current_marker;
                    }

                    current_line_start_marker = iterator.marker();
                },
                _ => {
                    iterator.advance();

                    if iterator.byte_offset == self.1.byte_offset {
                        extend_last_line = false;
                        span_end_marker = iterator.marker();
                    }
                },
            }
        }

        if !extend_last_line || line_markers.is_empty() {
            loop {
                match iterator.peek() {
                    Some('\n') | None => {
                        line_markers.push((current_line_start_marker, iterator.marker()));
                        break;
                    },
                    Some(_) => {
                        iterator.advance();
                    },
                }
            }
        }

        let last_line_number_fmt = format!("{}", span_end_marker.index.line + 1);

        // Using .len() is ok because digits are all ASCII.
        let line_number_width = last_line_number_fmt.len();

        if line_markers.len() == 1 {
            let (line_start_marker, line_end_marker) = line_markers[0];

            output.write_fmt(format_args!("{} | ", last_line_number_fmt))?;

            output.write(&contents[line_start_marker.byte_offset..line_end_marker.byte_offset].as_bytes())?;
            output.write(&['\n' as u8])?;

            output.write(&[' ' as u8].repeat(line_number_width))?;
            output.write(" | ".as_bytes())?;

            let whitespace_width = UnicodeSegmentation::graphemes(&contents[line_start_marker.byte_offset..span_start_marker.byte_offset], true).count();
            output.write(&[' ' as u8].repeat(whitespace_width))?;

            let span_width = span_end_marker.index.column - span_start_marker.index.column;

            if span_width > 0 {
                let highlight_width = UnicodeSegmentation::graphemes(&contents[span_start_marker.byte_offset..span_end_marker.byte_offset], true).count();
                output.write(&['^' as u8].repeat(highlight_width))?;

                if extend_last_line {
                    output.write(&['-' as u8])?;
                }
            } else {
                output.write(&['~' as u8])?;
            }

            output.write(&['\n' as u8])?;
        } else {
            for (relative_line_number, (line_start_marker, line_end_marker)) in line_markers.iter().enumerate() {
                let line_number = span_start_marker.index.line + relative_line_number;

                output.write_fmt(format_args!("{: >width$} | ", line_number + 1, width = line_number_width))?;

                output.write(&contents[line_start_marker.byte_offset..line_end_marker.byte_offset].as_bytes())?;
                output.write(&['\n' as u8])?;

                output.write(&[' ' as u8].repeat(line_number_width))?;
                output.write(" | ".as_bytes())?;

                if relative_line_number == 0 {
                    output.write(&[' ' as u8].repeat(span_start_marker.index.column))?;
                    output.write(&['^' as u8].repeat(line_end_marker.index.column - span_start_marker.index.column))?;
                    output.write(&['-' as u8])?;
                } else if relative_line_number == line_markers.len() - 1 {
                    output.write(&['^' as u8].repeat(span_end_marker.index.column))?;

                    if extend_last_line {
                        output.write(&['-' as u8])?;
                    }
                } else {
                    output.write(&['^' as u8].repeat(line_end_marker.index.column))?;
                    output.write(&['-' as u8])?;
                }

                output.write(&['\n' as u8])?;
            }
        }

        Ok(())
    }
}


#[derive(Debug)]
pub struct WithSpan<T, Index: CharIndex> {
    pub span: Span<Index>,
    pub value: T,
}

impl<T, Index: CharIndex> WithSpan<T, Index> {
    pub fn new(value: T, span: Span<Index>) -> Self {
        Self {
            span,
            value,
        }
    }
}
