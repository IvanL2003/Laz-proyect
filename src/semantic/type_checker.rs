use std::collections::{HashMap, HashSet};
use crate::parser::ast::*;
use crate::utils::error::SemanticError;

pub struct TypeChecker {
    functions: HashMap<String, usize>, // name -> param count
    structs: HashMap<String, Vec<String>>, // name -> field names
    errors: Vec<SemanticError>,
}

impl TypeChecker {
    pub fn check(program: &Program) -> Result<(), Vec<SemanticError>> {
        let mut checker = TypeChecker {
            functions: HashMap::new(),
            structs: HashMap::new(),
            errors: Vec::new(),
        };

        checker.collect_declarations(program);
        checker.validate_program(program);

        if checker.errors.is_empty() {
            Ok(())
        } else {
            Err(checker.errors)
        }
    }

    fn collect_declarations(&mut self, program: &Program) {
        let mut seen_fns = HashSet::new();
        let mut seen_structs = HashSet::new();

        for decl in &program.declarations {
            match decl {
                Declaration::Function(f) => {
                    if !seen_fns.insert(f.name.clone()) {
                        self.errors.push(SemanticError {
                            message: format!("duplicate function '{}'", f.name),
                            span: f.span,
                        });
                    }
                    self.functions.insert(f.name.clone(), f.params.len());
                }
                Declaration::Struct(s) => {
                    if !seen_structs.insert(s.name.clone()) {
                        self.errors.push(SemanticError {
                            message: format!("duplicate struct '{}'", s.name),
                            span: s.span,
                        });
                    }
                    let field_names: Vec<String> = s.fields.iter().map(|f| f.name.clone()).collect();
                    self.structs.insert(s.name.clone(), field_names);
                }
                Declaration::Connect(_) => {}
                Declaration::Statement(_) => {}
            }
        }
    }

    fn validate_program(&mut self, program: &Program) {
        for decl in &program.declarations {
            match decl {
                Declaration::Function(f) => self.validate_block(&f.body, true),
                Declaration::Struct(_) => {}
                Declaration::Connect(_) => {}
                Declaration::Statement(stmt) => self.validate_stmt(stmt, false),
            }
        }
    }

    fn validate_block(&mut self, block: &Block, in_function: bool) {
        for stmt in &block.statements {
            self.validate_stmt(stmt, in_function);
        }
    }

    fn validate_stmt(&mut self, stmt: &Stmt, in_function: bool) {
        match stmt {
            Stmt::Let { type_ann, initializer, .. } => {
                self.validate_type(type_ann);
                self.validate_expr(initializer);
            }
            Stmt::Assign { value, .. } => {
                self.validate_expr(value);
            }
            Stmt::If { condition, then_block, else_branch, .. } => {
                self.validate_expr(condition);
                self.validate_block(then_block, in_function);
                if let Some(else_b) = else_branch {
                    match else_b.as_ref() {
                        ElseBranch::ElseBlock(block) => self.validate_block(block, in_function),
                        ElseBranch::ElseIf(if_stmt) => self.validate_stmt(if_stmt, in_function),
                    }
                }
            }
            Stmt::While { condition, body, .. } => {
                self.validate_expr(condition);
                self.validate_block(body, in_function);
            }
            Stmt::For { start, end, body, .. } => {
                self.validate_expr(start);
                self.validate_expr(end);
                self.validate_block(body, in_function);
            }
            Stmt::Return { value, span } => {
                if !in_function {
                    self.errors.push(SemanticError {
                        message: "return statement outside of function".to_string(),
                        span: *span,
                    });
                }
                if let Some(expr) = value {
                    self.validate_expr(expr);
                }
            }
            Stmt::Print { args, .. } => {
                for arg in args {
                    self.validate_expr(arg);
                }
            }
            Stmt::Expression { expr, .. } => {
                self.validate_expr(expr);
            }
        }
    }

    fn validate_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::FnCall { callee, args, span } => {
                if let Some(&expected_args) = self.functions.get(callee.as_str()) {
                    if args.len() != expected_args {
                        self.errors.push(SemanticError {
                            message: format!(
                                "function '{}' expects {} arguments, got {}",
                                callee, expected_args, args.len()
                            ),
                            span: *span,
                        });
                    }
                } else {
                    self.errors.push(SemanticError {
                        message: format!("undefined function '{}'", callee),
                        span: *span,
                    });
                }
                for arg in args {
                    self.validate_expr(arg);
                }
            }
            Expr::StructInit { name, fields, span } => {
                if let Some(expected_fields) = self.structs.get(name).cloned() {
                    let provided: HashSet<&str> = fields.iter().map(|(n, _)| n.as_str()).collect();
                    for ef in &expected_fields {
                        if !provided.contains(ef.as_str()) {
                            self.errors.push(SemanticError {
                                message: format!("missing field '{}' in struct '{}'", ef, name),
                                span: *span,
                            });
                        }
                    }
                    for (pf, _) in fields {
                        if !expected_fields.contains(pf) {
                            self.errors.push(SemanticError {
                                message: format!("unknown field '{}' in struct '{}'", pf, name),
                                span: *span,
                            });
                        }
                    }
                } else {
                    self.errors.push(SemanticError {
                        message: format!("undefined struct '{}'", name),
                        span: *span,
                    });
                }
                for (_, expr) in fields {
                    self.validate_expr(expr);
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                self.validate_expr(left);
                self.validate_expr(right);
            }
            Expr::UnaryOp { operand, .. } => {
                self.validate_expr(operand);
            }
            Expr::FieldAccess { object, .. } => {
                self.validate_expr(object);
            }
            Expr::Grouped { expr, .. } => {
                self.validate_expr(expr);
            }
            Expr::SqlSelect { condition, .. } => {
                if let Some(cond) = condition {
                    self.validate_expr(cond);
                }
            }
            Expr::SqlInsert { values, .. } => {
                for val in values {
                    self.validate_expr(val);
                }
            }
            // Literals and identifiers - nothing to validate at this stage
            _ => {}
        }
    }

    fn validate_type(&mut self, type_ann: &TypeAnnotation) {
        match type_ann {
            TypeAnnotation::UserDefined(name) => {
                if !self.structs.contains_key(name) {
                    // We don't have span info on TypeAnnotation directly,
                    // so we skip this check here (it'll be caught at runtime)
                }
            }
            TypeAnnotation::List(inner) => {
                self.validate_type(inner);
            }
            _ => {}
        }
    }
}
