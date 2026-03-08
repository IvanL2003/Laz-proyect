use crate::lexer::{Comment, Span};
use crate::parser::ast::*;

const INDENT: &str = "    "; // 4 spaces

pub struct Formatter {
    comments: Vec<Comment>,
    output: String,
    indent_level: usize,
    next_comment_idx: usize,
}

impl Formatter {
    pub fn new(mut comments: Vec<Comment>) -> Self {
        comments.sort_by_key(|c| c.line);
        Formatter {
            comments,
            output: String::new(),
            indent_level: 0,
            next_comment_idx: 0,
        }
    }

    pub fn format(mut self, program: &Program) -> String {
        self.format_program(program);
        self.emit_remaining_comments();
        // Ensure file ends with exactly one newline
        let result = self.output.trim_end().to_string();
        result + "\n"
    }

    // --- Indentation ---

    fn indent(&self) -> String {
        INDENT.repeat(self.indent_level)
    }

    // --- Comment helpers ---

    fn emit_leading_comments(&mut self, before_line: usize) {
        while self.next_comment_idx < self.comments.len() {
            let comment = &self.comments[self.next_comment_idx];
            if comment.line < before_line && !comment.is_inline {
                let indent = self.indent();
                self.output.push_str(&format!("{}// {}\n", indent, comment.text));
                self.next_comment_idx += 1;
            } else {
                break;
            }
        }
    }

    fn emit_inline_comment(&mut self, on_line: usize) {
        if self.next_comment_idx < self.comments.len() {
            let comment = &self.comments[self.next_comment_idx];
            if comment.line == on_line && comment.is_inline {
                // Remove trailing newline, append comment, re-add newline
                if self.output.ends_with('\n') {
                    self.output.pop();
                }
                self.output.push_str(&format!(" // {}\n", comment.text));
                self.next_comment_idx += 1;
            }
        }
    }

    fn emit_remaining_comments(&mut self) {
        while self.next_comment_idx < self.comments.len() {
            let comment = &self.comments[self.next_comment_idx];
            let indent = self.indent();
            if comment.is_inline {
                // Trailing inline comment at end of file
                if self.output.ends_with('\n') {
                    self.output.pop();
                }
                self.output.push_str(&format!(" // {}\n", comment.text));
            } else {
                self.output.push_str(&format!("{}// {}\n", indent, comment.text));
            }
            self.next_comment_idx += 1;
        }
    }

    // --- Program & Declarations ---

    fn format_program(&mut self, program: &Program) {
        let mut prev_decl_kind: Option<&str> = None;
        let decl_count = program.declarations.len();

        for (i, decl) in program.declarations.iter().enumerate() {
            let decl_line = self.decl_span(decl).line;

            // Determine next declaration's line for comment boundary
            let next_decl_line = if i + 1 < decl_count {
                self.decl_span(&program.declarations[i + 1]).line
            } else {
                usize::MAX
            };

            // Add blank line between different declaration kinds
            let current_kind = match decl {
                Declaration::Function(_) => "fn",
                Declaration::Struct(_) => "struct",
                Declaration::Connect(_) => "connect",
                Declaration::Statement(_) => "stmt",
                Declaration::Import { .. } => "import",
            };
            if let Some(prev) = prev_decl_kind {
                if prev != current_kind || current_kind == "fn" {
                    self.output.push('\n');
                }
            }
            prev_decl_kind = Some(current_kind);

            self.emit_leading_comments(decl_line);
            self.format_declaration(decl, next_decl_line);
        }
    }

    fn decl_span(&self, decl: &Declaration) -> Span {
        match decl {
            Declaration::Function(f) => f.span,
            Declaration::Struct(s) => s.span,
            Declaration::Connect(c) => c.span,
            Declaration::Statement(s) => self.stmt_span(s),
            Declaration::Import { span, .. } => *span,
        }
    }

    fn format_declaration(&mut self, decl: &Declaration, next_decl_line: usize) {
        match decl {
            Declaration::Function(f) => self.format_fn_decl(f, next_decl_line),
            Declaration::Struct(s) => self.format_struct_decl(s, next_decl_line),
            Declaration::Connect(c) => self.format_connect(c),
            Declaration::Statement(s) => self.format_statement(s),
            Declaration::Import { path, span } => {
                self.output.push_str(&format!("{}import \"{}\";\n", self.indent(), path));
                self.emit_inline_comment(span.line);
            }
        }
    }

    fn format_connect(&mut self, decl: &ConnectDecl) {
        let connect_keyword = match decl.connect_type {
            crate::lexer::token::ConnectType::File => "file",
            crate::lexer::token::ConnectType::Db => "db",
            crate::lexer::token::ConnectType::Api => "api",
        };
        self.output.push_str(&format!(
            "{}connect {} \"{}\" as {};\n",
            self.indent(),
            connect_keyword,
            decl.file_path,
            decl.alias
        ));
        self.emit_inline_comment(decl.span.line);
    }

    fn format_struct_decl(&mut self, decl: &StructDecl, next_decl_line: usize) {
        self.output.push_str(&format!(
            "{}struct {} {{\n",
            self.indent(),
            decl.name
        ));
        self.indent_level += 1;

        for field in &decl.fields {
            self.emit_leading_comments(field.span.line);
            self.output.push_str(&format!(
                "{}{}: {},\n",
                self.indent(),
                field.name,
                self.format_type(&field.type_ann)
            ));
            self.emit_inline_comment(field.span.line);
        }

        self.indent_level -= 1;
        self.output.push_str(&format!("{}}}\n", self.indent()));
        // Don't consume comments that belong to the next declaration
        let _ = next_decl_line;
    }

    fn format_fn_decl(&mut self, decl: &FnDecl, next_decl_line: usize) {
        let params = decl
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, self.format_type(&p.type_ann)))
            .collect::<Vec<_>>()
            .join(", ");

        let return_type = format!(" -> {}", self.format_type(&decl.return_type));

        self.output.push_str(&format!(
            "{}fn {}({}){} {{\n",
            self.indent(),
            decl.name,
            params,
            return_type
        ));

        self.indent_level += 1;
        // Use next_decl_line as the boundary - comments after the last statement
        // but before next_decl_line belong inside this function body
        self.format_block_contents(&decl.body, next_decl_line);
        self.indent_level -= 1;

        self.output.push_str(&format!("{}}}\n", self.indent()));
    }

    // --- Type formatting ---

    fn format_type(&self, type_ann: &TypeAnnotation) -> String {
        match type_ann {
            TypeAnnotation::Int => "int".to_string(),
            TypeAnnotation::Float => "float".to_string(),
            TypeAnnotation::Bool => "bool".to_string(),
            TypeAnnotation::StringType => "string".to_string(),
            TypeAnnotation::Void => "void".to_string(),
            TypeAnnotation::List(inner) => format!("list<{}>", self.format_type(inner)),
            TypeAnnotation::Result(ok_t, err_t) => {
                format!("Result<{}, {}>", self.format_type(ok_t), self.format_type(err_t))
            }
            TypeAnnotation::Option(inner) => format!("Option<{}>", self.format_type(inner)),
            TypeAnnotation::UserDefined(name) => name.clone(),
        }
    }

    // --- Block ---

    fn format_block_contents(&mut self, block: &Block, next_sibling_line: usize) {
        for stmt in &block.statements {
            self.format_statement(stmt);
        }
        // Emit comments that are inside this block (after last stmt, before closing brace).
        // The closing brace line is estimated as: if there are statements, check if
        // the next comment's line is less than the next sibling declaration's line.
        // We use a conservative approach: only consume comments whose line is less than
        // next_sibling_line AND whose line is reasonably close to the last statement.
        if let Some(last_stmt) = block.statements.last() {
            let last_line = self.stmt_span(last_stmt).line;
            // Only consume comments that are between last statement and next sibling
            // AND are within a reasonable range (before any blank line gap to next decl)
            while self.next_comment_idx < self.comments.len() {
                let comment = &self.comments[self.next_comment_idx];
                // Comment must be after last statement and before the next declaration
                if comment.line > last_line && comment.line < next_sibling_line && !comment.is_inline {
                    // Heuristic: if the comment is closer to the last statement than to
                    // the next declaration, it belongs inside this block
                    let dist_to_last = comment.line - last_line;
                    let dist_to_next = next_sibling_line.saturating_sub(comment.line);
                    if dist_to_last <= dist_to_next {
                        let indent = self.indent();
                        self.output.push_str(&format!("{}// {}\n", indent, comment.text));
                        self.next_comment_idx += 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
    }

    // --- Statements ---

    fn format_statement(&mut self, stmt: &Stmt) {
        let stmt_line = self.stmt_span(stmt).line;
        self.emit_leading_comments(stmt_line);

        match stmt {
            Stmt::Let {
                name,
                mutable,
                type_ann,
                initializer,
                span,
            } => {
                let mut_kw = if *mutable { "mut " } else { "" };
                let type_part = match type_ann {
                    Some(ta) => format!(": {}", self.format_type(ta)),
                    None => String::new(),
                };
                self.output.push_str(&format!(
                    "{}let {}{}{} = {};\n",
                    self.indent(),
                    mut_kw,
                    name,
                    type_part,
                    self.format_expr(initializer)
                ));
                self.emit_inline_comment(span.line);
            }

            Stmt::Assign {
                target,
                value,
                span,
            } => {
                let target_str = match target {
                    AssignTarget::Variable(name) => name.clone(),
                    AssignTarget::FieldAccess { object, field } => {
                        format!("{}.{}", self.format_expr(object), field)
                    }
                    AssignTarget::Index { object, index } => {
                        format!("{}[{}]", object, self.format_expr(index))
                    }
                };
                self.output.push_str(&format!(
                    "{}{} = {};\n",
                    self.indent(),
                    target_str,
                    self.format_expr(value)
                ));
                self.emit_inline_comment(span.line);
            }

            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.output.push_str(&format!(
                    "{}if {} {{\n",
                    self.indent(),
                    self.format_expr(condition)
                ));
                self.indent_level += 1;
                self.format_block_inner(then_block);
                self.indent_level -= 1;
                self.format_else_block(else_block.as_ref());
            }

            Stmt::While {
                condition,
                body,
                ..
            } => {
                self.output.push_str(&format!(
                    "{}while {} {{\n",
                    self.indent(),
                    self.format_expr(condition)
                ));
                self.indent_level += 1;
                self.format_block_inner(body);
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
            }

            Stmt::For {
                variable,
                start,
                end,
                body,
                ..
            } => {
                self.output.push_str(&format!(
                    "{}for {} in {}..{} {{\n",
                    self.indent(),
                    variable,
                    self.format_expr(start),
                    self.format_expr(end)
                ));
                self.indent_level += 1;
                self.format_block_inner(body);
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
            }

            Stmt::Return { value, span } => {
                match value {
                    Some(expr) => {
                        self.output.push_str(&format!(
                            "{}return {};\n",
                            self.indent(),
                            self.format_expr(expr)
                        ));
                    }
                    None => {
                        self.output
                            .push_str(&format!("{}return;\n", self.indent()));
                    }
                }
                self.emit_inline_comment(span.line);
            }

            Stmt::Print { args, span } => {
                let args_str = args
                    .iter()
                    .map(|a| self.format_expr(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.output.push_str(&format!(
                    "{}print({});\n",
                    self.indent(),
                    args_str
                ));
                self.emit_inline_comment(span.line);
            }

            Stmt::Expression { expr, span } => {
                self.output.push_str(&format!(
                    "{}{};\n",
                    self.indent(),
                    self.format_expr(expr)
                ));
                self.emit_inline_comment(span.line);
            }

            Stmt::Match { subject, arms, span } => {
                self.output.push_str(&format!(
                    "{}match {} {{\n",
                    self.indent(),
                    self.format_expr(subject)
                ));
                self.indent_level += 1;
                for arm in arms {
                    let pattern_str = self.format_pattern(&arm.pattern);
                    self.output.push_str(&format!("{}{} => {{\n", self.indent(), pattern_str));
                    self.indent_level += 1;
                    self.format_block_inner(&arm.body);
                    self.indent_level -= 1;
                    self.output.push_str(&format!("{}}}\n", self.indent()));
                }
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
                self.emit_inline_comment(span.line);
            }
        }
    }

    fn format_block_inner(&mut self, block: &Block) {
        for stmt in &block.statements {
            self.format_statement(stmt);
        }
    }

    fn stmt_span(&self, stmt: &Stmt) -> Span {
        match stmt {
            Stmt::Let { span, .. }
            | Stmt::Assign { span, .. }
            | Stmt::If { span, .. }
            | Stmt::While { span, .. }
            | Stmt::For { span, .. }
            | Stmt::Return { span, .. }
            | Stmt::Print { span, .. }
            | Stmt::Expression { span, .. }
            | Stmt::Match { span, .. } => *span,
        }
    }

    fn format_pattern(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Ok(bind)   => format!("ok({})", bind),
            Pattern::Err(bind)  => format!("err({})", bind),
            Pattern::Some(bind) => format!("some({})", bind),
            Pattern::None       => "none".to_string(),
            Pattern::Wildcard   => "_".to_string(),
            Pattern::Ident(name) => name.clone(),
        }
    }

    // Detecta si un else_block es un else-if desazucarado:
    //   Some(Block { statements: [Stmt::If { ... }] }) → reconstruye como "else if ..."
    //   Some(Block { statements: [...] })              → formatea como "else { ... }"
    //   None                                           → cierra el if con "}"
    fn format_else_block(&mut self, else_block: Option<&Block>) {
        match else_block {
            Some(block) => {
                // Detectar el patron else-if: bloque con un unico Stmt::If
                if block.statements.len() == 1 {
                    if let Stmt::If { condition, then_block, else_block: inner_else, .. }
                        = &block.statements[0]
                    {
                        // Reconstruir "else if cond { ... }"
                        self.output.push_str(&format!(
                            "{}}} else if {} {{\n",
                            self.indent(),
                            self.format_expr(condition)
                        ));
                        self.indent_level += 1;
                        self.format_block_inner(then_block);
                        self.indent_level -= 1;
                        // Continuar la cadena recursivamente
                        self.format_else_block(inner_else.as_ref());
                        return;
                    }
                }
                // else normal: bloque con varios statements o sin Stmt::If
                self.output.push_str(&format!("{}}} else {{\n", self.indent()));
                self.indent_level += 1;
                self.format_block_inner(block);
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", self.indent()));
            }
            None => {
                self.output.push_str(&format!("{}}}\n", self.indent()));
            }
        }
    }

    // --- Expressions ---

    fn format_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::IntLiteral { value, .. } => value.to_string(),
            Expr::FloatLiteral { value, .. } => {
                let s = value.to_string();
                if s.contains('.') {
                    s
                } else {
                    format!("{}.0", s)
                }
            }
            Expr::StringLiteral { value, .. } => {
                format!("\"{}\"", self.escape_string(value))
            }
            Expr::BoolLiteral { value, .. } => value.to_string(),
            Expr::Identifier { name, .. } => name.clone(),

            Expr::BinaryOp {
                left, op, right, ..
            } => {
                let op_str = match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    BinaryOp::Mod => "%",
                    BinaryOp::Eq => "==",
                    BinaryOp::Neq => "!=",
                    BinaryOp::Lt => "<",
                    BinaryOp::Lte => "<=",
                    BinaryOp::Gt => ">",
                    BinaryOp::Gte => ">=",
                    BinaryOp::And => "&&",
                    BinaryOp::Or => "||",
                };
                format!(
                    "{} {} {}",
                    self.format_expr(left),
                    op_str,
                    self.format_expr(right)
                )
            }

            Expr::UnaryOp { op, operand, .. } => {
                let op_str = match op {
                    UnaryOp::Neg => "-",
                    UnaryOp::Not => "!",
                };
                format!("{}{}", op_str, self.format_expr(operand))
            }

            Expr::FnCall { callee, args, .. } => {
                let args_str = args
                    .iter()
                    .map(|a| self.format_expr(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", callee, args_str)
            }

            Expr::FieldAccess { object, field, .. } => {
                format!("{}.{}", self.format_expr(object), field)
            }

            Expr::StructInit { name, fields, .. } => {
                let fields_str = fields
                    .iter()
                    .map(|(fname, fexpr)| format!("{}: {}", fname, self.format_expr(fexpr)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{} {{ {} }}", name, fields_str)
            }

            Expr::Grouped { expr, .. } => {
                format!("({})", self.format_expr(expr))
            }

            Expr::ListLiteral { elements, .. } => {
                let elems_str = elements
                    .iter()
                    .map(|e| self.format_expr(e))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", elems_str)
            }

            Expr::Index { object, index, .. } => {
                format!("{}[{}]", self.format_expr(object), self.format_expr(index))
            }

            Expr::Lambda { params, body, .. } => {
                let params_str = params.join(", ");
                // Forma compacta: cuerpo de un solo return -> |x| expr
                if body.statements.len() == 1 {
                    if let Stmt::Return { value: Some(expr), .. } = &body.statements[0] {
                        return format!("|{}| {}", params_str, self.format_expr(expr));
                    }
                }
                // Forma bloque: |x| { ... }
                let stmts: Vec<String> = body.statements.iter().map(|s| {
                    let mut f = Formatter::new(vec![]);
                    f.indent_level = self.indent_level + 1;
                    f.format_statement(s);
                    f.output
                }).collect();
                format!("|{}| {{\n{}{}}}", params_str, stmts.join(""), self.indent())
            }

            Expr::SqlSelect {
                columns,
                table_ref,
                condition,
                single,
                ..
            } => {
                let mut parts = vec!["#SELECT".to_string()];
                if *single {
                    parts.push("SINGLE".to_string());
                }
                parts.push(columns.join(", "));
                parts.push("FROM".to_string());
                parts.push(self.format_sql_table_ref(table_ref));
                if let Some(cond) = condition {
                    parts.push("WHERE".to_string());
                    parts.push(self.format_expr(cond));
                }
                parts.join(" ")
            }

            Expr::SqlInsert {
                table_ref, values, ..
            } => {
                let vals_str = values
                    .iter()
                    .map(|v| self.format_expr(v))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "#INSERT INTO {} VALUES ({})",
                    self.format_sql_table_ref(table_ref),
                    vals_str
                )
            }
        }
    }

    fn format_sql_table_ref(&self, table_ref: &SqlTableRef) -> String {
        match table_ref {
            SqlTableRef::Alias(alias) => alias.clone(),
            SqlTableRef::Inline(path) => format!("file(\"{}\")", path),
        }
    }

    fn escape_string(&self, s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\t', "\\t")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn format_code(input: &str) -> String {
        let mut lexer = Lexer::new(input);
        let (tokens, comments) = lexer.tokenize_with_comments().unwrap();
        let program = Parser::new(tokens).parse().unwrap();
        Formatter::new(comments).format(&program)
    }

    #[test]
    fn test_simple_let() {
        let result = format_code("fn main() -> void { let x: int = 5; }");
        assert!(result.contains("    let x: int = 5;"));
    }

    #[test]
    fn test_let_mut() {
        let result = format_code("fn main() -> void { let mut x: int = 5; }");
        assert!(result.contains("    let mut x: int = 5;"));
    }

    #[test]
    fn test_struct_formatting() {
        let result = format_code("struct Point { x: float, y: float, }");
        assert!(result.contains("struct Point {\n"));
        assert!(result.contains("    x: float,\n"));
        assert!(result.contains("    y: float,\n"));
        assert!(result.contains("}\n"));
    }

    #[test]
    fn test_function_formatting() {
        let result = format_code("fn add(a: int, b: int) -> int { return a + b; }");
        assert!(result.contains("fn add(a: int, b: int) -> int {\n"));
        assert!(result.contains("    return a + b;\n"));
        assert!(result.contains("}\n"));
    }

    #[test]
    fn test_if_else() {
        let result = format_code("fn main() -> void { if x > 0 { print(x); } else { print(0); } }");
        assert!(result.contains("    if x > 0 {\n"));
        assert!(result.contains("    } else {\n"));
    }

    #[test]
    fn test_for_loop() {
        let result = format_code("fn main() -> void { for i in 0..10 { print(i); } }");
        assert!(result.contains("    for i in 0..10 {\n"));
        assert!(result.contains("        print(i);\n"));
    }

    #[test]
    fn test_comment_preservation() {
        let input = "// Header comment\nfn main() -> void { let x: int = 5; }";
        let result = format_code(input);
        assert!(result.contains("// Header comment\n"));
    }

    #[test]
    fn test_inline_comment() {
        let input = "fn main() -> void {\n    let x: int = 5; // important\n}";
        let result = format_code(input);
        assert!(result.contains("let x: int = 5; // important\n"));
    }

    #[test]
    fn test_sql_select() {
        let input = "connect file \"users.csv\" as users;\nstruct User { name: string, age: int, }\nfn main() -> void { let all: list<User> = #SELECT * FROM users; }";
        let result = format_code(input);
        assert!(result.contains("#SELECT * FROM users"));
    }

    #[test]
    fn test_sql_select_inline() {
        let input = "struct User { name: string, }\nfn main() -> void { let all: list<User> = #SELECT * FROM file(\"users.csv\"); }";
        let result = format_code(input);
        assert!(result.contains("#SELECT * FROM file(\"users.csv\")"));
    }

    #[test]
    fn test_connect_file() {
        let result = format_code("connect file \"data.csv\" as data;");
        assert!(result.contains("connect file \"data.csv\" as data;\n"));
    }

    #[test]
    fn test_binary_ops_spacing() {
        let result = format_code("fn main() -> void { let x: int = 1 + 2 * 3; }");
        assert!(result.contains("1 + 2 * 3"));
    }

    #[test]
    fn test_while_loop() {
        let result = format_code("fn main() -> void { while x > 0 { x = x - 1; } }");
        assert!(result.contains("    while x > 0 {\n"));
    }

    #[test]
    fn test_struct_init() {
        let result = format_code("struct P { x: int, }\nfn main() -> void { let p: P = P { x: 1 }; }");
        assert!(result.contains("P { x: 1 }"));
    }

    #[test]
    fn test_idempotent() {
        let input = "fn main() -> void {\n    let x: int = 42;\n    print(x);\n}\n";
        let first = format_code(input);
        let second = format_code(&first);
        assert_eq!(first, second, "Formatter should be idempotent");
    }
}
