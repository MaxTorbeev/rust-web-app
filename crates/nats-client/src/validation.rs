pub(crate) fn is_valid_entity_name(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|character| {
            !character.is_ascii_whitespace() && !matches!(character, b'.' | b'*' | b'>')
        })
}

pub(crate) fn is_valid_publish_subject(subject: &str) -> bool {
    is_valid_subject(subject, false)
}

pub(crate) fn is_valid_subject_filter(subject: &str) -> bool {
    is_valid_subject(subject, true)
}

fn is_valid_subject(subject: &str, allow_wildcards: bool) -> bool {
    if subject.is_empty() {
        return false;
    }

    let mut tokens = subject.split('.').peekable();

    while let Some(token) = tokens.next() {
        if token.is_empty() {
            return false;
        }

        if allow_wildcards && token == ">" {
            return tokens.peek().is_none();
        }

        if allow_wildcards && token == "*" {
            continue;
        }

        if token
            .bytes()
            .any(|character| character.is_ascii_whitespace() || matches!(character, b'*' | b'>'))
        {
            return false;
        }
    }

    true
}
