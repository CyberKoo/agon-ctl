use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityParseError {
    message: String,
}

impl CapabilityParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CapabilityParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid capabilities string: {}", self.message)
    }
}

impl Error for CapabilityParseError {}

#[derive(Debug)]
enum Node<'a> {
    Atom(&'a str),
    List(Vec<Node<'a>>),
}

pub(super) fn parse(raw: &str) -> Result<BTreeMap<u32, Vec<u8>>, CapabilityParseError> {
    let nodes = Parser::new(raw).parse()?;
    let mut values = BTreeMap::new();
    if let Some(entries) = find_section(&nodes, "vcp") {
        parse_entries(entries, None, &mut values);
    }
    Ok(values)
}

fn find_section<'a, 'b>(nodes: &'b [Node<'a>], name: &str) -> Option<&'b [Node<'a>]> {
    for node in nodes {
        let Node::List(items) = node else {
            continue;
        };
        for (index, item) in items.iter().enumerate() {
            if atom(item).is_some_and(|atom| atom.eq_ignore_ascii_case(name)) {
                if let Some(Node::List(entries)) = items.get(index + 1) {
                    return Some(entries);
                }
            }
        }
        if let Some(found) = find_section(items, name) {
            return Some(found);
        }
    }
    None
}

fn parse_entries(
    entries: &[Node<'_>],
    prefix: Option<[u8; 2]>,
    output: &mut BTreeMap<u32, Vec<u8>>,
) {
    let mut index = 0;
    while index < entries.len() {
        let Some(name) = atom(&entries[index]) else {
            index += 1;
            continue;
        };
        let child_list = entries.get(index + 1).and_then(as_list);
        if prefix.is_none() {
            if let Some(children) = child_list {
                if parse_namespace(name, children, output) {
                    index += 2;
                    continue;
                }
            }
        }
        let Some(key) = capability_key(name, prefix) else {
            index += 1;
            continue;
        };
        let listed = output.entry(key).or_default();
        if let Some(children) = child_list {
            collect_values(children, listed);
            index += 2;
        } else {
            index += 1;
        }
    }
}

fn capability_key(name: &str, prefix: Option<[u8; 2]>) -> Option<u32> {
    match prefix {
        Some(prefix) => {
            hex_byte(name).map(|code| u32::from_be_bytes([0, prefix[0], prefix[1], code]))
        }
        None if name.len() == 6 && name.is_ascii() => u32::from_str_radix(name, 16).ok(),
        None => hex_byte(name).map(u32::from),
    }
}

fn parse_namespace(name: &str, entries: &[Node<'_>], output: &mut BTreeMap<u32, Vec<u8>>) -> bool {
    let (prefix, combined) = match name {
        name if name.eq_ignore_ascii_case("E2") => ([0xE2, 0xA0], false),
        name if name.eq_ignore_ascii_case("E2A0") => ([0xE2, 0xA0], true),
        name if name.eq_ignore_ascii_case("F8") => ([0xF8, 0xE1], false),
        name if name.eq_ignore_ascii_case("F8E1") => ([0xF8, 0xE1], true),
        _ => return false,
    };
    if combined {
        parse_entries(entries, Some(prefix), output);
        return true;
    }
    let second = if prefix[0] == 0xE2 { "A0" } else { "E1" };
    if entries
        .first()
        .and_then(atom)
        .is_none_or(|atom| !atom.eq_ignore_ascii_case(second))
    {
        return false;
    }
    match entries.get(1).and_then(as_list) {
        Some(children) => parse_entries(children, Some(prefix), output),
        None => parse_entries(&entries[1..], Some(prefix), output),
    }
    true
}

fn collect_values(entries: &[Node<'_>], output: &mut Vec<u8>) {
    if is_max_value(entries) {
        return;
    }
    let mut index = 0;
    while index < entries.len() {
        match &entries[index] {
            Node::Atom(word)
                if word.eq_ignore_ascii_case("max")
                    && entries
                        .get(index + 1)
                        .and_then(atom)
                        .is_some_and(|next| next.eq_ignore_ascii_case("val")) =>
            {
                index += 3;
            }
            Node::Atom(value) => {
                if let Some(value) = hex_byte(value) {
                    output.push(value);
                }
                index += 1;
            }
            Node::List(children) => {
                collect_values(children, output);
                index += 1;
            }
        }
    }
}

fn is_max_value(entries: &[Node<'_>]) -> bool {
    entries
        .first()
        .and_then(atom)
        .is_some_and(|word| word.eq_ignore_ascii_case("max"))
        && entries
            .get(1)
            .and_then(atom)
            .is_some_and(|word| word.eq_ignore_ascii_case("val"))
}

fn atom<'a>(node: &Node<'a>) -> Option<&'a str> {
    match node {
        Node::Atom(atom) => Some(atom),
        Node::List(_) => None,
    }
}

fn as_list<'a, 'b>(node: &'b Node<'a>) -> Option<&'b [Node<'a>]> {
    match node {
        Node::Atom(_) => None,
        Node::List(items) => Some(items),
    }
}

fn hex_byte(input: &str) -> Option<u8> {
    if input.is_empty() || input.len() > 2 || !input.is_ascii() {
        return None;
    }
    u8::from_str_radix(input, 16).ok()
}

struct Parser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    fn parse(mut self) -> Result<Vec<Node<'a>>, CapabilityParseError> {
        let nodes = self.nodes(false, 0)?;
        if self.position != self.input.len() {
            return Err(self.error("unexpected closing parenthesis"));
        }
        Ok(nodes)
    }

    fn nodes(&mut self, nested: bool, depth: usize) -> Result<Vec<Node<'a>>, CapabilityParseError> {
        if depth > 16 {
            return Err(self.error("parenthesis nesting is too deep"));
        }
        let mut nodes = Vec::new();
        loop {
            self.skip_whitespace();
            let Some(byte) = self.peek() else {
                if nested {
                    return Err(self.error("unclosed parenthesis"));
                }
                return Ok(nodes);
            };
            match byte {
                b'(' => {
                    self.position += 1;
                    nodes.push(Node::List(self.nodes(true, depth + 1)?));
                }
                b')' => {
                    if !nested {
                        return Ok(nodes);
                    }
                    self.position += 1;
                    return Ok(nodes);
                }
                _ => nodes.push(Node::Atom(self.atom())),
            }
        }
    }

    fn atom(&mut self) -> &'a str {
        let start = self.position;
        while let Some(byte) = self.peek() {
            if byte.is_ascii_whitespace() || matches!(byte, b'(' | b')') {
                break;
            }
            self.position += 1;
        }
        &self.input[start..self.position]
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(|byte| byte.is_ascii_whitespace()) {
            self.position += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.as_bytes().get(self.position).copied()
    }

    fn error(&self, message: &str) -> CapabilityParseError {
        CapabilityParseError::new(format!("{message} at byte {}", self.position))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_and_namespaced_explicit_value_lists() {
        let raw = "(prot(monitor)vcp(10 14(00 01 05) 86(01 02 05) \
                   E2(A0(20(00 02 04) 40(01))) F8(E1(12(03 01 00 02 04 05)))))";
        let values = parse(raw).unwrap();
        assert_eq!(values.get(&0x14).unwrap(), &[0, 1, 5]);
        assert_eq!(values.get(&0x86).unwrap(), &[1, 2, 5]);
        assert_eq!(values.get(&0xE2A020).unwrap(), &[0, 2, 4]);
        assert_eq!(values.get(&0xE2A040).unwrap(), &[1]);
        assert_eq!(values.get(&0xF8E112).unwrap(), &[3, 1, 0, 2, 4, 5]);
        assert!(values.contains_key(&0x10));
    }

    #[test]
    fn accepts_namespace_codes_without_an_extra_parenthesis_level() {
        let values = parse("(vcp(E2(A0 20(00 02 04))F8(E1 51(00 02))))").unwrap();
        assert_eq!(values.get(&0xE2A020).unwrap(), &[0, 2, 4]);
        assert_eq!(values.get(&0xF8E151).unwrap(), &[0, 2]);
    }

    #[test]
    fn parses_flat_six_digit_namespaced_codes() {
        let values =
            parse("(vcp(E2A020(00 02 04) E2A04B F8E151(00 01 02 03) F8E112(00 01 02 03 04)))")
                .unwrap();
        assert_eq!(values.get(&0xE2A020).unwrap(), &[0, 2, 4]);
        assert!(values.get(&0xE2A04B).unwrap().is_empty());
        assert_eq!(values.get(&0xF8E151).unwrap(), &[0, 1, 2, 3]);
        assert_eq!(values.get(&0xF8E112).unwrap(), &[0, 1, 2, 3, 4]);
    }

    #[test]
    fn max_val_is_not_an_explicitly_supported_value() {
        let values = parse("(vcp(E2(A0(20(00 02 04)(max val 8A) 40(00 01 max val 01)))))").unwrap();
        assert_eq!(values.get(&0xE2A020).unwrap(), &[0, 2, 4]);
        assert_eq!(values.get(&0xE2A040).unwrap(), &[0, 1]);
    }

    #[test]
    fn duplicate_values_and_their_order_are_preserved() {
        let values = parse("(vcp(86(01 01 02) 86(02 05)))").unwrap();
        assert_eq!(values.get(&0x86).unwrap(), &[1, 1, 2, 2, 5]);
    }

    #[test]
    fn malformed_parentheses_are_rejected() {
        assert!(parse("(vcp(86(01 02))").is_err());
        assert!(parse("vcp(86(01 02))))").is_err());
    }
}
