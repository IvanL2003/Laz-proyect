use std::collections::{HashMap, HashSet};
use std::path::Path;
use crate::lexer::ConnectType;
use crate::parser::ast::*;
use crate::utils::error::{SemanticError, SemanticWarning};

pub struct TypeChecker {
    functions: HashMap<String, usize>,     // name -> param count
    structs: HashMap<String, Vec<String>>, // name -> field names
    enums: HashMap<String, Vec<String>>,   // name -> variant names
    errors: Vec<SemanticError>,
    warnings: Vec<SemanticWarning>,
}

impl TypeChecker {
    /// `base_dir` is needed to resolve relative SQLite paths in `connect db`.
    /// Returns Ok(warnings) on success, Err(errors) on failure.
    pub fn check(program: &Program, base_dir: &Path) -> Result<Vec<SemanticWarning>, Vec<SemanticError>> {
        let mut checker = TypeChecker {
            functions: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        checker.collect_declarations(program, base_dir);
        checker.validate_program(program);

        if checker.errors.is_empty() {
            Ok(checker.warnings)
        } else {
            Err(checker.errors)
        }
    }

    fn collect_declarations(&mut self, program: &Program, base_dir: &Path) {
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
                Declaration::Enum(e) => {
                    self.enums.insert(e.name.clone(), e.variants.clone());
                }
                Declaration::Connect(c) => {
                    // Pre-pass for `connect db`: read SQLite schema and register structs
                    // so the type checker knows their fields without the user declaring them.
                    if matches!(c.connect_type, ConnectType::Db) && !c.mappings.is_empty() {
                        let db_path = base_dir.join(&c.file_path);
                        let db_path_str = db_path.to_string_lossy();
                        match crate::utils::sqlite::read_schema(&db_path_str) {
                            Ok(schema) => {
                                for mapping in &c.mappings {
                                    if let Some(cols) = schema.get(&mapping.table_name) {
                                        let field_names: Vec<String> =
                                            cols.iter().map(|(name, _)| name.clone()).collect();
                                        self.structs.insert(mapping.struct_name.clone(), field_names);
                                    }
                                }
                            }
                            Err(_) => {
                                // DB file not found at analysis time (e.g. tests without .db files).
                                // Runtime will report a proper error when the DB is opened.
                            }
                        }
                    }
                }
                Declaration::Statement(_) => {}
                Declaration::Import { .. } => {}
            }
        }
    }

    fn validate_program(&mut self, program: &Program) {
        for decl in &program.declarations {
            match decl {
                Declaration::Function(f) => self.validate_block(&f.body, true),
                Declaration::Struct(_) => {}
                Declaration::Enum(_) => {}
                Declaration::Connect(_) => {}
                Declaration::Statement(stmt) => self.validate_stmt(stmt, false),
                Declaration::Import { .. } => {} // el interprete maneja la carga
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
                if let Some(ta) = type_ann {
                    self.validate_type(ta);
                }
                self.validate_expr(initializer);
            }
            Stmt::Assign { value, .. } => {
                self.validate_expr(value);
            }
            Stmt::If { condition, then_block, else_block, .. } => {
                self.validate_expr(condition);
                self.validate_block(then_block, in_function);
                // else_block puede ser un else normal o un else-if desazucarado (else { if ... })
                // En ambos casos basta con validar el bloque recursivamente
                if let Some(block) = else_block {
                    self.validate_block(block, in_function);
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
            Stmt::ForEach { iterable, body, .. } => {
                self.validate_expr(iterable);
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
            Stmt::Match { subject, arms, span } => {
                self.validate_expr(subject);
                // Warn if no catch-all arm (Wildcard '_' or named binding like `x`)
                let has_catchall = arms.iter().any(|arm| {
                    matches!(arm.pattern, Pattern::Wildcard | Pattern::Ident(_))
                });
                if !has_catchall {
                    self.warnings.push(SemanticWarning {
                        message: "match has no wildcard arm '_'; unmatched values will panic at runtime".to_string(),
                        span: *span,
                    });
                }
                for arm in arms {
                    self.validate_block(&arm.body, in_function);
                }
            }
        }
    }

    fn validate_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::FnCall { callee, args, span } => {
                const BUILTINS: &[(&str, usize)] = &[
                    // Tipo
                    ("typeOf", 1),
                    // Listas / Colecciones
                    ("len", 1), ("pop", 1),
                    ("sort", 1), ("first", 1), ("last", 1),
                    ("concat", 2), ("zip", 2), ("unzip", 1),
                    ("reverse", 1), ("slice", 3), ("contains", 2),
                    ("join", 2), ("range", 2),
                    // HOFs
                    ("map", 2), ("filter", 2), ("reduce", 3), ("sortBy", 2),
                    // Conversión
                    ("toString", 1), ("parseInt", 1), ("toFloat", 1),
                    // Strings
                    ("trim", 1), ("lower", 1), ("upper", 1),
                    ("replace", 3), ("split", 2), ("substring", 3),
                    ("startsWith", 2), ("endsWith", 2),
                    ("indexOf", 2), ("lastIndexOf", 2),
                    // Math
                    ("abs", 1), ("sqrt", 1), ("floor", 1), ("ceil", 1),
                    ("pow", 2), ("powf", 2), ("min", 2), ("max", 2),
                    // Result / Option
                    ("ok", 1), ("err", 1), ("some", 1), ("none", 0),
                    ("unwrap", 1), ("is_ok", 1), ("is_err", 1), ("is_some", 1), ("is_none", 1),
                    // Dict
                    ("keys", 1), ("values", 1), ("get", 2), ("containsKey", 2), ("remove", 2),
                    // push tiene aridad variable (2 o 3), se valida en runtime
                ];
                let builtin = BUILTINS.iter().find(|(name, _)| *name == callee.as_str());
                // push es especial: 2 args (lista) o 3 args (dict)
                if callee == "push" {
                    if args.len() != 2 && args.len() != 3 {
                        self.errors.push(SemanticError {
                            message: format!("built-in 'push' expects 2 or 3 arguments, got {}", args.len()),
                            span: *span,
                        });
                    }
                    for arg in args { self.validate_expr(arg); }
                    return;
                }

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
                } else if let Some(&(_, expected)) = builtin {
                    if args.len() != expected {
                        self.errors.push(SemanticError {
                            message: format!(
                                "built-in '{}' expects {} argument(s), got {}",
                                callee, expected, args.len()
                            ),
                            span: *span,
                        });
                    }
                }
                // Si no se reconoce, podría ser una variable que contiene un Value::Func
                // (lambda asignada a variable). El runtime lo validará.
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
            Expr::ListLiteral { elements, .. } => {
                for elem in elements {
                    self.validate_expr(elem);
                }
            }
            Expr::Index { object, index, .. } => {
                self.validate_expr(object);
                self.validate_expr(index);
            }
            Expr::Lambda { body, .. } => {
                for stmt in &body.statements {
                    self.validate_stmt(stmt, true);
                }
            }
            Expr::Try { expr, .. } => {
                self.validate_expr(expr);
            }
            Expr::FString { parts, .. } => {
                for part in parts {
                    if let AstFStringPart::Expr(expr) = part {
                        self.validate_expr(expr);
                    }
                }
            }
            Expr::EnumVariant { enum_name, variant, span } => {
                if let Some(variants) = self.enums.get(enum_name) {
                    if !variants.contains(variant) {
                        self.errors.push(SemanticError {
                            message: format!("enum '{}' has no variant '{}'", enum_name, variant),
                            span: *span,
                        });
                    }
                } else {
                    self.errors.push(SemanticError {
                        message: format!("undefined enum '{}'", enum_name),
                        span: *span,
                    });
                }
            }
            Expr::DictLiteral { entries, .. } => {
                for (k, v) in entries {
                    self.validate_expr(k);
                    self.validate_expr(v);
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
            TypeAnnotation::Dict(key_type, value_type) => {
                self.validate_type(key_type);
                self.validate_type(value_type);
            }
            TypeAnnotation::Result(ok_t, err_t) => {
                self.validate_type(ok_t);
                self.validate_type(err_t);
            }
            TypeAnnotation::Option(inner) => {
                self.validate_type(inner);
            }
            _ => {}
        }
    }
}
