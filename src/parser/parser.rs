use crate::lexer::{ConnectType, Span, Token, TokenKind};
use crate::parser::ast::*;
use crate::utils::error::ParseError;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    // --- Helper methods ---

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.pos].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        token
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(kind)
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<Token, ParseError> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(ParseError {
                message: format!("expected '{}', found '{}'", kind, self.peek_kind()),
                expected: format!("{}", kind),
                found: format!("{}", self.peek_kind()),
                span: self.peek().span,
            })
        }
    }

    fn expect_ident(&mut self) -> Result<(String, Span), ParseError> {
        if let TokenKind::Ident(_) = self.peek_kind() {
            let token = self.advance();
            if let TokenKind::Ident(name) = token.kind {
                Ok((name, token.span))
            } else {
                unreachable!()
            }
        } else {
            Err(ParseError {
                message: format!("expected identifier, found '{}'", self.peek_kind()),
                expected: "identifier".to_string(),
                found: format!("{}", self.peek_kind()),
                span: self.peek().span,
            })
        }
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }

    // --- Parsing ---

    pub fn parse(&mut self) -> Result<Program, ParseError> {
        let mut declarations = Vec::new();

        while !self.is_at_end() {
            declarations.push(self.parse_declaration()?);
        }

        Ok(Program { declarations })
    }

    fn parse_declaration(&mut self) -> Result<Declaration, ParseError> {
        match self.peek_kind() {
            TokenKind::Fn => Ok(Declaration::Function(self.parse_fn_decl()?)),
            TokenKind::Struct => Ok(Declaration::Struct(self.parse_struct_decl()?)),
            TokenKind::Connect => Ok(Declaration::Connect(self.parse_connect()?)),
            _ => Ok(Declaration::Statement(self.parse_statement()?)),
        }
    }

    fn parse_connect(&mut self) -> Result<ConnectDecl, ParseError> {
        let connect_token = self.advance(); // consume 'connect'
        let connect_type: ConnectType = match self.peek_kind() {
            TokenKind::File => {
                self.advance(); // consume 'file'
                ConnectType::File
            }

            TokenKind::Db => {
                self.advance(); // consume 'db'
                ConnectType::Db
            }

            TokenKind::Api => {
                self.advance(); // consume 'api'
                ConnectType::Api
            }

            _ => {
                return Err(ParseError {
                    message: format!(
                        "expected 'file', 'db' or 'api' after 'connect', found '{}'",
                        self.peek_kind()
                    ),
                    expected: "'file', 'db' or 'api'".to_string(),
                    found: format!("{}", self.peek_kind()),
                    span: self.peek().span,
                });
            }
        };

        // Expect string literal for file path
        let file_path = match self.peek_kind() {
            TokenKind::StringLiteral(_) => {
                let token = self.advance();
                if let TokenKind::StringLiteral(s) = token.kind {
                    s
                } else {
                    unreachable!()
                }
            }
            _ => {
                return Err(ParseError {
                    message: format!(
                        "expected file path string after 'connect csv', found '{}'",
                        self.peek_kind()
                    ),
                    expected: "string literal".to_string(),
                    found: format!("{}", self.peek_kind()),
                    span: self.peek().span,
                });
            }
        };

        self.expect(&TokenKind::As)?; // expect 'as'
        let (alias, _) = self.expect_ident()?; // alias name
        self.expect(&TokenKind::Semicolon)?;

        Ok(ConnectDecl {
            connect_type,
            file_path,
            alias,
            span: connect_token.span,
        })
    }

    fn parse_fn_decl(&mut self) -> Result<FnDecl, ParseError> {
        let fn_token = self.advance(); // consume 'fn'
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LeftParen)?;

        let mut params = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            loop {
                let (pname, pspan) = self.expect_ident()?;
                self.expect(&TokenKind::Colon)?;
                let type_ann = self.parse_type()?;
                params.push(Param {
                    name: pname,
                    type_ann,
                    span: pspan,
                });

                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RightParen)?;

        let return_type = if self.match_token(&TokenKind::Arrow) {
            self.parse_type()?
        } else {
            TypeAnnotation::Void
        };

        let body = self.parse_block()?;

        Ok(FnDecl {
            name,
            params,
            return_type,
            body,
            span: fn_token.span,
        })
    }

    fn parse_struct_decl(&mut self) -> Result<StructDecl, ParseError> {
        let struct_token = self.advance(); // consume 'struct'
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LeftBrace)?;

        let mut fields = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let (fname, fspan) = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let type_ann = self.parse_type()?;
            fields.push(StructField {
                name: fname,
                type_ann,
                span: fspan,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RightBrace)?;

        Ok(StructDecl {
            name,
            fields,
            span: struct_token.span,
        })
    }

    fn parse_type(&mut self) -> Result<TypeAnnotation, ParseError> {
        let token = self.advance();
        match token.kind {
            TokenKind::IntType => Ok(TypeAnnotation::Int),
            TokenKind::FloatType => Ok(TypeAnnotation::Float),
            TokenKind::BoolType => Ok(TypeAnnotation::Bool),
            TokenKind::StringType => Ok(TypeAnnotation::StringType),
            TokenKind::VoidType => Ok(TypeAnnotation::Void),
            TokenKind::ListType => {
                // Expect list<Type>
                self.expect(&TokenKind::Less)?;
                let inner = self.parse_type()?;
                self.expect(&TokenKind::Greater)?;
                Ok(TypeAnnotation::List(Box::new(inner)))
            }
            TokenKind::Ident(name) => Ok(TypeAnnotation::UserDefined(name)),
            _ => Err(ParseError {
                message: format!("expected type, found '{}'", token.kind),
                expected: "type".to_string(),
                found: format!("{}", token.kind),
                span: token.span,
            }),
        }
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let brace = self.expect(&TokenKind::LeftBrace)?;
        let mut statements = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }
        self.expect(&TokenKind::RightBrace)?;

        Ok(Block {
            statements,
            span: brace.span,
        })
    }

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        match self.peek_kind() {
            TokenKind::Let => self.parse_let_stmt(),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::For => self.parse_for_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::Print => self.parse_print_stmt(),
            _ => self.parse_assign_or_expr_stmt(),
        }
    }

    fn parse_let_stmt(&mut self) -> Result<Stmt, ParseError> {
        let let_token = self.advance(); // consume 'let'

        let mutable = self.match_token(&TokenKind::Mut);
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        let type_ann = self.parse_type()?;
        self.expect(&TokenKind::Equal)?;
        let initializer = self.parse_expression()?;
        self.expect(&TokenKind::Semicolon)?;

        Ok(Stmt::Let {
            name,
            mutable,
            type_ann,
            initializer,
            span: let_token.span,
        })
    }

    fn parse_if_stmt(&mut self) -> Result<Stmt, ParseError> {
        let if_token = self.advance(); // consume 'if'
        let condition = self.parse_expression()?;
        let then_block = self.parse_block()?;

        let else_branch = if self.match_token(&TokenKind::Else) {
            if self.check(&TokenKind::If) {
                Some(Box::new(ElseBranch::ElseIf(Box::new(
                    self.parse_if_stmt()?,
                ))))
            } else {
                Some(Box::new(ElseBranch::ElseBlock(self.parse_block()?)))
            }
        } else {
            None
        };

        Ok(Stmt::If {
            condition,
            then_block,
            else_branch,
            span: if_token.span,
        })
    }

    fn parse_while_stmt(&mut self) -> Result<Stmt, ParseError> {
        let while_token = self.advance(); // consume 'while'
        let condition = self.parse_expression()?;
        let body = self.parse_block()?;

        Ok(Stmt::While {
            condition,
            body,
            span: while_token.span,
        })
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, ParseError> {
        let for_token = self.advance(); // consume 'for'
        let (variable, _) = self.expect_ident()?;
        self.expect(&TokenKind::In)?;
        let start = self.parse_expression()?;
        self.expect(&TokenKind::DotDot)?;
        let end = self.parse_expression()?;
        let body = self.parse_block()?;

        Ok(Stmt::For {
            variable,
            start,
            end,
            body,
            span: for_token.span,
        })
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, ParseError> {
        let return_token = self.advance(); // consume 'return'

        let value = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.expect(&TokenKind::Semicolon)?;

        Ok(Stmt::Return {
            value,
            span: return_token.span,
        })
    }

    fn parse_print_stmt(&mut self) -> Result<Stmt, ParseError> {
        let print_token = self.advance(); // consume 'print'
        self.expect(&TokenKind::LeftParen)?;

        let mut args = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            loop {
                args.push(self.parse_expression()?);
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RightParen)?;
        self.expect(&TokenKind::Semicolon)?;

        Ok(Stmt::Print {
            args,
            span: print_token.span,
        })
    }

    fn parse_assign_or_expr_stmt(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.parse_expression()?;
        let span = expr.span();

        // Check if this is an assignment
        if self.check(&TokenKind::Equal) {
            self.advance(); // consume '='
            let value = self.parse_expression()?;
            self.expect(&TokenKind::Semicolon)?;

            let target = match expr {
                Expr::Identifier { name, .. } => AssignTarget::Variable(name),
                Expr::FieldAccess { object, field, .. } => {
                    AssignTarget::FieldAccess { object, field }
                }
                _ => {
                    return Err(ParseError {
                        message: "invalid assignment target".to_string(),
                        expected: "variable or field".to_string(),
                        found: "expression".to_string(),
                        span,
                    });
                }
            };

            Ok(Stmt::Assign {
                target,
                value,
                span,
            })
        } else {
            self.expect(&TokenKind::Semicolon)?;
            Ok(Stmt::Expression { expr, span })
        }
    }

    // --- Expression parsing (by precedence) ---

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;

        while self.check(&TokenKind::Or) {
            let span_start = left.span();
            self.advance();
            let right = self.parse_and()?;
            let span = Span {
                line: span_start.line,
                column: span_start.column,
                start: span_start.start,
                end: right.span().end,
            };
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_equality()?;

        while self.check(&TokenKind::And) {
            let span_start = left.span();
            self.advance();
            let right = self.parse_equality()?;
            let span = Span {
                line: span_start.line,
                column: span_start.column,
                start: span_start.start,
                end: right.span().end,
            };
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_comparison()?;

        loop {
            let op = match self.peek_kind() {
                TokenKind::EqualEqual => BinaryOp::Eq,
                TokenKind::BangEqual => BinaryOp::Neq,
                _ => break,
            };
            let span_start = left.span();
            self.advance();
            let right = self.parse_comparison()?;
            let span = Span {
                line: span_start.line,
                column: span_start.column,
                start: span_start.start,
                end: right.span().end,
            };
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_addition()?;

        loop {
            let op = match self.peek_kind() {
                TokenKind::Less => BinaryOp::Lt,
                TokenKind::LessEqual => BinaryOp::Lte,
                TokenKind::Greater => BinaryOp::Gt,
                TokenKind::GreaterEqual => BinaryOp::Gte,
                _ => break,
            };
            let span_start = left.span();
            self.advance();
            let right = self.parse_addition()?;
            let span = Span {
                line: span_start.line,
                column: span_start.column,
                start: span_start.start,
                end: right.span().end,
            };
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_addition(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_multiplication()?;

        loop {
            let op = match self.peek_kind() {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            let span_start = left.span();
            self.advance();
            let right = self.parse_multiplication()?;
            let span = Span {
                line: span_start.line,
                column: span_start.column,
                start: span_start.start,
                end: right.span().end,
            };
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_multiplication(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match self.peek_kind() {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Mod,
                _ => break,
            };
            let span_start = left.span();
            self.advance();
            let right = self.parse_unary()?;
            let span = Span {
                line: span_start.line,
                column: span_start.column,
                start: span_start.start,
                end: right.span().end,
            };
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        match self.peek_kind() {
            TokenKind::Minus => {
                let token = self.advance();
                let operand = self.parse_unary()?;
                let span = Span {
                    line: token.span.line,
                    column: token.span.column,
                    start: token.span.start,
                    end: operand.span().end,
                };
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                    span,
                })
            }
            TokenKind::Bang => {
                let token = self.advance();
                let operand = self.parse_unary()?;
                let span = Span {
                    line: token.span.line,
                    column: token.span.column,
                    start: token.span.start,
                    end: operand.span().end,
                };
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                    span,
                })
            }
            _ => self.parse_call(),
        }
    }

    fn parse_call(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.check(&TokenKind::LeftParen) {
                // Function call — only valid on identifiers
                if let Expr::Identifier {
                    name,
                    span: id_span,
                } = expr
                {
                    self.advance(); // consume '('
                    let mut args = Vec::new();
                    if !self.check(&TokenKind::RightParen) {
                        loop {
                            args.push(self.parse_expression()?);
                            if !self.match_token(&TokenKind::Comma) {
                                break;
                            }
                        }
                    }
                    let close = self.expect(&TokenKind::RightParen)?;
                    let span = Span {
                        line: id_span.line,
                        column: id_span.column,
                        start: id_span.start,
                        end: close.span.end,
                    };
                    expr = Expr::FnCall {
                        callee: name,
                        args,
                        span,
                    };
                } else {
                    break;
                }
            } else if self.check(&TokenKind::Dot) {
                self.advance(); // consume '.'
                let (field, field_span) = self.expect_ident()?;
                let span = Span {
                    line: expr.span().line,
                    column: expr.span().column,
                    start: expr.span().start,
                    end: field_span.end,
                };
                expr = Expr::FieldAccess {
                    object: Box::new(expr),
                    field,
                    span,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.peek().clone();

        match &token.kind {
            TokenKind::IntLiteral(v) => {
                let v = *v;
                self.advance();
                Ok(Expr::IntLiteral {
                    value: v,
                    span: token.span,
                })
            }
            TokenKind::FloatLiteral(v) => {
                let v = *v;
                self.advance();
                Ok(Expr::FloatLiteral {
                    value: v,
                    span: token.span,
                })
            }
            TokenKind::StringLiteral(v) => {
                let v = v.clone();
                self.advance();
                Ok(Expr::StringLiteral {
                    value: v,
                    span: token.span,
                })
            }
            TokenKind::BoolLiteral(v) => {
                let v = *v;
                self.advance();
                Ok(Expr::BoolLiteral {
                    value: v,
                    span: token.span,
                })
            }
            TokenKind::LeftParen => {
                self.advance(); // consume '('
                let expr = self.parse_expression()?;
                let close = self.expect(&TokenKind::RightParen)?;
                let span = Span {
                    line: token.span.line,
                    column: token.span.column,
                    start: token.span.start,
                    end: close.span.end,
                };
                Ok(Expr::Grouped {
                    expr: Box::new(expr),
                    span,
                })
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();

                // Check for struct initialization: Name { field: value, ... }
                if self.check(&TokenKind::LeftBrace) {
                    // Look ahead to see if this is a struct init (Name { ident : ...)
                    // vs a block. We check: { <ident> :
                    if self.is_struct_init() {
                        self.advance(); // consume '{'
                        let mut fields = Vec::new();
                        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
                            let (fname, _) = self.expect_ident()?;
                            self.expect(&TokenKind::Colon)?;
                            let value = self.parse_expression()?;
                            fields.push((fname, value));

                            if !self.match_token(&TokenKind::Comma) {
                                break;
                            }
                        }
                        let close = self.expect(&TokenKind::RightBrace)?;
                        let span = Span {
                            line: token.span.line,
                            column: token.span.column,
                            start: token.span.start,
                            end: close.span.end,
                        };
                        return Ok(Expr::StructInit { name, fields, span });
                    }
                }

                Ok(Expr::Identifier {
                    name,
                    span: token.span,
                })
            }
            TokenKind::Hash => {
                self.advance(); // consume '#'
                self.parse_sql_expr(token.span)
            }
            _ => Err(ParseError {
                message: format!("expected expression, found '{}'", token.kind),
                expected: "expression".to_string(),
                found: format!("{}", token.kind),
                span: token.span,
            }),
        }
    }

    fn parse_sql_expr(&mut self, hash_span: Span) -> Result<Expr, ParseError> {
        match self.peek_kind() {
            TokenKind::Select => self.parse_sql_select(hash_span),
            TokenKind::Insert => self.parse_sql_insert(hash_span),
            _ => Err(ParseError {
                message: format!(
                    "expected SELECT or INSERT after '#', found '{}'",
                    self.peek_kind()
                ),
                expected: "SELECT or INSERT".to_string(),
                found: format!("{}", self.peek_kind()),
                span: self.peek().span,
            }),
        }
    }

    fn parse_sql_select(&mut self, hash_span: Span) -> Result<Expr, ParseError> {
        self.advance(); // consume SELECT

        // Check for SINGLE
        let single = self.match_token(&TokenKind::Single);

        // Parse columns: col1, col2, ... or *
        let columns = self.parse_sql_columns()?;

        // Expect FROM
        self.expect(&TokenKind::From)?;

        // Table reference: alias or csv("file")
        let table_ref = self.parse_sql_table_ref()?;

        // Optional WHERE clause
        let condition = if self.match_token(&TokenKind::Where) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        let span = Span {
            line: hash_span.line,
            column: hash_span.column,
            start: hash_span.start,
            end: self.tokens[self.pos.saturating_sub(1)].span.end,
        };

        Ok(Expr::SqlSelect {
            columns,
            table_ref,
            condition,
            single,
            span,
        })
    }

    fn parse_sql_columns(&mut self) -> Result<Vec<String>, ParseError> {
        let mut columns = Vec::new();

        // Check for * (all columns)
        if self.check(&TokenKind::Star) {
            self.advance();
            columns.push("*".to_string());
            return Ok(columns);
        }

        // Parse comma-separated column names
        loop {
            let (col, _) = self.expect_ident()?;
            columns.push(col);
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        Ok(columns)
    }

    fn parse_sql_insert(&mut self, hash_span: Span) -> Result<Expr, ParseError> {
        self.advance(); // consume INSERT

        // Expect INTO
        self.expect(&TokenKind::Into)?;

        // Table reference: alias or csv("file")
        let table_ref = self.parse_sql_table_ref()?;

        // Expect VALUES
        self.expect(&TokenKind::Values)?;

        // Expect ( expr, expr, ... )
        self.expect(&TokenKind::LeftParen)?;
        let mut values = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            loop {
                values.push(self.parse_expression()?);
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }
        let close = self.expect(&TokenKind::RightParen)?;

        let span = Span {
            line: hash_span.line,
            column: hash_span.column,
            start: hash_span.start,
            end: close.span.end,
        };

        Ok(Expr::SqlInsert {
            table_ref,
            values,
            span,
        })
    }

    fn parse_sql_table_ref(&mut self) -> Result<SqlTableRef, ParseError> {
        // Check for file("file.csv")
        if self.check(&TokenKind::File) {
            self.advance(); // consume 'file'
            self.expect(&TokenKind::LeftParen)?;
            let file_path = match self.peek_kind() {
                TokenKind::StringLiteral(_) => {
                    let token = self.advance();
                    if let TokenKind::StringLiteral(s) = token.kind {
                        s
                    } else {
                        unreachable!()
                    }
                }
                _ => {
                    return Err(ParseError {
                        message: format!(
                            "expected file path string in file(), found '{}'",
                            self.peek_kind()
                        ),
                        expected: "string literal".to_string(),
                        found: format!("{}", self.peek_kind()),
                        span: self.peek().span,
                    });
                }
            };
            self.expect(&TokenKind::RightParen)?;
            Ok(SqlTableRef::Inline(file_path))
        } else {
            // Alias (identifier)
            let (alias, _) = self.expect_ident()?;
            Ok(SqlTableRef::Alias(alias))
        }
    }

    /// Look ahead to determine if `{` starts a struct init (Ident { field: ... })
    fn is_struct_init(&self) -> bool {
        // Current token is `{`, check if pattern is `{ <ident> :`
        if self.pos + 2 < self.tokens.len() {
            matches!(self.tokens[self.pos + 1].kind, TokenKind::Ident(_))
                && matches!(self.tokens[self.pos + 2].kind, TokenKind::Colon)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(input: &str) -> Program {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse().unwrap()
    }

    #[test]
    fn test_let_statement() {
        let program = parse("let x: int = 42;");
        assert_eq!(program.declarations.len(), 1);
        match &program.declarations[0] {
            Declaration::Statement(Stmt::Let { name, mutable, .. }) => {
                assert_eq!(name, "x");
                assert!(!mutable);
            }
            _ => panic!("Expected Let statement"),
        }
    }

    #[test]
    fn test_let_mut() {
        let program = parse("let mut x: int = 0;");
        match &program.declarations[0] {
            Declaration::Statement(Stmt::Let { mutable, .. }) => assert!(mutable),
            _ => panic!("Expected Let statement"),
        }
    }

    #[test]
    fn test_function_decl() {
        let program = parse("fn add(a: int, b: int) -> int { return a; }");
        match &program.declarations[0] {
            Declaration::Function(f) => {
                assert_eq!(f.name, "add");
                assert_eq!(f.params.len(), 2);
                assert_eq!(f.return_type, TypeAnnotation::Int);
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_struct_decl() {
        let program = parse("struct Point { x: float, y: float }");
        match &program.declarations[0] {
            Declaration::Struct(s) => {
                assert_eq!(s.name, "Point");
                assert_eq!(s.fields.len(), 2);
            }
            _ => panic!("Expected struct"),
        }
    }

    #[test]
    fn test_if_else() {
        let program = parse("fn main() { if true { print(1); } else { print(2); } }");
        assert_eq!(program.declarations.len(), 1);
    }

    #[test]
    fn test_for_loop() {
        let program = parse("fn main() { for i in 0..10 { print(i); } }");
        assert_eq!(program.declarations.len(), 1);
    }

    #[test]
    fn test_while_loop() {
        let program = parse("fn main() { let mut x: int = 5; while x > 0 { x = x - 1; } }");
        assert_eq!(program.declarations.len(), 1);
    }

    #[test]
    fn test_binary_ops_precedence() {
        let program = parse("fn main() { let x: int = 1 + 2 * 3; }");
        match &program.declarations[0] {
            Declaration::Function(f) => {
                match &f.body.statements[0] {
                    Stmt::Let { initializer, .. } => {
                        // Should be Add(1, Mul(2, 3)) not Mul(Add(1,2), 3)
                        match initializer {
                            Expr::BinaryOp {
                                op: BinaryOp::Add,
                                right,
                                ..
                            } => {
                                assert!(matches!(
                                    **right,
                                    Expr::BinaryOp {
                                        op: BinaryOp::Mul,
                                        ..
                                    }
                                ));
                            }
                            _ => panic!("Expected Add at top level"),
                        }
                    }
                    _ => panic!("Expected Let"),
                }
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_struct_init() {
        let program = parse("fn main() { let p: Point = Point { x: 1.0, y: 2.0 }; }");
        assert_eq!(program.declarations.len(), 1);
    }

    #[test]
    fn test_field_access() {
        let program = parse("fn main() { let x: float = p.x; }");
        assert_eq!(program.declarations.len(), 1);
    }

    #[test]
    fn test_fn_call() {
        let program = parse("fn main() { let x: int = add(1, 2); }");
        assert_eq!(program.declarations.len(), 1);
    }

    #[test]
    fn test_nested_expressions() {
        let program = parse("fn main() { let x: bool = (1 + 2) * 3 == 9 && true; }");
        assert_eq!(program.declarations.len(), 1);
    }

    #[test]
    fn test_unary_ops() {
        let program = parse("fn main() { let x: int = -5; let y: bool = !true; }");
        assert_eq!(program.declarations.len(), 1);
    }
}
