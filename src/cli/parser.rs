pub fn split<'a>(input: &'a str) -> Vec<&'a str> {
    let mut tokens: Vec<&str> = Vec::new();
    let mut start: Option<usize> = None;
    let mut in_quotes = false;

    for (i, c) in input.char_indices() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                if start.is_none() {
                    start = Some(i);
                } else {
                    tokens.push(&input[start.unwrap() + 1..i]);
                    start = None;
                }
            }
            ' ' if !in_quotes => {
                if start.is_some() {
                    tokens.push(&input[start.unwrap()..i]);
                    start = None;
                }
            }
            _ => {
                if start.is_none() {
                    start = Some(i);
                }
            }
        }
    }

    if start.is_some() {
        tokens.push(&input[start.unwrap()..input.len()]);
    }

    // Assume no end quote means the string continues on to the end

    tokens
}
