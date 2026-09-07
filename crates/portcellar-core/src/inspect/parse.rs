use std::collections::BTreeMap;

pub(crate) fn parse_key_values(text: &str) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();
    for line in text.lines() {
        let tokens = quoted_tokens(line);
        if tokens.len() >= 2 {
            values.insert(tokens[0].clone(), tokens[1].clone());
        }
    }
    values
}

pub(crate) fn values_for_key(text: &str, key: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let tokens = quoted_tokens(line);
            (tokens.len() >= 2 && tokens[0] == key).then(|| tokens[1].clone())
        })
        .collect()
}

pub(crate) fn quoted_tokens(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '"' {
            continue;
        }

        let mut token = String::new();
        while let Some(ch) = chars.next() {
            match ch {
                '"' => break,
                '\\' => {
                    if let Some(next) = chars.next() {
                        token.push(next);
                    }
                }
                _ => token.push(ch),
            }
        }
        tokens.push(token);
    }

    tokens
}
