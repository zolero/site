

use std::collections::HashMap;

use pest::error::{Error, ErrorVariant, InputLocation};
use pest::iterators::Pair;

use pest_meta::parser::{self, Rule};
use pest_meta::{optimizer, validator};

use pest_vm::Vm;

use wasm_bindgen::prelude::*;

mod serializer;

static mut VM: Option<Vm> = None;

/// Compiles the given grammar and returns any errors as a vector of hash maps.
#[wasm_bindgen]
pub fn compile_grammar_wasm(grammar: String) -> JsValue {
    let result = compile_grammar(&grammar);
    serde_wasm_bindgen::to_value(&result).expect_throw("could not serialize grammar results")
}

/// Parses the given input using the last compiled grammar and selected rule.
/// Returns the formatted output or error as a string.
#[wasm_bindgen]
pub fn parse_input_wasm(rule: String, input: String) -> String {
    parse_input(&rule, &input)
}

/// Formats the given grammar and returns the formatted version as a string.
#[wasm_bindgen]
pub fn format_grammar_wasm(grammar: String) -> String {
    let fmt = pest_fmt::Formatter::new(&grammar);
    fmt.format().unwrap_or_else(|_| grammar)
}

/// Compiles the grammar, updating the global VM state and returns any errors.
fn compile_grammar(grammar: &str) -> Vec<HashMap<String, String>> {
    let result = parser::parse(Rule::grammar_rules, grammar)
        .map_err(|error| error.renamed_rules(pest_meta::parser::rename_meta_rule));

    let pairs = match result {
        Ok(pairs) => pairs,
        Err(error) => return vec![convert_error(error, grammar)],
    };

    if let Err(errors) = validator::validate_pairs(pairs.clone()) {
        return errors.into_iter().map(|e| convert_error(e, grammar)).collect();
    }

    let ast = match parser::consume_rules(pairs) {
        Ok(ast) => ast,
        Err(errors) => return errors.into_iter().map(|e| convert_error(e, grammar)).collect(),
    };

    unsafe {
        VM = Some(Vm::new(optimizer::optimize(ast.clone())));
    }

    vec![]
}

/// Parses the input using the current VM and the specified rule.
fn parse_input(rule: &str, input: &str) -> String {
    let vm = unsafe { VM.as_ref().expect_throw("no VM") };

    match vm.parse(rule, input) {
        Ok(pairs) => {
            let lines: Vec<_> = pairs.map(|pair| format_pair(pair, 0, true)).collect();
            lines.join("\n")
        }
        Err(error) => serializer::format_error_json(&error),
    }
}



/// Converts a pest error into a hash map for serialization.
fn convert_error(error: Error<Rule>, grammar: &str) -> HashMap<String, String> {
    let message = match error.variant {
        ErrorVariant::CustomError { message } => message,
        _ => unreachable!(),
    };

    match error.location {
        InputLocation::Pos(pos) => {
            let mut map = HashMap::new();
            map.insert("from".to_owned(), line_col(pos, grammar));
            map.insert("to".to_owned(), line_col(pos, grammar));
            map.insert("message".to_owned(), message);
            map
        }
        InputLocation::Span((start, end)) => {
            let mut map = HashMap::new();
            map.insert("from".to_owned(), line_col(start, grammar));
            map.insert("to".to_owned(), line_col(end, grammar));
            map.insert("message".to_owned(), message);
            map
        }
    }
}

/// Formats a pair for output.
fn format_pair(pair: Pair<&str>, indent_level: usize, is_newline: bool) -> String {
    let indent = if is_newline {
        "  ".repeat(indent_level)
    } else {
        String::new()
    };

    let children: Vec<_> = pair.clone().into_inner().collect();
    let len = children.len();
    let children: Vec<_> = children.into_iter().map(|pair| {
        format_pair(
            pair,
            if len > 1 {
                indent_level + 1
            } else {
                indent_level
            },
            len > 1,
        )
    }).collect();

    let dash = if is_newline { "- " } else { "" };
    let pair_tag = match pair.as_node_tag() {
        Some(tag) => format!("(#{}) ", tag),
        None => String::new(),
    };

    match len {
        0 => format!(
            "{}{}{}{}: {:?}",
            indent, dash, pair_tag, pair.as_rule(), pair.as_span().as_str()
        ),
        1 => format!(
            "{}{}{}{} > {}",
            indent, dash, pair_tag, pair.as_rule(), children[0]
        ),
        _ => format!(
            "{}{}{}{}\n{}",
            indent, dash, pair_tag, pair.as_rule(), children.join("\n")
        ),
    }
}

/// Converts a byte position to a line and column number.
fn line_col(pos: usize, input: &str) -> String {
    let (line, col) = {
        let mut pos = pos;
        let slice = &input[..pos];
        let mut chars = slice.chars().peekable();
        let mut line_col = (1, 1);

        while pos != 0 {
            match chars.next() {
                Some('\r') => {
                    if let Some(&'\n') = chars.peek() {
                        chars.next();
                        if pos == 1 {
                            pos -= 1;
                        } else {
                            pos -= 2;
                        }
                        line_col = (line_col.0 + 1, 1);
                    } else {
                        pos -= 1;
                        line_col = (line_col.0, line_col.1 + 1);
                    }
                }
                Some('\n') => {
                    pos -= 1;
                    line_col = (line_col.0 + 1, 1);
                }
                Some(c) => {
                    pos -= c.len_utf8();
                    line_col = (line_col.0, line_col.1 + 1);
                }
                None => unreachable!(),
            }
        }
        line_col
    };

    format!("({}, {})", line - 1, col - 1)
}
