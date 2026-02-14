use crate::lexer::Span;

#[derive(Debug)]
pub enum NovaError {
    Lexer(LexerError),
    Parse(ParseError),
    Semantic(SemanticError),
    Runtime(RuntimeError),
}

#[derive(Debug, Clone)]
pub struct LexerError {
    pub message: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub expected: String,
    pub found: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct SemanticError {
    pub message: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub message: String,
    pub span: Span,
}

impl From<LexerError> for NovaError {
    fn from(e: LexerError) -> Self { NovaError::Lexer(e) }
}

impl From<ParseError> for NovaError {
    fn from(e: ParseError) -> Self { NovaError::Parse(e) }
}

impl From<SemanticError> for NovaError {
    fn from(e: SemanticError) -> Self { NovaError::Semantic(e) }
}

impl From<RuntimeError> for NovaError {
    fn from(e: RuntimeError) -> Self { NovaError::Runtime(e) }
}

pub fn format_error(error: &NovaError, source: &str, filename: &str) -> String {
    let (prefix, message, span) = match error {
        NovaError::Lexer(e) => ("L001", e.message.as_str(), e.span),
        NovaError::Parse(e) => ("P001", e.message.as_str(), e.span),
        NovaError::Semantic(e) => ("S001", e.message.as_str(), e.span),
        NovaError::Runtime(e) => ("R001", e.message.as_str(), e.span),
    };

    let lines: Vec<&str> = source.lines().collect();
    let line_str = if span.line > 0 && span.line <= lines.len() {
        lines[span.line - 1]
    } else {
        ""
    };

    let line_num = span.line;
    let col = span.column;
    let pad = " ".repeat(line_num.to_string().len());

    let caret_count = if span.end > span.start { span.end - span.start } else { 1 };
    let carets = "^".repeat(caret_count);
    let caret_pad = " ".repeat(col.saturating_sub(1));

    format!(
        "error[{}]: {}\n {}--> {}:{}:{}\n {} |\n{} | {}\n {} | {}{}\n",
        prefix, message,
        pad, filename, line_num, col,
        pad,
        line_num, line_str,
        pad, caret_pad, carets,
    )
}
