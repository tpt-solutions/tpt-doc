//! Data-merge templating: fill an HTML template from JSON, then render it.
//!
//! A deliberately small Mustache-like language:
//!
//! | Syntax | Meaning |
//! |---|---|
//! | `{{path.to.field}}` | HTML-escaped value (`items.0.name` indexes arrays) |
//! | `{{#if path}}…{{else}}…{{/if}}` | conditional (`else` optional) |
//! | `{{#each path}}…{{/each}}` | repeat for every array element / object value |
//! | `{{.}}` / `{{@index}}` | inside `each`: the current item / its 0-based index |
//!
//! Lookups try the innermost `each` item first, then the enclosing scopes, then
//! the root. A **missing** field is an error naming the path — a document that
//! silently prints a blank where an invoice number belongs is worse than no
//! document. All substituted text is HTML-escaped; there is no raw form.

use serde_json::Value;
use tpt_doc_core::DocError;

/// Maximum nesting of `#if` / `#each` blocks.
const MAX_DEPTH: usize = 16;
/// Maximum size of the rendered text.
const MAX_OUTPUT: usize = 16 * 1024 * 1024;

#[derive(Debug)]
enum Node {
    Text(String),
    Var(String),
    If {
        path: String,
        then: Vec<Node>,
        otherwise: Vec<Node>,
    },
    Each {
        path: String,
        body: Vec<Node>,
    },
}

/// Fill `template` from `data`, returning the merged text.
///
/// # Errors
/// Returns [`DocError`] on malformed template syntax, a missing field, a
/// non-scalar value used as text, excessive nesting, or oversized output.
pub fn merge(template: &str, data: &Value) -> Result<String, DocError> {
    let tokens = tokenize(template)?;
    let mut pos = 0;
    let nodes = parse(&tokens, &mut pos, 0, None)?;
    if pos != tokens.len() {
        return Err(bad("unexpected closing tag"));
    }
    let mut out = String::new();
    let mut scopes: Vec<(&Value, usize)> = Vec::new();
    render(&nodes, data, &mut scopes, &mut out)?;
    Ok(out)
}

fn bad(message: impl Into<String>) -> DocError {
    DocError::invalid_format(format!("template: {}", message.into()))
}

#[derive(Debug)]
enum Token<'a> {
    Text(&'a str),
    Tag(&'a str),
}

fn tokenize(src: &str) -> Result<Vec<Token<'_>>, DocError> {
    let mut tokens = Vec::new();
    let mut rest = src;
    while let Some(open) = rest.find("{{") {
        if open > 0 {
            tokens.push(Token::Text(&rest[..open]));
        }
        let after = &rest[open + 2..];
        let close = after
            .find("}}")
            .ok_or_else(|| bad("unterminated `{{` tag"))?;
        tokens.push(Token::Tag(after[..close].trim()));
        rest = &after[close + 2..];
    }
    if !rest.is_empty() {
        tokens.push(Token::Text(rest));
    }
    Ok(tokens)
}

/// Parse until the matching close of `until` (`"if"`/`"each"`) or the end.
fn parse(
    tokens: &[Token<'_>],
    pos: &mut usize,
    depth: usize,
    until: Option<&str>,
) -> Result<Vec<Node>, DocError> {
    if depth > MAX_DEPTH {
        return Err(bad("blocks nested too deeply"));
    }
    let mut nodes = Vec::new();
    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Text(t) => {
                nodes.push(Node::Text((*t).to_owned()));
                *pos += 1;
            }
            Token::Tag(tag) => {
                if let Some(path) = tag.strip_prefix("#if ") {
                    *pos += 1;
                    let then = parse(tokens, pos, depth + 1, Some("if"))?;
                    let mut otherwise = Vec::new();
                    if matches!(tokens.get(*pos), Some(Token::Tag("else"))) {
                        *pos += 1;
                        otherwise = parse(tokens, pos, depth + 1, Some("if"))?;
                    }
                    expect_close(tokens, pos, "if")?;
                    nodes.push(Node::If {
                        path: path.trim().to_owned(),
                        then,
                        otherwise,
                    });
                } else if let Some(path) = tag.strip_prefix("#each ") {
                    *pos += 1;
                    let body = parse(tokens, pos, depth + 1, Some("each"))?;
                    expect_close(tokens, pos, "each")?;
                    nodes.push(Node::Each {
                        path: path.trim().to_owned(),
                        body,
                    });
                } else if *tag == "else" || tag.starts_with('/') {
                    // Belongs to the enclosing block; let the caller decide.
                    if until.is_none() {
                        return Err(bad(format!("unexpected `{{{{{tag}}}}}`")));
                    }
                    return Ok(nodes);
                } else if tag.starts_with('#') {
                    return Err(bad(format!("unknown block `{{{{{tag}}}}}`")));
                } else if tag.is_empty() {
                    return Err(bad("empty `{{}}` tag"));
                } else {
                    nodes.push(Node::Var((*tag).to_owned()));
                    *pos += 1;
                }
            }
        }
    }
    match until {
        Some(block) => Err(bad(format!("`#{block}` is never closed"))),
        None => Ok(nodes),
    }
}

fn expect_close(tokens: &[Token<'_>], pos: &mut usize, block: &str) -> Result<(), DocError> {
    match tokens.get(*pos) {
        Some(Token::Tag(t)) if t.strip_prefix('/') == Some(block) => {
            *pos += 1;
            Ok(())
        }
        _ => Err(bad(format!("`#{block}` is never closed"))),
    }
}

fn resolve<'a>(path: &str, root: &'a Value, scopes: &[(&'a Value, usize)]) -> Option<&'a Value> {
    if path == "." {
        return scopes.last().map(|(v, _)| *v);
    }
    let walk = |start: &'a Value| {
        path.split('.').try_fold(start, |value, seg| match value {
            Value::Object(map) => map.get(seg),
            Value::Array(items) => seg.parse::<usize>().ok().and_then(|i| items.get(i)),
            _ => None,
        })
    };
    scopes
        .iter()
        .rev()
        .find_map(|(scope, _)| walk(scope))
        .or_else(|| walk(root))
}

fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

fn push_escaped(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
}

fn check_size(out: &str) -> Result<(), DocError> {
    if out.len() > MAX_OUTPUT {
        Err(bad("rendered output is too large"))
    } else {
        Ok(())
    }
}

fn render<'a>(
    nodes: &[Node],
    root: &'a Value,
    scopes: &mut Vec<(&'a Value, usize)>,
    out: &mut String,
) -> Result<(), DocError> {
    for node in nodes {
        match node {
            Node::Text(t) => out.push_str(t),
            Node::Var(path) => {
                if path == "@index" {
                    let (_, index) = scopes
                        .last()
                        .ok_or_else(|| bad("`@index` used outside `#each`"))?;
                    out.push_str(&index.to_string());
                } else {
                    match resolve(path, root, scopes) {
                        None => return Err(bad(format!("missing field `{path}`"))),
                        Some(Value::String(s)) => push_escaped(out, s),
                        Some(Value::Number(n)) => out.push_str(&n.to_string()),
                        Some(Value::Bool(b)) => out.push_str(&b.to_string()),
                        Some(Value::Null) => {}
                        Some(_) => {
                            return Err(bad(format!(
                                "field `{path}` is an array or object, not text"
                            )));
                        }
                    }
                }
            }
            Node::If {
                path,
                then,
                otherwise,
            } => {
                let value = resolve(path, root, scopes)
                    .ok_or_else(|| bad(format!("missing field `{path}`")))?;
                let branch = if truthy(value) { then } else { otherwise };
                render(branch, root, scopes, out)?;
            }
            Node::Each { path, body } => {
                let value = resolve(path, root, scopes)
                    .ok_or_else(|| bad(format!("missing field `{path}`")))?;
                let items: Vec<&Value> = match value {
                    Value::Array(a) => a.iter().collect(),
                    Value::Object(o) => o.values().collect(),
                    Value::Null => Vec::new(),
                    _ => return Err(bad(format!("`#each {path}` needs an array or object"))),
                };
                for (index, item) in items.into_iter().enumerate() {
                    scopes.push((item, index));
                    let result = render(body, root, scopes, out);
                    scopes.pop();
                    result?;
                    check_size(out)?;
                }
            }
        }
        check_size(out)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn substitutes_and_escapes() {
        let out = merge(
            "<p>{{name}} &amp; {{a.b}}</p>",
            &json!({"name": "<Bob>", "a": {"b": 3}}),
        )
        .unwrap();
        assert_eq!(out, "<p>&lt;Bob&gt; &amp; 3</p>");
    }

    #[test]
    fn each_with_index_and_parent_scope() {
        let data = json!({"cur": "NZD", "lines": [{"n": "a"}, {"n": "b"}]});
        let out = merge("{{#each lines}}{{@index}}:{{n}}/{{cur}};{{/each}}", &data).unwrap();
        assert_eq!(out, "0:a/NZD;1:b/NZD;");
        let out = merge("{{#each xs}}[{{.}}]{{/each}}", &json!({"xs": [1, "two"]})).unwrap();
        assert_eq!(out, "[1][two]");
    }

    #[test]
    fn if_else_truthiness() {
        let t = "{{#if x}}yes{{else}}no{{/if}}";
        assert_eq!(merge(t, &json!({"x": 1})).unwrap(), "yes");
        for falsy in [json!(0), json!(""), json!([]), json!(null), json!(false)] {
            assert_eq!(merge(t, &json!({ "x": falsy })).unwrap(), "no");
        }
    }

    #[test]
    fn errors_are_specific() {
        let missing = merge("{{nope}}", &json!({})).unwrap_err().to_string();
        assert!(missing.contains("missing field `nope`"), "{missing}");
        assert!(merge("{{#if a}}x", &json!({"a": 1})).is_err());
        assert!(merge("x{{/if}}", &json!({})).is_err());
        assert!(merge("{{", &json!({})).is_err());
        assert!(merge("{{a}}", &json!({"a": [1]})).is_err());
        assert!(merge("{{@index}}", &json!({})).is_err());
    }

    #[test]
    fn depth_and_output_are_bounded() {
        let deep = "{{#if a}}".repeat(40) + &"{{/if}}".repeat(40);
        assert!(merge(&deep, &json!({"a": 1})).is_err());
        // 3 nested loops of 300 items would be 27M iterations of output.
        let big = json!({"xs": vec![1; 300]});
        let t = "{{#each xs}}{{#each xs}}{{#each xs}}abcdefgh{{/each}}{{/each}}{{/each}}";
        assert!(merge(t, &big).is_err());
    }
}
