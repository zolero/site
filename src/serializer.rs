use pest::error::{Error, ErrorVariant, InputLocation, LineColLocation};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct SerializableError<R> {
    message: String,
    variant: SerializableErrorVariant<R>,
    location: SerializableInputLocation,
    line_col: SerializableLineColLocation,
    // path: Option<String>,
    // line: String,
    // continued_line: Option<String>,
    // parse_attempts: Option<Vec<R>>, // Simplified assuming R is already serializable
}

#[derive(Serialize, Deserialize, Debug)]
enum SerializableErrorVariant<R> {
    ParsingError {
        positives: Vec<R>,
        negatives: Vec<R>,
    },
    CustomError {
        message: String,
    },
}

#[derive(Serialize, Deserialize, Debug)]
enum SerializableInputLocation {
    Pos(usize),
    Span((usize, usize)),
}

#[derive(Serialize, Deserialize, Debug)]
enum SerializableLineColLocation {
    Pos((usize, usize)),
    Span((usize, usize), (usize, usize)),
}

fn convert_error_to_serializable<'a>(error: &'a Error<&'a str>) -> SerializableError<&'a str>
where
{
    SerializableError {
        message: error.clone().renamed_rules(|r| r.to_string()).to_string(),
        variant: match error.variant {
            ErrorVariant::ParsingError { ref positives, ref negatives } => SerializableErrorVariant::ParsingError {
                positives: positives.clone(),
                negatives: negatives.clone(),
            },
            ErrorVariant::CustomError { ref message } => SerializableErrorVariant::CustomError {
                message: message.clone(),
            },
        },
        location: match error.location {
            InputLocation::Pos(pos) => SerializableInputLocation::Pos(pos),
            InputLocation::Span((start, end)) => SerializableInputLocation::Span((start, end)),
        },
        line_col: match error.line_col {
            LineColLocation::Pos((line, col)) => SerializableLineColLocation::Pos((line, col)),
            LineColLocation::Span((start_line, start_col), (end_line, end_col)) => SerializableLineColLocation::Span((start_line, start_col), (end_line, end_col)),
        },
        // path: None,
        // line: "".to_string(),
        // continued_line: None,
        // parse_attempts: None,
    }
}

pub(crate) fn format_error_json(error: &Error<&str>) -> String {
    let serializable_error = convert_error_to_serializable(error);
    serde_json::to_string_pretty(&serializable_error).unwrap_or_else(|e| {
        eprintln!("Failed to serialize error: {}", e);
        "Failed to serialize error".to_string()
    })
}