#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ConnectType {
    File,
    Db,
    Api,
}
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BoolLiteral(bool),

    // Identifier
    Ident(String),

    // Keywords
    Fn,
    Let,
    Mut,
    If,
    Else,
    While,
    For,
    In,
    Return,
    Print,
    Struct,
    Enum,
    Break,
    Continue,
    Connect,
    File,
    Db,
    Api,
    As,
    Match,
    Import,

    // Type keywords
    IntType,
    FloatType,
    BoolType,
    StringType,
    VoidType,
    ListType,

    // Arithmetic
    Plus,
    Minus,
    Star,
    Slash,
    Percent,

    // Comparison
    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    // Logical
    And,
    Or,
    Bang,
    Pipe, // | (lambda delimiter)

    // Assignment
    Equal,

    // Punctuation
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Semicolon,
    Colon,
    ColonColon, // ::
    Arrow,    // ->
    FatArrow, // =>
    DotDot,

    // Brackets
    LeftBracket,
    RightBracket,

    // SQL prefix
    Hash,

    // SQL keywords (used after #)
    Select,
    Single,
    From,
    Where,
    Insert,
    Into,
    Values,

    // Try / propagation
    Question, // ? (postfix unwrap operator)

    // F-string:  f"Hola {name}, tienes {age} años"
    FStringRaw(Vec<FStringPart>),

    // Special
    Eof,
}

/// A segment of an f-string token.
#[derive(Debug, Clone, PartialEq)]
pub enum FStringPart {
    Literal(String), // plain text between interpolations
    Expr(String),    // raw source of the expression inside { }
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::IntLiteral(v) => write!(f, "{}", v),
            TokenKind::FloatLiteral(v) => write!(f, "{}", v),
            TokenKind::StringLiteral(v) => write!(f, "\"{}\"", v),
            TokenKind::BoolLiteral(v) => write!(f, "{}", v),
            TokenKind::Ident(v) => write!(f, "{}", v),
            TokenKind::Fn => write!(f, "fn"),
            TokenKind::Let => write!(f, "let"),
            TokenKind::Mut => write!(f, "mut"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::While => write!(f, "while"),
            TokenKind::For => write!(f, "for"),
            TokenKind::In => write!(f, "in"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::Print => write!(f, "print"),
            TokenKind::Struct => write!(f, "struct"),
            TokenKind::Enum => write!(f, "enum"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::Connect => write!(f, "connect"),
            TokenKind::File => write!(f, "file"),
            TokenKind::Db => write!(f, "db"),
            TokenKind::Api => write!(f, "api"),
            TokenKind::As => write!(f, "as"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::Import => write!(f, "import"),
            TokenKind::IntType => write!(f, "int"),
            TokenKind::FloatType => write!(f, "float"),
            TokenKind::BoolType => write!(f, "bool"),
            TokenKind::StringType => write!(f, "string"),
            TokenKind::VoidType => write!(f, "void"),
            TokenKind::ListType => write!(f, "list"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::EqualEqual => write!(f, "=="),
            TokenKind::BangEqual => write!(f, "!="),
            TokenKind::Less => write!(f, "<"),
            TokenKind::LessEqual => write!(f, "<="),
            TokenKind::Greater => write!(f, ">"),
            TokenKind::GreaterEqual => write!(f, ">="),
            TokenKind::And => write!(f, "&&"),
            TokenKind::Or => write!(f, "||"),
            TokenKind::Bang => write!(f, "!"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Equal => write!(f, "="),
            TokenKind::LeftParen => write!(f, "("),
            TokenKind::RightParen => write!(f, ")"),
            TokenKind::LeftBrace => write!(f, "{{"),
            TokenKind::RightBrace => write!(f, "}}"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::ColonColon => write!(f, "::"),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::FatArrow => write!(f, "=>"),
            TokenKind::DotDot => write!(f, ".."),
            TokenKind::LeftBracket => write!(f, "["),
            TokenKind::RightBracket => write!(f, "]"),
            TokenKind::Hash => write!(f, "#"),
            TokenKind::Select => write!(f, "SELECT"),
            TokenKind::Single => write!(f, "SINGLE"),
            TokenKind::From => write!(f, "FROM"),
            TokenKind::Where => write!(f, "WHERE"),
            TokenKind::Insert => write!(f, "INSERT"),
            TokenKind::Into => write!(f, "INTO"),
            TokenKind::Values => write!(f, "VALUES"),
            TokenKind::Question => write!(f, "?"),
            TokenKind::FStringRaw(_) => write!(f, "f-string"),
            TokenKind::Eof => write!(f, "EOF"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Comment {
    pub text: String,
    pub line: usize,
    pub column: usize,
    pub is_inline: bool,
}

pub fn lookup_keyword(ident: &str) -> Option<TokenKind> {
    match ident {
        "fn" => Some(TokenKind::Fn),
        "let" => Some(TokenKind::Let),
        "mut" => Some(TokenKind::Mut),
        "if" => Some(TokenKind::If),
        "else" => Some(TokenKind::Else),
        "while" => Some(TokenKind::While),
        "for" => Some(TokenKind::For),
        "in" => Some(TokenKind::In),
        "return" => Some(TokenKind::Return),
        "print" => Some(TokenKind::Print),
        "struct" => Some(TokenKind::Struct),
        "enum" => Some(TokenKind::Enum),
        "connect" => Some(TokenKind::Connect),
        "file" => Some(TokenKind::File),
        "db" => Some(TokenKind::Db),
        "api" => Some(TokenKind::Api),
        "as" => Some(TokenKind::As),
        "match" => Some(TokenKind::Match),
        "import" => Some(TokenKind::Import),
        "true" => Some(TokenKind::BoolLiteral(true)),
        "false" => Some(TokenKind::BoolLiteral(false)),
        "int" => Some(TokenKind::IntType),
        "float" => Some(TokenKind::FloatType),
        "bool" => Some(TokenKind::BoolType),
        "string" => Some(TokenKind::StringType),
        "void" => Some(TokenKind::VoidType),
        "list" => Some(TokenKind::ListType),
        "SELECT" => Some(TokenKind::Select),
        "SINGLE" => Some(TokenKind::Single),
        "FROM" => Some(TokenKind::From),
        "WHERE" => Some(TokenKind::Where),
        "INSERT" => Some(TokenKind::Insert),
        "INTO" => Some(TokenKind::Into),
        "VALUES" => Some(TokenKind::Values),
        _ => None,
    }
}
