use crate::error::{RickError, Result};
use std::collections::HashMap;

/// A simple YAML value representation.
#[derive(Debug, Clone)]
pub enum YamlValue {
    String(String),
    Bool(bool),
    List(Vec<YamlValue>),
    Map(Vec<(String, YamlValue)>),
    Null,
}

impl YamlValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            YamlValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            YamlValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&Vec<YamlValue>> {
        match self {
            YamlValue::List(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&Vec<(String, YamlValue)>> {
        match self {
            YamlValue::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&YamlValue> {
        match self {
            YamlValue::Map(entries) => {
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

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|v| v.as_str())
    }
}

/// Parse a simple YAML string into a YamlValue.
/// Supports: scalars, maps, lists of maps (with `- key: val` syntax).
pub fn parse_yaml(input: &str) -> Result<YamlValue> {
    let lines: Vec<&str> = input.lines().collect();
    if lines.is_empty() {
        return Ok(YamlValue::Null);
    }
    let (val, _) = parse_block(&lines, 0, 0)?;
    Ok(val)
}

fn indent_level(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn parse_block(lines: &[&str], start: usize, base_indent: usize) -> Result<(YamlValue, usize)> {
    let mut entries: Vec<(String, YamlValue)> = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            i += 1;
            continue;
        }

        let indent = indent_level(line);
        if indent < base_indent {
            break;
        }

        // Handle list item
        if trimmed.starts_with("- ") {
            // This is a list at the current level — collect all items
            let mut list_items: Vec<YamlValue> = Vec::new();
            while i < lines.len() {
                let line2 = lines[i];
                let trimmed2 = line2.trim();
                if trimmed2.is_empty() || trimmed2.starts_with('#') {
                    i += 1;
                    continue;
                }
                let indent2 = indent_level(line2);
                if indent2 < base_indent {
                    break;
                }
                if !trimmed2.starts_with("- ") || indent2 != indent {
                    break;
                }
                // Parse the item after "- "
                let item_content = &trimmed2[2..];
                if let Some((key, val)) = parse_kv(item_content) {
                    // It's a map item starting with first key
                    let mut map_entries: Vec<(String, YamlValue)> = Vec::new();
                    map_entries.push((key, val));
                    i += 1;
                    // Parse continuation keys at deeper indent
                    let item_indent = indent + 2;
                    while i < lines.len() {
                        let line3 = lines[i];
                        let trimmed3 = line3.trim();
                        if trimmed3.is_empty() || trimmed3.starts_with('#') {
                            i += 1;
                            continue;
                        }
                        let indent3 = indent_level(line3);
                        if indent3 < item_indent {
                            break;
                        }
                        if indent3 == item_indent {
                            if let Some((k, v)) = parse_kv(trimmed3) {
                                // Check if value is a multiline block indicator "|"
                                let rest = trimmed3[trimmed3.find(':').unwrap() + 1..].trim();
                                if rest == "|" {
                                    // Consume multiline block
                                    i += 1;
                                    let mut block_lines = Vec::new();
                                    let block_indent = item_indent + 2;
                                    while i < lines.len() {
                                        let bl = lines[i];
                                        let bl_trimmed = bl.trim();
                                        if bl_trimmed.is_empty() {
                                            block_lines.push("");
                                            i += 1;
                                            continue;
                                        }
                                        if indent_level(bl) < block_indent {
                                            break;
                                        }
                                        block_lines.push(bl_trimmed);
                                        i += 1;
                                    }
                                    map_entries.push((k, YamlValue::String(block_lines.join("\n"))));
                                } else if matches!(v, YamlValue::Null) {
                                    // Key with no inline value — check for nested block
                                    i += 1;
                                    if i < lines.len() {
                                        let peek = lines[i];
                                        let peek_trimmed = peek.trim();
                                        if !peek_trimmed.is_empty() && !peek_trimmed.starts_with('#') {
                                            let peek_indent = indent_level(peek);
                                            if peek_indent > indent3 {
                                                let (block_val, new_i) = parse_block(lines, i, peek_indent)?;
                                                map_entries.push((k, block_val));
                                                i = new_i;
                                            } else {
                                                map_entries.push((k, YamlValue::Null));
                                            }
                                        } else {
                                            map_entries.push((k, YamlValue::Null));
                                        }
                                    } else {
                                        map_entries.push((k, YamlValue::Null));
                                    }
                                } else {
                                    map_entries.push((k, v));
                                    i += 1;
                                }
                            } else {
                                break;
                            }
                        } else {
                            // Lines deeper than item_indent — skip them
                            // (e.g., continuation of a multiline value we didn't catch)
                            i += 1;
                        }
                    }
                    list_items.push(YamlValue::Map(map_entries));
                } else {
                    list_items.push(parse_scalar(item_content));
                    i += 1;
                }
            }
            // If we're inside a map and this list is a value, return it
            // But at top level, if entries is empty, this IS the value
            if entries.is_empty() {
                return Ok((YamlValue::List(list_items), i));
            } else {
                // Shouldn't happen in well-formed YAML we handle
                return Ok((YamlValue::Map(entries), i));
            }
        }

        // Handle key: value
        if let Some((key, val)) = parse_kv(trimmed) {
            // Check if this is a multiline block indicator
            let rest = trimmed[trimmed.find(':').unwrap() + 1..].trim();
            if rest == "|" {
                // Consume multiline block
                i += 1;
                let mut block_lines = Vec::new();
                let block_indent = indent + 2;
                while i < lines.len() {
                    let bl = lines[i];
                    let bl_trimmed = bl.trim();
                    if bl_trimmed.is_empty() {
                        block_lines.push("");
                        i += 1;
                        continue;
                    }
                    if indent_level(bl) < block_indent {
                        break;
                    }
                    block_lines.push(bl_trimmed);
                    i += 1;
                }
                entries.push((key, YamlValue::String(block_lines.join("\n"))));
            } else {
                match val {
                    YamlValue::Null => {
                        // Check if next lines form a block (list or nested map)
                        i += 1;
                        if i < lines.len() {
                            let next_line = lines[i];
                            let next_trimmed = next_line.trim();
                            if !next_trimmed.is_empty() && !next_trimmed.starts_with('#') {
                                let next_indent = indent_level(next_line);
                                if next_indent > indent {
                                    let (block_val, new_i) = parse_block(lines, i, next_indent)?;
                                    entries.push((key, block_val));
                                    i = new_i;
                                    continue;
                                }
                            }
                        }
                        entries.push((key, YamlValue::Null));
                    }
                    _ => {
                        entries.push((key, val));
                        i += 1;
                    }
                }
            }
        } else {
            i += 1;
        }
    }

    Ok((YamlValue::Map(entries), i))
}

fn parse_kv(s: &str) -> Option<(String, YamlValue)> {
    // Find the first colon that's followed by space or end-of-string
    let colon_pos = s.find(':')?;
    let key = s[..colon_pos].trim().to_string();
    if key.is_empty() {
        return None;
    }
    let rest = s[colon_pos + 1..].trim();
    if rest.is_empty() {
        Some((key, YamlValue::Null))
    } else {
        Some((key, parse_scalar(rest)))
    }
}

fn parse_scalar(s: &str) -> YamlValue {
    // Handle quoted strings
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        return YamlValue::String(s[1..s.len() - 1].to_string());
    }
    // Handle booleans
    match s.to_lowercase().as_str() {
        "true" | "yes" => return YamlValue::Bool(true),
        "false" | "no" => return YamlValue::Bool(false),
        "null" | "~" => return YamlValue::Null,
        _ => {}
    }
    YamlValue::String(s.to_string())
}
