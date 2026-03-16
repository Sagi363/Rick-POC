use crate::error::{RickError, Result};
use std::collections::HashMap;

/// Minimal JSON value representation.
#[derive(Debug, Clone)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(entries) => {
                for (k, v) in entries {
                    if k == key {
                        return Some(v);
                    }
                }
                None
            }
            _ => None,
        }
    }
}

/// Serialize a JsonValue to a JSON string.
pub fn to_json_string(val: &JsonValue) -> String {
    match val {
        JsonValue::Null => "null".to_string(),
        JsonValue::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        JsonValue::Number(n) => {
            if *n == (*n as i64) as f64 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        JsonValue::String(s) => format!("\"{}\"", escape_json_string(s)),
        JsonValue::Array(items) => {
            let parts: Vec<String> = items.iter().map(|v| to_json_string(v)).collect();
            format!("[{}]", parts.join(","))
        }
        JsonValue::Object(entries) => {
            let parts: Vec<String> = entries
                .iter()
                .map(|(k, v)| format!("\"{}\":{}", escape_json_string(k), to_json_string(v)))
                .collect();
            format!("{{{}}}", parts.join(","))
        }
    }
}

/// Pretty-print JSON with indentation.
pub fn to_json_pretty(val: &JsonValue, indent: usize) -> String {
    let spaces = " ".repeat(indent);
    let inner = " ".repeat(indent + 2);
    match val {
        JsonValue::Null => "null".to_string(),
        JsonValue::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        JsonValue::Number(n) => {
            if *n == (*n as i64) as f64 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        JsonValue::String(s) => format!("\"{}\"", escape_json_string(s)),
        JsonValue::Array(items) => {
            if items.is_empty() {
                return "[]".to_string();
            }
            let parts: Vec<String> = items
                .iter()
                .map(|v| format!("{}{}", inner, to_json_pretty(v, indent + 2)))
                .collect();
            format!("[\n{}\n{}]", parts.join(",\n"), spaces)
        }
        JsonValue::Object(entries) => {
            if entries.is_empty() {
                return "{}".to_string();
            }
            let parts: Vec<String> = entries
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}\"{}\": {}",
                        inner,
                        escape_json_string(k),
                        to_json_pretty(v, indent + 2)
                    )
                })
                .collect();
            format!("{{\n{}\n{}}}", parts.join(",\n"), spaces)
        }
    }
}

fn escape_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

/// Parse a JSON string into a JsonValue.
pub fn parse_json(input: &str) -> Result<JsonValue> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(RickError::Parse("Empty JSON input".to_string()));
    }
    let chars: Vec<char> = trimmed.chars().collect();
    let (val, _) = parse_value(&chars, 0)?;
    Ok(val)
}

fn parse_value(chars: &[char], pos: usize) -> Result<(JsonValue, usize)> {
    let pos = skip_ws(chars, pos);
    if pos >= chars.len() {
        return Err(RickError::Parse("Unexpected end of JSON".to_string()));
    }
    match chars[pos] {
        '"' => parse_string(chars, pos),
        '{' => parse_object(chars, pos),
        '[' => parse_array(chars, pos),
        't' | 'f' => parse_bool(chars, pos),
        'n' => parse_null(chars, pos),
        '-' | '0'..='9' => parse_number(chars, pos),
        c => Err(RickError::Parse(format!("Unexpected char '{}' at {}", c, pos))),
    }
}

fn skip_ws(chars: &[char], mut pos: usize) -> usize {
    while pos < chars.len() && chars[pos].is_ascii_whitespace() {
        pos += 1;
    }
    pos
}

fn parse_string(chars: &[char], pos: usize) -> Result<(JsonValue, usize)> {
    // pos should be at '"'
    let mut i = pos + 1;
    let mut s = String::new();
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            match chars[i + 1] {
                '"' => { s.push('"'); i += 2; }
                '\\' => { s.push('\\'); i += 2; }
                'n' => { s.push('\n'); i += 2; }
                'r' => { s.push('\r'); i += 2; }
                't' => { s.push('\t'); i += 2; }
                '/' => { s.push('/'); i += 2; }
                _ => { s.push(chars[i + 1]); i += 2; }
            }
        } else if chars[i] == '"' {
            return Ok((JsonValue::String(s), i + 1));
        } else {
            s.push(chars[i]);
            i += 1;
        }
    }
    Err(RickError::Parse("Unterminated string".to_string()))
}

fn parse_object(chars: &[char], pos: usize) -> Result<(JsonValue, usize)> {
    let mut i = skip_ws(chars, pos + 1);
    let mut entries = Vec::new();
    if i < chars.len() && chars[i] == '}' {
        return Ok((JsonValue::Object(entries), i + 1));
    }
    loop {
        i = skip_ws(chars, i);
        let (key_val, new_i) = parse_string(chars, i)?;
        let key = match key_val {
            JsonValue::String(s) => s,
            _ => return Err(RickError::Parse("Expected string key".to_string())),
        };
        i = skip_ws(chars, new_i);
        if i >= chars.len() || chars[i] != ':' {
            return Err(RickError::Parse("Expected ':'".to_string()));
        }
        i = skip_ws(chars, i + 1);
        let (val, new_i) = parse_value(chars, i)?;
        entries.push((key, val));
        i = skip_ws(chars, new_i);
        if i < chars.len() && chars[i] == ',' {
            i += 1;
        } else if i < chars.len() && chars[i] == '}' {
            return Ok((JsonValue::Object(entries), i + 1));
        } else {
            return Err(RickError::Parse("Expected ',' or '}'".to_string()));
        }
    }
}

fn parse_array(chars: &[char], pos: usize) -> Result<(JsonValue, usize)> {
    let mut i = skip_ws(chars, pos + 1);
    let mut items = Vec::new();
    if i < chars.len() && chars[i] == ']' {
        return Ok((JsonValue::Array(items), i + 1));
    }
    loop {
        let (val, new_i) = parse_value(chars, i)?;
        items.push(val);
        i = skip_ws(chars, new_i);
        if i < chars.len() && chars[i] == ',' {
            i += 1;
        } else if i < chars.len() && chars[i] == ']' {
            return Ok((JsonValue::Array(items), i + 1));
        } else {
            return Err(RickError::Parse("Expected ',' or ']'".to_string()));
        }
    }
}

fn parse_bool(chars: &[char], pos: usize) -> Result<(JsonValue, usize)> {
    if chars[pos..].starts_with(&['t', 'r', 'u', 'e']) {
        Ok((JsonValue::Bool(true), pos + 4))
    } else if chars[pos..].starts_with(&['f', 'a', 'l', 's', 'e']) {
        Ok((JsonValue::Bool(false), pos + 5))
    } else {
        Err(RickError::Parse("Invalid bool".to_string()))
    }
}

fn parse_null(chars: &[char], pos: usize) -> Result<(JsonValue, usize)> {
    if chars[pos..].starts_with(&['n', 'u', 'l', 'l']) {
        Ok((JsonValue::Null, pos + 4))
    } else {
        Err(RickError::Parse("Invalid null".to_string()))
    }
}

fn parse_number(chars: &[char], pos: usize) -> Result<(JsonValue, usize)> {
    let mut i = pos;
    if i < chars.len() && chars[i] == '-' {
        i += 1;
    }
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    if i < chars.len() && chars[i] == '.' {
        i += 1;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
    }
    let num_str: String = chars[pos..i].iter().collect();
    let n: f64 = num_str
        .parse()
        .map_err(|_| RickError::Parse(format!("Invalid number: {}", num_str)))?;
    Ok((JsonValue::Number(n), i))
}
