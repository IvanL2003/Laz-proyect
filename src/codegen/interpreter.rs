// --- Añadir función nativa typeOf ---
fn native_type_of(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError {
            message: format!("typeOf espera 1 argumento, recibió {}", args.len()),
            span: Span { line: 0, column: 0, start: 0, end: 0 },
        });
    }
    let type_name = args[0].type_name().to_string();
    Ok(Value::Str(type_name))
}
use std::collections::HashMap;
use std::path::PathBuf;
use crate::lexer::Span;
use crate::parser::ast::*;
use crate::utils::error::RuntimeError;
use crate::utils::csv::DataTable;

// --- Runtime Values ---

// Valores posibles en tiempo de ejecucion.
// Cada variante corresponde a un tipo del lenguaje:
//   Int(i64)                             -->  int         ej: 42, -7
//   Float(f64)                           -->  float       ej: 3.14, 1.0
//   Bool(bool)                           -->  bool        ej: true, false
//   Str(String)                          -->  string      ej: "hello"
//   StructInstance { type_name, fields } -->  NombreStruct  ej: User { name: "Bob", age: 25 }
//   List(Vec<Value>)                     -->  list<T>     ej: [User{...}, User{...}]
//   Void                                 -->  void        (retorno de funciones sin valor)
#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    StructInstance {
        type_name: String,              // nombre del struct ej: "User", "Point"
        fields: HashMap<String, Value>, // HashMap campo -> valor
    },
    List(Vec<Value>),
    Void,
}

impl Value {
    // Devuelve el nombre del tipo como string (usado por typeOf y mensajes de error)
    // Int   --> "int"  |  Float --> "float"  |  Bool   --> "bool"
    // Str   --> "string"  |  List  --> "list"   |  Void   --> "void"
    // StructInstance --> nombre del struct ej: "User", "Point"
    fn type_name(&self) -> &str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::Str(_) => "string",
            Value::StructInstance { type_name, .. } => type_name,
            Value::List(_) => "list",
            Value::Void => "void",
        }
    }

    fn to_display_string(&self) -> String {
        match self {
            Value::Int(v) => v.to_string(),
            Value::Float(v) => {
                if *v == (*v as i64) as f64 && !v.is_nan() && !v.is_infinite() {
                    format!("{:.1}", v)
                } else {
                    v.to_string()
                }
            }
            Value::Bool(v) => v.to_string(),
            Value::Str(v) => v.clone(),
            Value::StructInstance { type_name, fields } => {
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_display_string()))
                    .collect();
                format!("{} {{ {} }}", type_name, field_strs.join(", "))
            }
            Value::List(items) => {
                let strs: Vec<String> = items.iter().map(|v| v.to_display_string()).collect();
                format!("[{}]", strs.join(", "))
            }
            Value::Void => "void".to_string(),
        }
    }
}

// --- Environment ---

// Una variable almacena su valor y si es mutable (let mut) o no (let)
struct Variable {
    value: Value,
    mutable: bool, // true si fue declarada con `let mut`
}

// Pila de scopes (HashMap) para manejar el alcance de variables.
// Estructura: [scope_global, scope_funcion, scope_bloque, ...]
//
// Ejemplo para este codigo:
//   let x = 1;          --> scope global: { x: Int(1) }
//   fn foo() {
//     let y = 2;        --> scope funcion: { y: Int(2) }
//     if true {
//       let z = 3;      --> scope bloque: { z: Int(3) }
//     }                 --> pop scope bloque
//   }                   --> pop scope funcion
//
// get() busca de mas interno a mas externo (shadowing natural)
// set() busca y verifica mutabilidad antes de modificar
// define() inserta en el scope mas interno (el ultimo)
struct Environment {
    scopes: Vec<HashMap<String, Variable>>,
}

impl Environment {
    fn new() -> Self {
        Environment {
            scopes: vec![HashMap::new()], // siempre hay al menos el scope global
        }
    }

    // Entra en un nuevo bloque/funcion
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    // Sale del bloque/funcion actual (destruye todas sus variables)
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    // Declara una nueva variable en el scope actual (let / let mut)
    fn define(&mut self, name: String, value: Value, mutable: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, Variable { value, mutable });
        }
    }

    // Busca una variable recorriendo scopes de mas interno a mas externo
    // Error si no existe en ningun scope
    fn get(&self, name: &str, span: Span) -> Result<Value, RuntimeError> {
        for scope in self.scopes.iter().rev() {
            if let Some(var) = scope.get(name) {
                return Ok(var.value.clone());
            }
        }
        Err(RuntimeError {
            message: format!("undefined variable '{}'", name),
            span,
        })
    }

    // Modifica una variable existente (solo si es mutable)
    // Busca de mas interno a mas externo igual que get()
    fn set(&mut self, name: &str, value: Value, span: Span) -> Result<(), RuntimeError> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(var) = scope.get_mut(name) {
                if !var.mutable {
                    return Err(RuntimeError {
                        message: format!("cannot assign to immutable variable '{}'", name),
                        span,
                    });
                }
                var.value = value;
                return Ok(());
            }
        }
        Err(RuntimeError {
            message: format!("undefined variable '{}'", name),
            span,
        })
    }
}

// --- Statement result for control flow ---

enum StmtResult {
    Normal,
    Return(Value),
}

// --- Interpreter ---

pub struct Interpreter {
    environment: Environment,
    functions: HashMap<String, FnDecl>,
    structs: HashMap<String, StructDecl>,
    alias: HashMap<String, String>, // alias -> file path
    base_dir: PathBuf,
    native_functions: HashMap<String, fn(Vec<Value>) -> Result<Value, RuntimeError>>,
}

impl Interpreter {
    pub fn new(base_dir: PathBuf) -> Self {
        let mut native_functions = HashMap::new();
        native_functions.insert("typeOf".to_string(), native_type_of as fn(Vec<Value>) -> Result<Value, RuntimeError>);
        Interpreter {
            environment: Environment::new(),
            functions: HashMap::new(),
            structs: HashMap::new(),
            alias: HashMap::new(),
            base_dir,
            native_functions,
        }
    }

    pub fn run(&mut self, program: &Program) -> Result<(), RuntimeError> {
        // Register all functions, structs, and file connections
        for decl in &program.declarations {
            match decl {
                Declaration::Function(f) => {
                    self.functions.insert(f.name.clone(), f.clone());
                }
                Declaration::Struct(s) => {
                    self.structs.insert(s.name.clone(), s.clone());
                }
                Declaration::Connect(c) => {
                    self.alias.insert(c.alias.clone(), c.file_path.clone());
                }
                Declaration::Statement(_) => {}
            }
        }

        // Execute top-level statements
        for decl in &program.declarations {
            if let Declaration::Statement(stmt) = decl {
                self.execute_stmt(stmt)?;
            }
        }

        // Call main if it exists
        if self.functions.contains_key("main") {
            let dummy_span = Span { line: 0, column: 0, start: 0, end: 0 };
            self.call_function("main", vec![], dummy_span)?;
        }

        Ok(())
    }

    fn execute_block(&mut self, block: &Block) -> Result<StmtResult, RuntimeError> {
        self.environment.push_scope();
        let result = self.execute_block_inner(block);
        self.environment.pop_scope();
        result
    }

    fn execute_block_inner(&mut self, block: &Block) -> Result<StmtResult, RuntimeError> {
        for stmt in &block.statements {
            match self.execute_stmt(stmt)? {
                StmtResult::Normal => {}
                ret @ StmtResult::Return(_) => return Ok(ret),
            }
        }
        Ok(StmtResult::Normal)
    }

    fn execute_stmt(&mut self, stmt: &Stmt) -> Result<StmtResult, RuntimeError> {
        match stmt {
            // let x: int = 42;           mutable=false, type_ann=Some(Int)
            // let mut count = 0;         mutable=true,  type_ann=None (inferido)
            // let data = #SELECT * ...;  initializer=SqlSelect (caso especial: mode string/struct)
            // Si type_ann es None no se verifica compatibilidad de tipos
            Stmt::Let { name, mutable, type_ann, initializer, span } => {
                let value = match initializer {
                    // Inicializador SQL: determina use_string_mode segun el tipo anotado
                    // list<list<string>> o list<primitivo> --> string mode (todo como strings)
                    // list<Struct> o sin tipo --> struct mode (convierte a StructInstance)
                    Expr::SqlSelect { columns, table_ref, condition, single, span: sql_span } => {
                        let use_string_mode = type_ann.as_ref()
                            .map(|t| Self::is_primitive_list_type(t))
                            .unwrap_or(false);
                        self.execute_sql_select(columns, table_ref, condition.as_deref(), *single, *sql_span, use_string_mode)?
                    }
                    // Cualquier otra expresion: se evalua normalmente
                    _ => self.evaluate_expr(initializer)?,
                };
                if let Some(ta) = type_ann {
                    self.check_type_compat(&value, ta, *span)?;
                }
                self.environment.define(name.clone(), value, *mutable);
                Ok(StmtResult::Normal)
            }

            // x = 5;       AssignTarget::Variable("x")
            // p.x = 1.0;   AssignTarget::FieldAccess { object=Identifier("p"), field="x" }
            // Para FieldAccess: get struct → modifica campo → set struct de vuelta
            Stmt::Assign { target, value, span } => {
                let val = self.evaluate_expr(value)?;
                match target {
                    // Reasignacion simple de variable (debe ser mut)
                    AssignTarget::Variable(name) => {
                        self.environment.set(name, val, *span)?;
                    }
                    // Reasignacion de campo de struct (solo un nivel de profundidad)
                    AssignTarget::FieldAccess { object, field } => {
                        // We need to get the struct, modify the field, and set it back
                        if let Expr::Identifier { name, .. } = object.as_ref() {
                            let mut struct_val = self.environment.get(name, *span)?;
                            if let Value::StructInstance { ref mut fields, .. } = struct_val {
                                if fields.contains_key(field) {
                                    fields.insert(field.clone(), val);
                                } else {
                                    return Err(RuntimeError {
                                        message: format!("struct has no field '{}'", field),
                                        span: *span,
                                    });
                                }
                                self.environment.set(name, struct_val, *span)?;
                            } else {
                                return Err(RuntimeError {
                                    message: format!("'{}' is not a struct", name),
                                    span: *span,
                                });
                            }
                        } else {
                            return Err(RuntimeError {
                                message: "nested field assignment not supported".to_string(),
                                span: *span,
                            });
                        }
                    }
                }
                Ok(StmtResult::Normal)
            }

            // if cond { ... }
            // if cond1 { ... } else if cond2 { ... } else if cond3 { ... } else { ... }
            //
            // "else if" fue desazucarado por el parser a else { if ... }
            // por lo que else_block es simplemente Option<Block>:
            //   None       = sin rama else
            //   Some(block)= else o else-if (el bloque puede contener un Stmt::If anidado)
            // La condicion siempre debe ser bool.
            Stmt::If { condition, then_block, else_block, span } => {
                let cond = self.evaluate_expr(condition)?;
                let cond_bool = match cond {
                    Value::Bool(b) => b,
                    _ => return Err(RuntimeError {
                        message: format!("condition must be bool, got '{}'", cond.type_name()),
                        span: *span,
                    }),
                };

                if cond_bool {
                    self.execute_block(then_block)
                } else if let Some(block) = else_block {
                    // Ejecuta el bloque else (que puede contener un if anidado en caso de else-if)
                    self.execute_block(block)
                } else {
                    Ok(StmtResult::Normal)
                }
            }

            // while cond { ... }
            // La condicion DEBE ser bool; itera hasta que sea false
            // Si el cuerpo tiene return, propaga hacia arriba (StmtResult::Return)
            Stmt::While { condition, body, span } => {
                loop {
                    let cond = self.evaluate_expr(condition)?;
                    let cond_bool = match cond {
                        Value::Bool(b) => b,
                        _ => return Err(RuntimeError {
                            message: format!("condition must be bool, got '{}'", cond.type_name()),
                            span: *span,
                        }),
                    };

                    if !cond_bool {
                        break;
                    }

                    match self.execute_block(body)? {
                        StmtResult::Normal => {}
                        ret @ StmtResult::Return(_) => return Ok(ret),
                    }
                }
                Ok(StmtResult::Normal)
            }

            // for i in 1..10 { ... }
            // variable="i"  start y end DEBEN ser int; end es EXCLUSIVO (como Rust)
            // La variable del bucle es inmutable y solo existe dentro del cuerpo
            Stmt::For { variable, start, end, body, span } => {
                let start_val = self.evaluate_expr(start)?;
                let end_val = self.evaluate_expr(end)?;

                let (start_i, end_i) = match (&start_val, &end_val) {
                    (Value::Int(s), Value::Int(e)) => (*s, *e),
                    _ => return Err(RuntimeError {
                        message: "for range bounds must be integers".to_string(),
                        span: *span,
                    }),
                };

                for i in start_i..end_i {
                    // Nuevo scope por iteracion para aislar la variable del bucle
                    self.environment.push_scope();
                    self.environment.define(variable.clone(), Value::Int(i), false);
                    let result = self.execute_block_inner(body);
                    self.environment.pop_scope();

                    match result? {
                        StmtResult::Normal => {}
                        ret @ StmtResult::Return(_) => return Ok(ret),
                    }
                }
                Ok(StmtResult::Normal)
            }

            // return 42;   value=Some(expr)   --> StmtResult::Return(Value)
            // return;      value=None          --> StmtResult::Return(Void)
            // StmtResult::Return burbujea hasta call_function, que lo extrae
            Stmt::Return { value, .. } => {
                let val = match value {
                    Some(expr) => self.evaluate_expr(expr)?,
                    None => Value::Void,
                };
                Ok(StmtResult::Return(val))
            }

            // print(arg1, arg2, ...);
            // Evalua cada arg y llama to_display_string(), luego imprime separados por espacio
            // Es un statement, no una funcion: no retorna valor
            Stmt::Print { args, .. } => {
                let values: Vec<String> = args
                    .iter()
                    .map(|a| self.evaluate_expr(a).map(|v| v.to_display_string()))
                    .collect::<Result<Vec<_>, _>>()?;
                println!("{}", values.join(" "));
                Ok(StmtResult::Normal)
            }

            // fn_call();  o cualquier expresion usada como statement
            // El valor de retorno se evalua pero se descarta (no se guarda en ninguna variable)
            Stmt::Expression { expr, .. } => {
                self.evaluate_expr(expr)?;
                Ok(StmtResult::Normal)
            }
        }
    }

    fn evaluate_expr(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            // 42, -7  -->  Value::Int(i64)
            Expr::IntLiteral { value, .. } => Ok(Value::Int(*value)),
            // 3.14, 1.0  -->  Value::Float(f64)
            Expr::FloatLiteral { value, .. } => Ok(Value::Float(*value)),
            // "hello"  -->  Value::Str(String)
            Expr::StringLiteral { value, .. } => Ok(Value::Str(value.clone())),
            // true / false  -->  Value::Bool(bool)
            Expr::BoolLiteral { value, .. } => Ok(Value::Bool(*value)),

            // x, name, users  -->  busca la variable en el Environment (scope mas interno primero)
            Expr::Identifier { name, span } => {
                self.environment.get(name, *span)
            }

            // (expr)  -->  simplemente evalua la expresion interior, el agrupamiento no cambia el valor
            Expr::Grouped { expr, .. } => self.evaluate_expr(expr),

            // Operadores unarios:
            //   UnaryOp::Neg --> -x  (int o float; error si otro tipo)
            //   UnaryOp::Not --> !x  (solo bool; error si otro tipo)
            Expr::UnaryOp { op, operand, span } => {
                let val = self.evaluate_expr(operand)?;
                match op {
                    UnaryOp::Neg => match val {
                        Value::Int(v) => Ok(Value::Int(-v)),
                        Value::Float(v) => Ok(Value::Float(-v)),
                        _ => Err(RuntimeError {
                            message: format!("cannot negate '{}'", val.type_name()),
                            span: *span,
                        }),
                    },
                    UnaryOp::Not => match val {
                        Value::Bool(v) => Ok(Value::Bool(!v)),
                        _ => Err(RuntimeError {
                            message: format!("cannot apply '!' to '{}'", val.type_name()),
                            span: *span,
                        }),
                    },
                }
            }

            // left op right
            // && y || tienen evaluacion short-circuit: no evalua right si el resultado ya es seguro
            //   a && b --> si a es false, devuelve false sin evaluar b
            //   a || b --> si a es true,  devuelve true  sin evaluar b
            // El resto de operadores (+, -, *, /, %, ==, !=, <, <=, >, >=) van a eval_binary_op
            Expr::BinaryOp { left, op, right, span } => {
                // Short-circuit for logical operators
                if matches!(op, BinaryOp::And | BinaryOp::Or) {
                    let left_val = self.evaluate_expr(left)?;
                    match op {
                        BinaryOp::And => {
                            if let Value::Bool(false) = left_val {
                                return Ok(Value::Bool(false));
                            }
                            let right_val = self.evaluate_expr(right)?;
                            match (&left_val, &right_val) {
                                (Value::Bool(_), Value::Bool(b)) => Ok(Value::Bool(*b)),
                                _ => Err(RuntimeError {
                                    message: format!("'&&' requires bool operands, got '{}' and '{}'", left_val.type_name(), right_val.type_name()),
                                    span: *span,
                                }),
                            }
                        }
                        BinaryOp::Or => {
                            if let Value::Bool(true) = left_val {
                                return Ok(Value::Bool(true));
                            }
                            let right_val = self.evaluate_expr(right)?;
                            match (&left_val, &right_val) {
                                (Value::Bool(_), Value::Bool(b)) => Ok(Value::Bool(*b)),
                                _ => Err(RuntimeError {
                                    message: format!("'||' requires bool operands, got '{}' and '{}'", left_val.type_name(), right_val.type_name()),
                                    span: *span,
                                }),
                            }
                        }
                        _ => unreachable!(),
                    }
                } else {
                    let left_val = self.evaluate_expr(left)?;
                    let right_val = self.evaluate_expr(right)?;
                    self.eval_binary_op(op, left_val, right_val, *span)
                }
            }

            // nombre(arg1, arg2, ...)
            // callee es el nombre; args se evaluan en orden antes de llamar
            // call_function busca primero en native_functions (typeOf), luego en functions del usuario
            Expr::FnCall { callee, args, span } => {
                let arg_values: Vec<Value> = args
                    .iter()
                    .map(|a| self.evaluate_expr(a))
                    .collect::<Result<Vec<_>, _>>()?;
                self.call_function(callee, arg_values, *span)
            }

            // objeto.campo   ej: p.x, user.name
            // Evalua object (debe ser StructInstance) y extrae el campo por nombre
            Expr::FieldAccess { object, field, span } => {
                let obj = self.evaluate_expr(object)?;
                match obj {
                    Value::StructInstance { fields, .. } => {
                        fields.get(field).cloned().ok_or_else(|| RuntimeError {
                            message: format!("struct has no field '{}'", field),
                            span: *span,
                        })
                    }
                    _ => Err(RuntimeError {
                        message: format!("cannot access field '{}' on '{}'", field, obj.type_name()),
                        span: *span,
                    }),
                }
            }

            // NombreStruct { campo1: expr1, campo2: expr2 }
            // ej: Point { x: 1.0, y: 2.0 }
            // 1. Busca la definicion del struct para verificar que existe
            // 2. Evalua cada expresion de campo
            // 3. Verifica que esten todos los campos declarados
            // 4. Devuelve Value::StructInstance { type_name, fields: HashMap }
            Expr::StructInit { name, fields, span } => {
                let struct_decl = self.structs.get(name).cloned().ok_or_else(|| RuntimeError {
                    message: format!("undefined struct '{}'", name),
                    span: *span,
                })?;

                let mut field_values = HashMap::new();
                for (fname, fexpr) in fields {
                    let val = self.evaluate_expr(fexpr)?;
                    field_values.insert(fname.clone(), val);
                }

                // Verify all fields are provided
                for sf in &struct_decl.fields {
                    if !field_values.contains_key(&sf.name) {
                        return Err(RuntimeError {
                            message: format!("missing field '{}' in struct '{}'", sf.name, name),
                            span: *span,
                        });
                    }
                }

                Ok(Value::StructInstance {
                    type_name: name.clone(),
                    fields: field_values,
                })
            }

            // #SELECT cols FROM tabla WHERE cond
            // Cuando aparece en una expresion (no en let), siempre usa struct mode (use_string_mode=false)
            // Para string mode, el Stmt::Let lo llama directamente con use_string_mode=true
            Expr::SqlSelect { columns, table_ref, condition, single, span } => {
                self.execute_sql_select(columns, table_ref, condition.as_deref(), *single, *span, false)
            }

            // #INSERT INTO tabla VALUES (expr1, expr2, ...)
            // Evalua las expresiones de values y las convierte a strings para el DataTable
            Expr::SqlInsert { table_ref, values, span } => {
                self.execute_sql_insert(table_ref, values, *span)
            }
        }
    }

    // --- SQL Execution ---

    // Resuelve un SqlTableRef a (PathBuf, nombre_tabla):
    //   Alias("users")       -->  busca en self.alias["users"] --> base_dir + "users.csv"
    //   Inline("data.csv")   -->  usa la ruta directamente      --> base_dir + "data.csv"
    fn resolve_file_path(&self, table_ref: &SqlTableRef, span: Span) -> Result<(PathBuf, String), RuntimeError> {
        match table_ref {
            SqlTableRef::Alias(alias) => {
                if let Some(file_path) = self.alias.get(alias) {
                    Ok((self.base_dir.join(file_path), alias.clone()))
                } else {
                    return Err(RuntimeError {
                        message: format!("unknown table alias '{}'", alias),
                        span,
                    });
                }
            }
            SqlTableRef::Inline(file_path) => {
                let path = std::path::Path::new(file_path);
                let name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(file_path)
                    .to_string();
                Ok((self.base_dir.join(file_path), name))
            }
        }
    }

    // Convierte una celda del CSV (String) al Value mas especifico posible:
    //   "42"    --> Value::Int(42)
    //   "3.14"  --> Value::Float(3.14)
    //   "true"  --> Value::Bool(true)
    //   "Alice" --> Value::Str("Alice")   (fallback)
    fn cell_to_value(s: &str) -> Value {
        // Try int
        if let Ok(i) = s.parse::<i64>() {
            return Value::Int(i);
        }
        // Try float
        if let Ok(f) = s.parse::<f64>() {
            return Value::Float(f);
        }
        // Try bool
        match s {
            "true" => return Value::Bool(true),
            "false" => return Value::Bool(false),
            _ => {}
        }
        // Default: string
        Value::Str(s.to_string())
    }

    fn row_to_struct(struct_name: &str, headers: &[String], row: &[String], columns: &[String]) -> Value {
        let mut fields = HashMap::new();
        let use_all = columns.len() == 1 && columns[0] == "*";

        for (i, header) in headers.iter().enumerate() {
            if use_all || columns.contains(header) {
                fields.insert(header.clone(), Self::cell_to_value(&row[i]));
            }
        }

        Value::StructInstance {
            type_name: struct_name.to_string(),
            fields,
        }
    }

    /// Convert a row to a list of string values (for list<list<string>> mode)
    fn row_to_string_list(headers: &[String], row: &[String], columns: &[String]) -> Value {
        let use_all = columns.len() == 1 && columns[0] == "*";
        let values: Vec<Value> = if use_all {
            row.iter().map(|s| Value::Str(s.clone())).collect()
        } else {
            columns.iter().filter_map(|col| {
                headers.iter().position(|h| h == col)
                    .map(|i| Value::Str(row[i].clone()))
            }).collect()
        };
        Value::List(values)
    }

    /// Check if type_ann is a primitive list type (not struct-based)
    fn is_primitive_list_type(type_ann: &TypeAnnotation) -> bool {
        matches!(type_ann,
            TypeAnnotation::List(inner) if matches!(inner.as_ref(),
                TypeAnnotation::List(_) | TypeAnnotation::StringType | TypeAnnotation::Int | TypeAnnotation::Float | TypeAnnotation::Bool
            )
        )
    }

    /// Find a declared struct whose fields match the given table headers
    fn find_matching_struct(&self, headers: &[String], columns: &[String]) -> Option<String> {
        let use_all = columns.len() == 1 && columns[0] == "*";
        let target_fields: Vec<&String> = if use_all {
            headers.iter().collect()
        } else {
            columns.iter().collect()
        };

        for (name, decl) in &self.structs {
            let struct_fields: Vec<&String> = decl.fields.iter().map(|f| &f.name).collect();
            // Check if all target fields exist in the struct
            let all_match = target_fields.iter().all(|f| struct_fields.contains(f));
            if all_match && target_fields.len() == struct_fields.len() {
                return Some(name.clone());
            }
        }
        // If exact match not found, try partial match (SELECT specific columns)
        if !use_all {
            for (name, decl) in &self.structs {
                let struct_fields: Vec<&String> = decl.fields.iter().map(|f| &f.name).collect();
                let all_match = target_fields.iter().all(|f| struct_fields.contains(f));
                if all_match {
                    return Some(name.clone());
                }
            }
        }
        None
    }

    fn execute_sql_select(
        &mut self,
        columns: &[String],
        table_ref: &SqlTableRef,
        condition: Option<&Expr>,
        single: bool,
        span: Span,
        use_string_mode: bool,
    ) -> Result<Value, RuntimeError> {
        let (file_path, table_name) = self.resolve_file_path(table_ref, span)?;
        let table = DataTable::from_file(&file_path).map_err(|e| RuntimeError {
            message: e,
            span,
        })?;

        // Validate columns exist
        let use_all = columns.len() == 1 && columns[0] == "*";
        if !use_all {
            for col in columns {
                if table.column_index(col).is_none() {
                    return Err(RuntimeError {
                        message: format!("column '{}' not found in table '{}'", col, table_name),
                        span,
                    });
                }
            }
        }

        // Determine the struct type name (only needed in struct mode)
        let struct_name = if !use_string_mode {
            self.find_matching_struct(&table.headers, columns)
                .unwrap_or_else(|| table_name.clone())
        } else {
            String::new()
        };

        let mut results = Vec::new();

        for row_idx in 0..table.rows.len() {
            let matches = if let Some(cond) = condition {
                // Push a scope with column values as variables
                self.environment.push_scope();
                for (i, header) in table.headers.iter().enumerate() {
                    let val = Self::cell_to_value(&table.rows[row_idx][i]);
                    self.environment.define(header.clone(), val, false);
                }
                let result = self.evaluate_expr(cond);
                self.environment.pop_scope();

                match result? {
                    Value::Bool(b) => b,
                    other => return Err(RuntimeError {
                        message: format!("WHERE condition must be bool, got '{}'", other.type_name()),
                        span,
                    }),
                }
            } else {
                true // No WHERE = all rows match
            };

            if matches {
                let row_value = if use_string_mode {
                    Self::row_to_string_list(&table.headers, &table.rows[row_idx], columns)
                } else {
                    Self::row_to_struct(&struct_name, &table.headers, &table.rows[row_idx], columns)
                };
                if single {
                    return Ok(row_value);
                }
                results.push(row_value);
            }
        }

        if single {
            return Err(RuntimeError {
                message: format!("SELECT SINGLE found no matching rows in '{}'", table_name),
                span,
            });
        }

        Ok(Value::List(results))
    }

    fn execute_sql_insert(
        &mut self,
        table_ref: &SqlTableRef,
        value_exprs: &[Expr],
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let (file_path, _table_name) = self.resolve_file_path(table_ref, span)?;

        // Read existing table
        let mut table = if file_path.exists() {
            DataTable::from_file(&file_path).map_err(|e| RuntimeError {
                message: e,
                span,
            })?
        } else {
            return Err(RuntimeError {
                message: format!("file '{}' does not exist", file_path.display()),
                span,
            });
        };

        // Evaluate values
        let mut row_values = Vec::new();
        for expr in value_exprs {
            let val = self.evaluate_expr(expr)?;
            let s = match val {
                Value::Int(v) => v.to_string(),
                Value::Float(v) => v.to_string(),
                Value::Bool(v) => v.to_string(),
                Value::Str(v) => v,
                _ => return Err(RuntimeError {
                    message: format!("cannot insert value of type '{}'", val.type_name()),
                    span,
                }),
            };
            row_values.push(s);
        }

        // Append and save
        if let Err(e) = table.append_row(&row_values) {
            return Err(RuntimeError { message: e, span });
        }

        match table.save_to_file(&file_path) {
            Ok(()) => Ok(Value::Bool(true)),
            Err(e) => Err(RuntimeError { message: e, span }),
        }
    }

    // --- Binary ops ---

    fn eval_binary_op(&self, op: &BinaryOp, left: Value, right: Value, span: Span) -> Result<Value, RuntimeError> {
        match op {
            // Arithmetic
            BinaryOp::Add => match (left, right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + b as f64)),
                (Value::Str(a), Value::Str(b)) => Ok(Value::Str(a + &b)),
                (a, b) => Err(RuntimeError {
                    message: format!("cannot add '{}' and '{}'", a.type_name(), b.type_name()),
                    span,
                }),
            },
            BinaryOp::Sub => self.numeric_op(left, right, span, "subtract", |a, b| a - b, |a, b| a - b),
            BinaryOp::Mul => self.numeric_op(left, right, span, "multiply", |a, b| a * b, |a, b| a * b),
            BinaryOp::Div => {
                // Check for division by zero
                match (&left, &right) {
                    (_, Value::Int(0)) => return Err(RuntimeError {
                        message: "division by zero".to_string(),
                        span,
                    }),
                    (_, Value::Float(f)) if *f == 0.0 => return Err(RuntimeError {
                        message: "division by zero".to_string(),
                        span,
                    }),
                    _ => {}
                }
                self.numeric_op(left, right, span, "divide", |a, b| a / b, |a, b| a / b)
            }
            BinaryOp::Mod => {
                match (&left, &right) {
                    (_, Value::Int(0)) => return Err(RuntimeError {
                        message: "modulo by zero".to_string(),
                        span,
                    }),
                    _ => {}
                }
                self.numeric_op(left, right, span, "modulo", |a, b| a % b, |a, b| a % b)
            }

            // Comparison
            BinaryOp::Eq => self.comparison_op(left, right, span, |ord| ord == std::cmp::Ordering::Equal),
            BinaryOp::Neq => self.comparison_op(left, right, span, |ord| ord != std::cmp::Ordering::Equal),
            BinaryOp::Lt => self.comparison_op(left, right, span, |ord| ord == std::cmp::Ordering::Less),
            BinaryOp::Lte => self.comparison_op(left, right, span, |ord| ord != std::cmp::Ordering::Greater),
            BinaryOp::Gt => self.comparison_op(left, right, span, |ord| ord == std::cmp::Ordering::Greater),
            BinaryOp::Gte => self.comparison_op(left, right, span, |ord| ord != std::cmp::Ordering::Less),

            BinaryOp::And | BinaryOp::Or => unreachable!("handled in evaluate_expr"),
        }
    }

    fn numeric_op(
        &self, left: Value, right: Value, span: Span, op_name: &str,
        int_op: impl Fn(i64, i64) -> i64,
        float_op: impl Fn(f64, f64) -> f64,
    ) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(a, b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(float_op(a, b))),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(float_op(a as f64, b))),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(float_op(a, b as f64))),
            (a, b) => Err(RuntimeError {
                message: format!("cannot {} '{}' and '{}'", op_name, a.type_name(), b.type_name()),
                span,
            }),
        }
    }

    fn comparison_op(
        &self, left: Value, right: Value, span: Span,
        cmp: impl Fn(std::cmp::Ordering) -> bool,
    ) -> Result<Value, RuntimeError> {
        let ordering = match (&left, &right) {
            (Value::Int(a), Value::Int(b)) => a.cmp(b),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
            (Value::Int(a), Value::Float(b)) => (*a as f64).partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
            (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(*b as f64)).unwrap_or(std::cmp::Ordering::Equal),
            (Value::Str(a), Value::Str(b)) => a.cmp(b),
            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            _ => return Err(RuntimeError {
                message: format!("cannot compare '{}' and '{}'", left.type_name(), right.type_name()),
                span,
            }),
        };
        Ok(Value::Bool(cmp(ordering)))
    }

    fn call_function(&mut self, name: &str, args: Vec<Value>, span: Span) -> Result<Value, RuntimeError> {
        // Primero busca funciones nativas
        if let Some(native_fn) = self.native_functions.get(name) {
            return native_fn(args);
        }

        let func = self.functions.get(name).cloned().ok_or_else(|| RuntimeError {
            message: format!("undefined function '{}'", name),
            span,
        })?;

        if args.len() != func.params.len() {
            return Err(RuntimeError {
                message: format!(
                    "function '{}' expects {} arguments, got {}",
                    name, func.params.len(), args.len()
                ),
                span,
            });
        }

        self.environment.push_scope();

        for (param, arg) in func.params.iter().zip(args) {
            self.environment.define(param.name.clone(), arg, false);
        }

        let result = self.execute_block_inner(&func.body);
        self.environment.pop_scope();

        match result? {
            StmtResult::Normal => Ok(Value::Void),
            StmtResult::Return(val) => Ok(val),
        }
    }

    fn check_type_compat(&self, value: &Value, type_ann: &TypeAnnotation, span: Span) -> Result<(), RuntimeError> {
        let compatible = match (value, type_ann) {
            (Value::Int(_), TypeAnnotation::Int) => true,
            (Value::Float(_), TypeAnnotation::Float) => true,
            (Value::Bool(_), TypeAnnotation::Bool) => true,
            (Value::Str(_), TypeAnnotation::StringType) => true,
            (Value::Void, TypeAnnotation::Void) => true,
            (Value::StructInstance { type_name, .. }, TypeAnnotation::UserDefined(name)) => type_name == name,
            (Value::List(_), TypeAnnotation::List(_)) => true,
            (Value::List(_), TypeAnnotation::UserDefined(_)) => true,  // Lists from SQL are loosely typed
            // Allow int -> float promotion in let bindings
            (Value::Int(_), TypeAnnotation::Float) => true,
            _ => false,
        };

        if !compatible {
            Err(RuntimeError {
                message: format!(
                    "type mismatch: expected '{}', got '{}'",
                    Self::type_ann_name(type_ann),
                    value.type_name()
                ),
                span,
            })
        } else {
            Ok(())
        }
    }

    fn type_ann_name(t: &TypeAnnotation) -> String {
        match t {
            TypeAnnotation::Int => "int".to_string(),
            TypeAnnotation::Float => "float".to_string(),
            TypeAnnotation::Bool => "bool".to_string(),
            TypeAnnotation::StringType => "string".to_string(),
            TypeAnnotation::Void => "void".to_string(),
            TypeAnnotation::List(inner) => format!("list<{}>", Self::type_ann_name(inner)),
            TypeAnnotation::UserDefined(name) => name.clone(),
        }
    }
}
