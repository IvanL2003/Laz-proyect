use crate::lexer::{ConnectType, Span};

// Placeholder - se implementa en Fase 3
#[derive(Debug, Clone)]
pub struct Program {
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone)]
pub enum Declaration {
    Function(FnDecl),
    Struct(StructDecl),
    Connect(ConnectDecl),
    Statement(Stmt),
}

#[derive(Debug, Clone)]
pub struct ConnectDecl {
    pub connect_type: ConnectType,
    pub file_path: String,
    pub alias: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeAnnotation,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<StructField>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnotation {
    Int,
    Float,
    Bool,
    StringType,
    Void,
    List(Box<TypeAnnotation>),
    UserDefined(String),
}

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let {
        name: String,
        mutable: bool,
        type_ann: TypeAnnotation,
        initializer: Expr,
        span: Span,
    },
    Assign {
        target: AssignTarget,
        value: Expr,
        span: Span,
    },
    If {
        condition: Expr,
        then_block: Block,
        else_branch: Option<Box<ElseBranch>>,
        span: Span,
    },
    While {
        condition: Expr,
        body: Block,
        span: Span,
    },
    For {
        variable: String,
        start: Expr,
        end: Expr,
        body: Block,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Print {
        args: Vec<Expr>,
        span: Span,
    },
    Expression {
        expr: Expr,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum ElseBranch {
    ElseIf(Box<Stmt>),
    ElseBlock(Block),
}

#[derive(Debug, Clone)]
pub enum AssignTarget {
    Variable(String),
    FieldAccess { object: Box<Expr>, field: String },
}

#[derive(Debug, Clone)]
pub enum Expr {
    IntLiteral { value: i64, span: Span },
    FloatLiteral { value: f64, span: Span },
    StringLiteral { value: String, span: Span },
    BoolLiteral { value: bool, span: Span },
    Identifier { name: String, span: Span },
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        span: Span,
    },
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    FnCall {
        callee: String,
        args: Vec<Expr>,
        span: Span,
    },
    FieldAccess {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    StructInit {
        name: String,
        fields: Vec<(String, Expr)>,
        span: Span,
    },
    Grouped {
        expr: Box<Expr>,
        span: Span,
    },
    SqlSelect {
        columns: Vec<String>,         // column names (* = all)
        table_ref: SqlTableRef,       // alias or csv("file")
        condition: Option<Box<Expr>>, // WHERE clause (reuses normal Expr)
        single: bool,                 // true for #SELECT SINGLE
        span: Span,
    },
    SqlInsert {
        table_ref: SqlTableRef,       // alias or csv("file")
        values: Vec<Expr>,            // VALUES (...)
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::IntLiteral { span, .. }
            | Expr::FloatLiteral { span, .. }
            | Expr::StringLiteral { span, .. }
            | Expr::BoolLiteral { span, .. }
            | Expr::Identifier { span, .. }
            | Expr::BinaryOp { span, .. }
            | Expr::UnaryOp { span, .. }
            | Expr::FnCall { span, .. }
            | Expr::FieldAccess { span, .. }
            | Expr::StructInit { span, .. }
            | Expr::Grouped { span, .. }
            | Expr::SqlSelect { span, .. }
            | Expr::SqlInsert { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SqlTableRef {
    Alias(String),              // FROM users (uses connect csv alias)
    Inline(String),             // FROM csv("users.csv") (direct file path)
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Lte, Gt, Gte,
    And, Or,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Neg,
    Not,
}
