//! RFC 6570 URI Template expansion (Level 1–3 subset).
//!
//! Implements enough of [RFC 6570](https://www.rfc-editor.org/rfc/rfc6570) to
//! cover WoT TD `uriVariables` use cases:
//!
//! - Level 1: `{var}` — simple string expansion with percent-encoding.
//! - Level 2: `{+var}` — reserved expansion (no encoding), `{#var}` — fragment.
//! - Level 3: multiple variables `{var1,var2}`, path-style `{/var}`,
//!   label `{.var}`, form-query `{?var}`, `{;var}`, `{&var}`.
//!
//! Level 4 (prefix/modifier) expressions are **not** supported. When
//! encountered, the expression is left verbatim in the output and an error is
//! returned.

use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};

/// Error returned when URI template expansion fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateExpandError {
    /// The template contained a Level 4 modifier (e.g., `{var:3}`) that this
    /// implementation does not support.
    UnsupportedModifier(String),
    /// A variable referenced inside an expression was not present in the
    /// provided values.
    ///
    /// Per RFC 6570 §3.2.1 a missing variable causes the expression to be
    /// skipped. This error variant is returned only when the caller requests
    /// strict validation via [`expand_uri_template_strict`].
    MissingVariable(String),
    /// The template contained a malformed expression (unbalanced braces, empty
    /// expression, invalid operator).
    MalformedExpression(String),
}

impl core::fmt::Display for TemplateExpandError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedModifier(expr) => {
                write!(f, "Unsupported URI template modifier in '{}'", expr)
            }
            Self::MissingVariable(name) => {
                write!(f, "Missing URI template variable '{}'", name)
            }
            Self::MalformedExpression(expr) => {
                write!(f, "Malformed URI template expression '{}'", expr)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TemplateExpandError {}

/// Expands a URI template using the provided variable values.
///
/// Missing variables are silently skipped per RFC 6570 §3.2.1. Use
/// [`expand_uri_template_strict`] when you need an error for missing variables.
///
/// # Examples
///
/// ```
/// # use std::collections::BTreeMap;
/// # use clinkz_wot_protocol_bindings::expand_uri_template;
/// let mut vars = BTreeMap::new();
/// vars.insert("thing".to_string(), "sensor-01".to_string());
/// vars.insert("prop".to_string(), "temperature".to_string());
///
/// let expanded = expand_uri_template(
///     "zenoh://clinkz/things/{thing}/properties/{prop}",
///     &vars,
/// ).unwrap();
/// assert_eq!(expanded, "zenoh://clinkz/things/sensor-01/properties/temperature");
/// ```
pub fn expand_uri_template(
    template: &str,
    vars: &BTreeMap<String, String>,
) -> Result<String, TemplateExpandError> {
    expand_inner(template, vars, false)
}

/// Like [`expand_uri_template`] but returns an error for any missing variable.
pub fn expand_uri_template_strict(
    template: &str,
    vars: &BTreeMap<String, String>,
) -> Result<String, TemplateExpandError> {
    expand_inner(template, vars, true)
}

fn expand_inner(
    template: &str,
    vars: &BTreeMap<String, String>,
    strict: bool,
) -> Result<String, TemplateExpandError> {
    let bytes = template.as_bytes();
    let mut result = String::with_capacity(template.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'{' {
            // Find the matching close brace.
            let end = match bytes[i + 1..].iter().position(|&b| b == b'}') {
                Some(pos) => i + 1 + pos,
                None => {
                    return Err(TemplateExpandError::MalformedExpression(format!(
                        "unbalanced '{{' at position {}",
                        i
                    )));
                }
            };

            let expr_str = core::str::from_utf8(&bytes[i + 1..end])
                .map_err(|_| TemplateExpandError::MalformedExpression(template.to_string()))?;

            if expr_str.is_empty() {
                return Err(TemplateExpandError::MalformedExpression(
                    "empty expression '{}'".to_string(),
                ));
            }

            let expanded = expand_expression(expr_str, vars, strict)?;
            result.push_str(&expanded);
            i = end + 1;
        } else {
            // Literal characters are copied verbatim.
            let next_brace = bytes[i..].iter().position(|&b| b == b'{');
            let chunk_end = match next_brace {
                Some(pos) => i + pos,
                None => bytes.len(),
            };
            result.push_str(core::str::from_utf8(&bytes[i..chunk_end]).unwrap_or(""));
            i = chunk_end;
        }
    }

    Ok(result)
}

/// Parsed RFC 6570 expression operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operator {
    /// Level 1: simple string expansion with percent-encoding.
    Simple,
    /// Level 2: reserved expansion (no percent-encoding).
    Reserved,
    /// Level 2: fragment expansion (`#` prefix, no encoding).
    Fragment,
    /// Level 3: path segment (`/` prefix).
    Path,
    /// Level 3: label (`.` prefix).
    Label,
    /// Level 3: path-style parameter (`;` prefix).
    Semi,
    /// Level 3: form-style query (`?` prefix, first var).
    Form,
    /// Level 3: form-style query continuation (`&` prefix).
    FormCont,
}

impl Operator {
    fn from_first_char(c: char) -> Option<Self> {
        match c {
            '+' => Some(Self::Reserved),
            '#' => Some(Self::Fragment),
            '.' => Some(Self::Label),
            '/' => Some(Self::Path),
            ';' => Some(Self::Semi),
            '?' => Some(Self::Form),
            '&' => Some(Self::FormCont),
            _ => None,
        }
    }

    fn first_separator(&self) -> &'static str {
        match self {
            Self::Reserved | Self::Simple => "",
            Self::Fragment => "#",
            Self::Label => ".",
            Self::Path => "/",
            Self::Semi => ";",
            Self::Form => "?",
            Self::FormCont => "&",
        }
    }

    fn item_separator(&self) -> &'static str {
        match self {
            Self::Path => "/",
            Self::Label => ".",
            Self::Semi => ";",
            Self::Form => "&",
            Self::FormCont => "&",
            _ => ",",
        }
    }

    fn encode_value(&self) -> bool {
        !matches!(self, Self::Reserved | Self::Fragment)
    }

    fn named(&self) -> bool {
        matches!(self, Self::Semi | Self::Form | Self::FormCont)
    }
}

fn expand_expression(
    expr: &str,
    vars: &BTreeMap<String, String>,
    strict: bool,
) -> Result<String, TemplateExpandError> {
    // Parse operator.
    let (operator, var_list_str) = match expr.chars().next() {
        Some(c) if Operator::from_first_char(c).is_some() => {
            let op = Operator::from_first_char(c).unwrap();
            (op, &expr[1..])
        }
        _ => (Operator::Simple, expr),
    };

    // Check for Level 4 modifiers (`:N` or `*`) — not supported.
    if var_list_str.contains(':') || var_list_str.contains('*') {
        return Err(TemplateExpandError::UnsupportedModifier(format!(
            "{{{}}}",
            expr
        )));
    }

    // Parse variable names.
    let var_names: Vec<&str> = var_list_str.split(',').map(|s| s.trim()).collect();

    if var_names.is_empty() || var_names.iter().any(|v| v.is_empty()) {
        return Err(TemplateExpandError::MalformedExpression(format!(
            "{{{}}}",
            expr
        )));
    }

    // Expand each variable.
    let mut parts: Vec<String> = Vec::new();

    for var_name in &var_names {
        match vars.get(*var_name) {
            Some(value) => {
                let encoded_value = if operator.encode_value() {
                    percent_encode(value)
                } else {
                    reserved_expand(value)
                };

                if operator.named() {
                    // For named operators (semi/form/form-cont), emit
                    // `name=value` (or just `name` for empty values).
                    if encoded_value.is_empty() {
                        parts.push((*var_name).to_string());
                    } else {
                        parts.push(format!("{}={}", var_name, encoded_value));
                    }
                } else {
                    parts.push(encoded_value);
                }
            }
            None => {
                if strict {
                    return Err(TemplateExpandError::MissingVariable(
                        (*var_name).to_string(),
                    ));
                }
                // Per RFC 6570 §3.2.1: skip missing variables.
            }
        }
    }

    if parts.is_empty() {
        return Ok(String::new());
    }

    // RFC 6570: prefix with first separator, then join items with item separator.
    let mut result = String::new();
    result.push_str(operator.first_separator());
    result.push_str(&parts.join(operator.item_separator()));

    Ok(result)
}

/// Percent-encodes a value per RFC 6570 §3.1 (Level 1 simple expansion).
///
/// Unreserved characters (RFC 3986 §2.3) are passed through: `A-Z a-z 0-9 - . _ ~`.
/// All other bytes are percent-encoded.
fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{:02X}", byte));
        }
    }
    out
}

/// Reserved expansion per RFC 6570 §3.2.3 (`{+var}`).
///
/// Allows unreserved and reserved characters (RFC 3986 §2.2) but percent-encodes
/// everything else. Characters that are not allowed unencoded: `%` (when not
/// part of a valid percent-encoded sequence), and characters outside the
/// unreserved + reserved sets.
fn reserved_expand(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];

        // Pass through unreserved characters.
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
            i += 1;
            continue;
        }

        // Pass through reserved characters (RFC 3986 §2.2) that are safe
        // in reserved expansion.
        if matches!(
            b,
            b':' | b'/'
                | b'?'
                | b'#'
                | b'['
                | b']'
                | b'@'
                | b'!'
                | b'$'
                | b'&'
                | b'\''
                | b'('
                | b')'
                | b'*'
                | b'+'
                | b','
                | b';'
                | b'='
        ) {
            out.push(b as char);
            i += 1;
            continue;
        }

        // Percent-encode everything else.
        out.push_str(&format!("%{:02X}", b));
        i += 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    fn vars(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn level1_simple_expansion() {
        let v = vars(&[("var", "value")]);
        assert_eq!(expand_uri_template("{var}", &v).unwrap(), "value");
    }

    #[test]
    fn level1_multiple_expressions() {
        let v = vars(&[("thing", "sensor"), ("prop", "temp")]);
        assert_eq!(
            expand_uri_template("/things/{thing}/props/{prop}", &v).unwrap(),
            "/things/sensor/props/temp"
        );
    }

    #[test]
    fn level1_percent_encoding() {
        let v = vars(&[("q", "hello world")]);
        assert_eq!(
            expand_uri_template("?q={q}", &v).unwrap(),
            "?q=hello%20world"
        );
    }

    #[test]
    fn level1_special_chars() {
        let v = vars(&[("path", "/a/b/c")]);
        assert_eq!(expand_uri_template("{path}", &v).unwrap(), "%2Fa%2Fb%2Fc");
    }

    #[test]
    fn level2_reserved_expansion() {
        let v = vars(&[("path", "/a/b/c")]);
        assert_eq!(expand_uri_template("{+path}", &v).unwrap(), "/a/b/c");
    }

    #[test]
    fn level2_fragment_expansion() {
        let v = vars(&[("section", "intro")]);
        assert_eq!(
            expand_uri_template("doc{#section}", &v).unwrap(),
            "doc#intro"
        );
    }

    #[test]
    fn level3_path_segments() {
        let v = vars(&[("x", "a"), ("y", "b")]);
        assert_eq!(expand_uri_template("{/x,y}", &v).unwrap(), "/a/b");
    }

    #[test]
    fn level3_form_query() {
        let v = vars(&[("x", "1024"), ("y", "768")]);
        let result = expand_uri_template("{?x,y}", &v).unwrap();
        assert_eq!(result, "?x=1024&y=768");
    }

    #[test]
    fn missing_variable_skipped() {
        let v = vars(&[("a", "1")]);
        assert_eq!(expand_uri_template("{a}{b}", &v).unwrap(), "1");
    }

    #[test]
    fn strict_mode_errors_on_missing() {
        let v = vars(&[("a", "1")]);
        assert!(matches!(
            expand_uri_template_strict("{a}{b}", &v),
            Err(TemplateExpandError::MissingVariable(name)) if name == "b"
        ));
    }

    #[test]
    fn unbalanced_brace_errors() {
        let v = BTreeMap::new();
        assert!(matches!(
            expand_uri_template("{var", &v),
            Err(TemplateExpandError::MalformedExpression(_))
        ));
    }

    #[test]
    fn level4_modifier_rejected() {
        let v = vars(&[("x", "abcdef")]);
        assert!(matches!(
            expand_uri_template("{x:3}", &v),
            Err(TemplateExpandError::UnsupportedModifier(_))
        ));
    }

    #[test]
    fn no_expressions_returns_literal() {
        let v = BTreeMap::new();
        assert_eq!(
            expand_uri_template("zenoh://host/path", &v).unwrap(),
            "zenoh://host/path"
        );
    }

    #[test]
    fn wot_td_typical_pattern() {
        let mut v = BTreeMap::new();
        v.insert("thing_id".to_string(), "gw001".to_string());
        v.insert("property".to_string(), "temperature".to_string());

        assert_eq!(
            expand_uri_template(
                "zenoh://clinkz/gateways/{thing_id}/properties/{property}",
                &v
            )
            .unwrap(),
            "zenoh://clinkz/gateways/gw001/properties/temperature"
        );
    }

    #[test]
    fn wot_td_reserved_base_pattern() {
        let mut v = BTreeMap::new();
        v.insert("base".to_string(), "clinkz/gateways/gw001".to_string());
        v.insert("prop".to_string(), "temperature".to_string());

        assert_eq!(
            expand_uri_template("{+base}/properties/{prop}", &v).unwrap(),
            "clinkz/gateways/gw001/properties/temperature"
        );
    }
}
