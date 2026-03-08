use crate::lexer::Span;
use crate::parser::ast::*;
use crate::utils::csv::DataTable;
use crate::utils::error::RuntimeError;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// --- Runtime Values ---

// Valores posibles en tiempo de ejecucion.
// Cada variante corresponde a un tipo del lenguaje:
//   Int(i64)                             -->  int
//   Float(f64)                           -->  float
//   Bool(bool)                           -->  bool
//   Str(String)                          -->  string
//   StructInstance { type_name, fields } -->  NombreStruct
//   List(Vec<Value>)                     -->  list<T>
//   Ok(Box<Value>)                       -->  ok(v)   variante ok de Result<T,E>
//   Err(Box<Value>)                      -->  err(e)  variante err de Result<T,E>
//   Some(Box<Value>)                     -->  some(v) variante some de Option<T>
//   None                                 -->  none    variante none de Option<T>
//   Void                                 -->  void    (retorno de funciones sin valor)
#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    StructInstance {
        type_name: String,
        fields: HashMap<String, Value>,
    },
    List(Vec<Value>),
    Ok(Box<Value>),
    Err(Box<Value>),
    Some(Box<Value>),
    None,
    Void,
    // Funcion de primera clase / closure
    // params   -> parametros con nombre y tipo (del AST)
    // body     -> bloque de codigo a ejecutar
    // captured -> snapshot del entorno en el momento de definicion (closure)
    Func {
        params: Vec<Param>,
        body: Block,
        captured: HashMap<String, Value>,
    },
    // Color::Red  (variante de un enum user-defined)
    EnumVariant {
        enum_name: String,
        variant: String,
    },
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            // Primitivos
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) => (*a as f64) == *b,
            (Value::Float(a), Value::Int(b)) => *a == (*b as f64),
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            // Compuestos
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Ok(a), Value::Ok(b)) => a == b,
            (Value::Err(a), Value::Err(b)) => a == b,
            (Value::Some(a), Value::Some(b)) => a == b,
            (Value::None, Value::None) => true,
            (Value::Void, Value::Void) => true,
            (
                Value::StructInstance {
                    type_name: t1,
                    fields: f1,
                },
                Value::StructInstance {
                    type_name: t2,
                    fields: f2,
                },
            ) => t1 == t2 && f1 == f2,
            // Las funciones no tienen igualdad estructural
            (Value::Func { .. }, Value::Func { .. }) => false,
            // Enum variants: iguales si mismo enum y mismo variant
            (
                Value::EnumVariant { enum_name: e1, variant: v1 },
                Value::EnumVariant { enum_name: e2, variant: v2 },
            ) => e1 == e2 && v1 == v2,
            // Tipos distintos → siempre false
            _ => false,
        }
    }
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
            Value::Ok(_) => "ok",
            Value::Err(_) => "err",
            Value::Some(_) => "some",
            Value::None => "none",
            Value::Void => "void",
            Value::Func { .. } => "fn",
            Value::EnumVariant { enum_name, .. } => enum_name,
        }
    }
    #[allow(dead_code)]
    fn is_truthy(&self) -> bool {
        match self {
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::Bool(b) => *b,
            Value::Str(s) => !s.is_empty(),
            Value::List(l) => !l.is_empty(),
            Value::Ok(_) | Value::Some(_) => true,
            Value::Err(_) | Value::None | Value::Void => false,
            Value::StructInstance { .. } => true,
            Value::Func { .. } => true,
            Value::EnumVariant { .. } => true,
        }
    }
    #[allow(dead_code)]
    fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Float(f) => Some(*f as i64),
            _ => None,
        }
    }
    #[allow(dead_code)]
    fn as_float(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }
    #[allow(dead_code)]
    fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
    #[allow(dead_code)]
    fn as_list(&self) -> Option<&Vec<Value>> {
        match self {
            Value::List(l) => Some(l),
            _ => None,
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
            Value::Ok(inner) => format!("ok({})", inner.to_display_string()),
            Value::Err(inner) => format!("err({})", inner.to_display_string()),
            Value::Some(inner) => format!("some({})", inner.to_display_string()),
            Value::None => "none".to_string(),
            Value::Void => "void".to_string(),
            Value::Func { params, .. } => {
                let names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
                format!("fn({})", names.join(", "))
            }
            Value::EnumVariant { enum_name, variant } => {
                format!("{}::{}", enum_name, variant)
            }
        }
    }

    fn partial_cmp(&self, b: &Value) -> Option<std::cmp::Ordering> {
        match (self, b) {
            (Value::Int(a), Value::Int(b)) => a.partial_cmp(b),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
            (Value::Int(a), Value::Float(b)) => (*a as f64).partial_cmp(b),
            (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(*b as f64)),
            (Value::Str(a), Value::Str(b)) => a.partial_cmp(b),
            _ => None, // otros tipos no son comparables
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

    // Snapshot de todas las variables visibles en el momento actual.
    // Las variables de scopes internos sobreescriben las de scopes externos.
    // Se usa para capturar el entorno en closures (Value::Func).
    fn snapshot(&self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        for scope in self.scopes.iter() {
            // exterior → interior: los internos sobreescriben
            for (name, var) in scope {
                map.insert(name.clone(), var.value.clone());
            }
        }
        map
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

// Represents the physical data source after resolving a SqlTableRef.
enum TableSource {
    Csv(PathBuf, String),     // (file_path, table_name)
    Sqlite(PathBuf, String),  // (db_path, table_name in sqlite)
}

// --- Interpreter ---

pub struct Interpreter {
    environment: Environment,
    functions: HashMap<String, FnDecl>,
    structs: HashMap<String, StructDecl>,
    alias: HashMap<String, String>,            // alias -> file path (CSV/file)
    db_tables: HashMap<String, (PathBuf, String)>, // table_name -> (db_path, table_in_sqlite)
    base_dir: PathBuf,
    native_functions: HashMap<String, fn(Vec<Value>) -> Result<Value, RuntimeError>>,
    imported_files: HashSet<PathBuf>, // para evitar imports circulares
    // Canal de propagacion del operador ?:
    // Cuando `expr?` encuentra err/none, guarda el valor aqui y devuelve Void como placeholder.
    // execute_block_inner lo detecta tras cada statement y emite StmtResult::Return.
    try_return: Option<Value>,
}

impl Interpreter {
    pub fn new(base_dir: PathBuf) -> Self {
        let mut native_functions: HashMap<String, fn(Vec<Value>) -> Result<Value, RuntimeError>> =
            HashMap::new();
        native_functions.insert("typeOf".to_string(), native_type_of);
        native_functions.insert("len".to_string(), native_len);
        native_functions.insert("push".to_string(), native_push);
        native_functions.insert("pop".to_string(), native_pop);
        native_functions.insert("toString".to_string(), native_to_string);
        native_functions.insert("parseInt".to_string(), native_parse_int);
        native_functions.insert("toFloat".to_string(), native_parse_float);
        native_functions.insert("ok".to_string(), native_ok);
        native_functions.insert("err".to_string(), native_err);
        native_functions.insert("some".to_string(), native_some);
        native_functions.insert("none".to_string(), native_none);
        native_functions.insert("unwrap".to_string(), native_unwrap);
        native_functions.insert("is_ok".to_string(), native_is_ok);
        native_functions.insert("is_err".to_string(), native_is_err);
        native_functions.insert("is_some".to_string(), native_is_some);
        native_functions.insert("is_none".to_string(), native_is_none);
        native_functions.insert("split".to_string(), native_split);
        native_functions.insert("join".to_string(), native_join);
        native_functions.insert("contains".to_string(), native_contains);
        native_functions.insert("trim".to_string(), native_trim);
        native_functions.insert("lower".to_string(), native_lower);
        native_functions.insert("upper".to_string(), native_upper);
        native_functions.insert("replace".to_string(), native_replace);
        native_functions.insert("substring".to_string(), native_substring);
        native_functions.insert("abs".to_string(), native_abs);
        native_functions.insert("sqrt".to_string(), native_sqrt);
        native_functions.insert("pow".to_string(), native_pow);
        native_functions.insert("powf".to_string(), native_powf);
        native_functions.insert("log".to_string(), native_log);
        native_functions.insert("sin".to_string(), native_sin);
        native_functions.insert("cos".to_string(), native_cos);
        native_functions.insert("tan".to_string(), native_tan);
        native_functions.insert("exp".to_string(), native_exp);
        native_functions.insert("ln".to_string(), native_ln);
        native_functions.insert("log".to_string(), native_log);
        native_functions.insert("log2".to_string(), native_log2);
        native_functions.insert("log10".to_string(), native_log10);
        native_functions.insert("floor".to_string(), native_floor);
        native_functions.insert("ceil".to_string(), native_ceil);
        native_functions.insert("round".to_string(), native_round);
        native_functions.insert("endsWith".to_string(), native_ends_with);
        native_functions.insert("startsWith".to_string(), native_starts_with);
        native_functions.insert("max".to_string(), native_max);
        native_functions.insert("min".to_string(), native_min);
        native_functions.insert("range".to_string(), native_range);
        native_functions.insert("indexOf".to_string(), native_index_of);
        native_functions.insert("lastIndexOf".to_string(), native_last_index_of);
        native_functions.insert("sort".to_string(), native_sort);
        native_functions.insert("zip".to_string(), native_zip);
        native_functions.insert("unzip".to_string(), native_unzip);
        native_functions.insert("first".to_string(), native_first);
        native_functions.insert("last".to_string(), native_last);
        native_functions.insert("concat".to_string(), native_concat);
        native_functions.insert("reverse".to_string(), native_reverse);
        native_functions.insert("slice".to_string(),native_slice);
        

        Interpreter {
            environment: Environment::new(),
            functions: HashMap::new(),
            structs: HashMap::new(),
            alias: HashMap::new(),
            db_tables: HashMap::new(),
            base_dir,
            native_functions,
            imported_files: HashSet::new(),
            try_return: None,
        }
    }

    pub fn run(&mut self, program: &Program) -> Result<(), RuntimeError> {
        // Register all functions, structs, enums, file connections, and process imports
        for decl in &program.declarations {
            match decl {
                Declaration::Function(f) => {
                    self.functions.insert(f.name.clone(), f.clone());
                }
                Declaration::Struct(s) => {
                    self.structs.insert(s.name.clone(), s.clone());
                }
                Declaration::Enum(_) => {
                    // Enums are structural: no runtime registration needed.
                    // Variants are created as Value::EnumVariant when evaluated.
                }
                Declaration::Connect(c) => {
                    self.register_connect(c);
                }
                Declaration::Import { path, span } => {
                    self.process_import(path, *span)?;
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
            let dummy_span = Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            };
            self.call_function("main", vec![], dummy_span)?;
        }

        Ok(())
    }

    fn process_import(&mut self, path: &str, span: Span) -> Result<(), RuntimeError> {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        let full_path = self.base_dir.join(path);
        let canonical = full_path.canonicalize().unwrap_or(full_path.clone());

        // Evitar imports circulares
        if self.imported_files.contains(&canonical) {
            return Ok(()); // ya importado, no hacer nada
        }
        self.imported_files.insert(canonical.clone());

        let source = std::fs::read_to_string(&full_path).map_err(|e| RuntimeError {
            message: format!("cannot import '{}': {}", path, e),
            span,
        })?;

        let tokens = Lexer::new(&source).tokenize().map_err(|e| RuntimeError {
            message: format!("import '{}' lexer error: {}", path, e.message),
            span,
        })?;

        let program = Parser::new(tokens).parse().map_err(|e| RuntimeError {
            message: format!("import '{}' parse error: {}", path, e.message),
            span,
        })?;

        // Solo registrar funciones, structs, enums y connects — no ejecutar statements
        for decl in &program.declarations {
            match decl {
                Declaration::Function(f) => {
                    self.functions.insert(f.name.clone(), f.clone());
                }
                Declaration::Struct(s) => {
                    self.structs.insert(s.name.clone(), s.clone());
                }
                Declaration::Enum(_) => {}
                Declaration::Connect(c) => {
                    self.register_connect(c);
                }
                Declaration::Import {
                    path: inner_path,
                    span: inner_span,
                } => {
                    // imports transitivos
                    self.process_import(inner_path, *inner_span)?;
                }
                Declaration::Statement(_) => {}
            }
        }

        Ok(())
    }

    // Registers a ConnectDecl: CSV/file/api go into self.alias; db goes into self.db_tables.
    fn register_connect(&mut self, c: &crate::parser::ast::ConnectDecl) {
        use crate::lexer::ConnectType;
        match c.connect_type {
            ConnectType::Db => {
                let db_path = self.base_dir.join(&c.file_path);
                for mapping in &c.mappings {
                    // table_name is the alias used in SQL; it maps to (db_path, actual_table)
                    self.db_tables.insert(
                        mapping.table_name.clone(),
                        (db_path.clone(), mapping.table_name.clone()),
                    );
                }
            }
            _ => {
                self.alias.insert(c.alias.clone(), c.file_path.clone());
            }
        }
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
                StmtResult::Normal => {
                    // Check if a ? operator triggered early return propagation
                    if let Some(val) = self.try_return.take() {
                        return Ok(StmtResult::Return(val));
                    }
                }
                ret @ StmtResult::Return(_) => return Ok(ret),
            }
        }
        Ok(StmtResult::Normal)
    }

    fn execute_stmt(&mut self, stmt: &Stmt) -> Result<StmtResult, RuntimeError> {
        match stmt {
            // let x: int = 42;   mutable=false, type_ann=Some(Int)
            // let mut count = 0;  mutable=true,  type_ann=None (inferido)
            // El inicializador es siempre una expresion que se evalua normalmente.
            // Si hay type_ann, se verifica compatibilidad DESPUES de evaluar.
            Stmt::Let {
                name,
                mutable,
                type_ann,
                initializer,
                span,
            } => {
                let value = self.evaluate_expr(initializer)?;
                if let Some(ta) = type_ann {
                    self.check_type_compat(&value, ta, *span)?;
                }
                self.environment.define(name.clone(), value, *mutable);
                Ok(StmtResult::Normal)
            }

            // x = 5;       AssignTarget::Variable("x")
            // p.x = 1.0;   AssignTarget::FieldAccess { object=Identifier("p"), field="x" }
            // Para FieldAccess: get struct → modifica campo → set struct de vuelta
            Stmt::Assign {
                target,
                value,
                span,
            } => {
                let val = self.evaluate_expr(value)?;
                match target {
                    // Reasignacion simple de variable (debe ser mut)
                    AssignTarget::Variable(name) => {
                        self.environment.set(name, val, *span)?;
                    }
                    // arr[i] = val — muta el elemento i de la lista
                    AssignTarget::Index {
                        object: var_name,
                        index,
                    } => {
                        let idx_val = self.evaluate_expr(index)?;
                        let idx = match idx_val {
                            Value::Int(i) => i,
                            _ => {
                                return Err(RuntimeError {
                                    message: "list index must be an integer".to_string(),
                                    span: *span,
                                })
                            }
                        };
                        let mut list_val = self.environment.get(var_name, *span)?;
                        if let Value::List(ref mut items) = list_val {
                            let len = items.len() as i64;
                            if idx < 0 || idx >= len {
                                return Err(RuntimeError {
                                    message: format!("index {} out of bounds (len={})", idx, len),
                                    span: *span,
                                });
                            }
                            items[idx as usize] = val;
                        } else {
                            return Err(RuntimeError {
                                message: format!("'{}' is not a list", var_name),
                                span: *span,
                            });
                        }
                        self.environment.set(var_name, list_val, *span)?;
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
            Stmt::If {
                condition,
                then_block,
                else_block,
                span,
            } => {
                let cond = self.evaluate_expr(condition)?;
                let cond_bool = match cond {
                    Value::Bool(b) => b,
                    _ => {
                        return Err(RuntimeError {
                            message: format!("condition must be bool, got '{}'", cond.type_name()),
                            span: *span,
                        })
                    }
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
            Stmt::While {
                condition,
                body,
                span,
            } => {
                loop {
                    let cond = self.evaluate_expr(condition)?;
                    let cond_bool = match cond {
                        Value::Bool(b) => b,
                        _ => {
                            return Err(RuntimeError {
                                message: format!(
                                    "condition must be bool, got '{}'",
                                    cond.type_name()
                                ),
                                span: *span,
                            })
                        }
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
            Stmt::For {
                variable,
                start,
                end,
                body,
                span,
            } => {
                let start_val = self.evaluate_expr(start)?;
                let end_val = self.evaluate_expr(end)?;

                let (start_i, end_i) = match (&start_val, &end_val) {
                    (Value::Int(s), Value::Int(e)) => (*s, *e),
                    _ => {
                        return Err(RuntimeError {
                            message: "for range bounds must be integers".to_string(),
                            span: *span,
                        })
                    }
                };

                for i in start_i..end_i {
                    // Nuevo scope por iteracion para aislar la variable del bucle
                    self.environment.push_scope();
                    self.environment
                        .define(variable.clone(), Value::Int(i), false);
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

            // match expr { pattern => { body } ... }
            // Evalua subject, recorre arms hasta el primer pattern que coincida,
            // ejecuta su body en un scope que incluye las variables bindeadas por el pattern.
            Stmt::Match {
                subject,
                arms,
                span,
            } => {
                let val = self.evaluate_expr(subject)?;
                for arm in arms {
                    if let Some(bindings) = self.match_pattern(&arm.pattern, &val) {
                        self.environment.push_scope();
                        for (name, bound_val) in bindings {
                            self.environment.define(name, bound_val, false);
                        }
                        let result = self.execute_block_inner(&arm.body);
                        self.environment.pop_scope();
                        return result;
                    }
                }
                // Ningun pattern coincidio (sin wildcard/_ al final)
                Err(RuntimeError {
                    message: format!(
                        "non-exhaustive match: no arm matched value '{}'",
                        val.to_display_string()
                    ),
                    span: *span,
                })
            }
        }
    }

    // Intenta hacer match de un valor contra un patron.
    // Devuelve Some(bindings) si coincide (lista de (nombre, valor) a bindear en el scope),
    // o None si no coincide.
    fn match_pattern(&self, pattern: &Pattern, value: &Value) -> Option<Vec<(String, Value)>> {
        match (pattern, value) {
            (Pattern::Ok(bind), Value::Ok(inner)) => Some(vec![(bind.clone(), *inner.clone())]),
            (Pattern::Err(bind), Value::Err(inner)) => Some(vec![(bind.clone(), *inner.clone())]),
            (Pattern::Some(bind), Value::Some(inner)) => Some(vec![(bind.clone(), *inner.clone())]),
            (Pattern::None, Value::None) => Some(vec![]),
            (Pattern::Wildcard, _) => Some(vec![]),
            (Pattern::Ident(name), v) => Some(vec![(name.clone(), v.clone())]),
            (
                Pattern::EnumVariant { enum_name: pe, variant: pv },
                Value::EnumVariant { enum_name: ve, variant: vv },
            ) if pe == ve && pv == vv => Some(vec![]),
            _ => None,
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
            // f"Hola {name}" --> concatena literales y valores evaluados
            Expr::FString { parts, .. } => {
                let mut result = String::new();
                for part in parts {
                    match part {
                        crate::parser::ast::AstFStringPart::Literal(s) => result.push_str(s),
                        crate::parser::ast::AstFStringPart::Expr(expr) => {
                            let val = self.evaluate_expr(expr)?;
                            result.push_str(&val.to_display_string());
                        }
                    }
                }
                Ok(Value::Str(result))
            }
            // true / false  -->  Value::Bool(bool)
            Expr::BoolLiteral { value, .. } => Ok(Value::Bool(*value)),

            // x, name, users  -->  busca en el Environment primero.
            // Si no está como variable pero sí como función declarada, devuelve Value::Func
            // (primera clase: let f = add;)
            Expr::Identifier { name, span } => match self.environment.get(name, *span) {
                Ok(val) => Ok(val),
                Err(_) => {
                    let maybe_func = self.functions.get(name.as_str()).cloned();
                    if let Some(func) = maybe_func {
                        let captured = self.environment.snapshot();
                        Ok(Value::Func {
                            params: func.params,
                            body: func.body,
                            captured,
                        })
                    } else {
                        Err(RuntimeError {
                            message: format!("undefined variable '{}'", name),
                            span: *span,
                        })
                    }
                }
            },

            // (expr)  -->  simplemente evalua la expresion interior, el agrupamiento no cambia el valor
            Expr::Grouped { expr, .. } => self.evaluate_expr(expr),

            // [e1, e2, e3]  -->  evalua cada elemento y construye Value::List
            // []            -->  Value::List(vec![])
            Expr::ListLiteral { elements, .. } => {
                let values = elements
                    .iter()
                    .map(|e| self.evaluate_expr(e))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::List(values))
            }

            // objeto[indice]  -->  accede al elemento indice de la lista
            // indice debe ser int; error si fuera de rango
            Expr::Index {
                object,
                index,
                span,
            } => {
                let obj_val = self.evaluate_expr(object)?;
                let idx_val = self.evaluate_expr(index)?;
                let idx = match idx_val {
                    Value::Int(i) => i,
                    _ => {
                        return Err(RuntimeError {
                            message: "list index must be an integer".to_string(),
                            span: *span,
                        })
                    }
                };
                match obj_val {
                    Value::List(items) => {
                        let len = items.len() as i64;
                        if idx < 0 || idx >= len {
                            Err(RuntimeError {
                                message: format!("index {} out of bounds (len={})", idx, len),
                                span: *span,
                            })
                        } else {
                            Ok(items[idx as usize].clone())
                        }
                    }
                    other => Err(RuntimeError {
                        message: format!("cannot index into '{}'", other.type_name()),
                        span: *span,
                    }),
                }
            }

            // |x, y| { body }  -->  Value::Func con snapshot del entorno actual (closure)
            // Los params se almacenan como Param sinteticos (tipo Void, no se usa en runtime)
            Expr::Lambda { params, body, .. } => {
                let captured = self.environment.snapshot();
                let synthetic_params: Vec<Param> = params
                    .iter()
                    .map(|name| Param {
                        name: name.clone(),
                        type_ann: TypeAnnotation::Void,
                        span: Span { line: 0, column: 0, start: 0, end: 0 },
                    })
                    .collect();
                Ok(Value::Func {
                    params:   synthetic_params,
                    body:     body.clone(),
                    captured,
                })
            }

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
            Expr::BinaryOp {
                left,
                op,
                right,
                span,
            } => {
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
                                    message: format!(
                                        "'&&' requires bool operands, got '{}' and '{}'",
                                        left_val.type_name(),
                                        right_val.type_name()
                                    ),
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
                                    message: format!(
                                        "'||' requires bool operands, got '{}' and '{}'",
                                        left_val.type_name(),
                                        right_val.type_name()
                                    ),
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
            Expr::FieldAccess {
                object,
                field,
                span,
            } => {
                let obj = self.evaluate_expr(object)?;
                match obj {
                    Value::StructInstance { fields, .. } => {
                        fields.get(field).cloned().ok_or_else(|| RuntimeError {
                            message: format!("struct has no field '{}'", field),
                            span: *span,
                        })
                    }
                    _ => Err(RuntimeError {
                        message: format!(
                            "cannot access field '{}' on '{}'",
                            field,
                            obj.type_name()
                        ),
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
                let struct_decl = self
                    .structs
                    .get(name)
                    .cloned()
                    .ok_or_else(|| RuntimeError {
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
            // Devuelve List<StructInstance> (o un StructInstance si es SINGLE).
            Expr::SqlSelect {
                columns,
                table_ref,
                condition,
                single,
                span,
            } => self.execute_sql_select(columns, table_ref, condition.as_deref(), *single, *span),

            // #INSERT INTO tabla VALUES (expr1, expr2, ...)
            // Evalua las expresiones de values y las convierte a strings para el DataTable
            Expr::SqlInsert {
                table_ref,
                values,
                span,
            } => self.execute_sql_insert(table_ref, values, *span),

            // expr?
            // Result: ok(v)? → v | err(e)? → set try_return=err(e), return Void
            // Option: some(v)? → v | none? → set try_return=none, return Void
            Expr::Try { expr, span } => {
                let val = self.evaluate_expr(expr)?;
                match val {
                    Value::Ok(inner)   => Ok(*inner),
                    Value::Some(inner) => Ok(*inner),
                    Value::Err(_) | Value::None => {
                        self.try_return = Some(val);
                        Ok(Value::Void)
                    }
                    other => Err(RuntimeError {
                        message: format!(
                            "? requires Result or Option, got '{}'",
                            other.type_name()
                        ),
                        span: *span,
                    }),
                }
            }

            // Color::Red  -->  Value::EnumVariant { enum_name: "Color", variant: "Red" }
            Expr::EnumVariant { enum_name, variant, .. } => {
                Ok(Value::EnumVariant {
                    enum_name: enum_name.clone(),
                    variant: variant.clone(),
                })
            }
        }
    }

    // --- SQL Execution ---

    // Represents the physical source for a SQL table reference.
    //   Csv(path, table_name)     -- a CSV/file connected via `connect file`
    //   Sqlite(db_path, table)    -- a SQLite table connected via `connect db`
    fn resolve_table_source(
        &self,
        table_ref: &SqlTableRef,
        span: Span,
    ) -> Result<TableSource, RuntimeError> {
        match table_ref {
            SqlTableRef::Alias(alias) => {
                // Check SQLite db_tables first
                if let Some((db_path, table_name)) = self.db_tables.get(alias) {
                    return Ok(TableSource::Sqlite(db_path.clone(), table_name.clone()));
                }
                // Fall back to CSV alias
                if let Some(file_path) = self.alias.get(alias) {
                    return Ok(TableSource::Csv(self.base_dir.join(file_path), alias.clone()));
                }
                Err(RuntimeError {
                    message: format!("unknown table alias '{}'", alias),
                    span,
                })
            }
            SqlTableRef::Inline(file_path) => {
                let path = std::path::Path::new(file_path);
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(file_path)
                    .to_string();
                Ok(TableSource::Csv(self.base_dir.join(file_path), name))
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

    fn row_to_struct(
        struct_name: &str,
        headers: &[String],
        row: &[String],
        columns: &[String],
    ) -> Value {
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
    ) -> Result<Value, RuntimeError> {
        let source = self.resolve_table_source(table_ref, span)?;

        let (headers, rows, table_name) = match source {
            TableSource::Csv(file_path, tname) => {
                let table = DataTable::from_file(&file_path)
                    .map_err(|e| RuntimeError { message: e, span })?;
                let rows = table.rows.clone();
                let headers = table.headers.clone();
                (headers, rows, tname)
            }
            TableSource::Sqlite(db_path, tname) => {
                let db_str = db_path.to_string_lossy().into_owned();
                let (headers, rows) = crate::utils::sqlite::load_table_rows(&db_str, &tname)
                    .map_err(|e| RuntimeError { message: e, span })?;
                (headers, rows, tname)
            }
        };

        // Validate columns exist
        let use_all = columns.len() == 1 && columns[0] == "*";
        if !use_all {
            for col in columns {
                if !headers.contains(col) {
                    return Err(RuntimeError {
                        message: format!("column '{}' not found in table '{}'", col, table_name),
                        span,
                    });
                }
            }
        }

        // Find a matching struct or use table_name as fallback
        let struct_name = self
            .find_matching_struct(&headers, columns)
            .unwrap_or_else(|| table_name.clone());

        let mut results = Vec::new();

        for row in &rows {
            let matches = if let Some(cond) = condition {
                // Temporary scope with row values as variables for WHERE evaluation
                self.environment.push_scope();
                for (i, header) in headers.iter().enumerate() {
                    let val = Self::cell_to_value(&row[i]);
                    self.environment.define(header.clone(), val, false);
                }
                let result = self.evaluate_expr(cond);
                self.environment.pop_scope();

                match result? {
                    Value::Bool(b) => b,
                    other => {
                        return Err(RuntimeError {
                            message: format!(
                                "WHERE condition must be bool, got '{}'",
                                other.type_name()
                            ),
                            span,
                        })
                    }
                }
            } else {
                true
            };

            if matches {
                let row_value = Self::row_to_struct(&struct_name, &headers, row, columns);
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
        // Evaluate value expressions first
        let mut row_values = Vec::new();
        for expr in value_exprs {
            let val = self.evaluate_expr(expr)?;
            let s = match &val {
                Value::Int(v) => v.to_string(),
                Value::Float(v) => v.to_string(),
                Value::Bool(v) => v.to_string(),
                Value::Str(v) => v.clone(),
                _ => {
                    return Err(RuntimeError {
                        message: format!("cannot insert value of type '{}'", val.type_name()),
                        span,
                    })
                }
            };
            row_values.push(s);
        }

        let source = self.resolve_table_source(table_ref, span)?;

        match source {
            TableSource::Sqlite(db_path, table_name) => {
                let db_str = db_path.to_string_lossy().into_owned();
                crate::utils::sqlite::insert_row(&db_str, &table_name, &row_values)
                    .map_err(|e| RuntimeError { message: e, span })?;
                Ok(Value::Bool(true))
            }
            TableSource::Csv(file_path, _) => {
                let mut table = if file_path.exists() {
                    DataTable::from_file(&file_path)
                        .map_err(|e| RuntimeError { message: e, span })?
                } else {
                    return Err(RuntimeError {
                        message: format!("file '{}' does not exist", file_path.display()),
                        span,
                    });
                };

                if let Err(e) = table.append_row(&row_values) {
                    return Err(RuntimeError { message: e, span });
                }

                match table.save_to_file(&file_path) {
                    Ok(()) => Ok(Value::Bool(true)),
                    Err(e) => Err(RuntimeError { message: e, span }),
                }
            }
        }
    }

    // --- Binary ops ---

    fn eval_binary_op(
        &self,
        op: &BinaryOp,
        left: Value,
        right: Value,
        span: Span,
    ) -> Result<Value, RuntimeError> {
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
            BinaryOp::Sub => {
                self.numeric_op(left, right, span, "subtract", |a, b| a - b, |a, b| a - b)
            }
            BinaryOp::Mul => {
                self.numeric_op(left, right, span, "multiply", |a, b| a * b, |a, b| a * b)
            }
            BinaryOp::Div => {
                // Check for division by zero
                match (&left, &right) {
                    (_, Value::Int(0)) => {
                        return Err(RuntimeError {
                            message: "division by zero".to_string(),
                            span,
                        })
                    }
                    (_, Value::Float(f)) if *f == 0.0 => {
                        return Err(RuntimeError {
                            message: "division by zero".to_string(),
                            span,
                        })
                    }
                    _ => {}
                }
                self.numeric_op(left, right, span, "divide", |a, b| a / b, |a, b| a / b)
            }
            BinaryOp::Mod => {
                match (&left, &right) {
                    (_, Value::Int(0)) => {
                        return Err(RuntimeError {
                            message: "modulo by zero".to_string(),
                            span,
                        })
                    }
                    _ => {}
                }
                self.numeric_op(left, right, span, "modulo", |a, b| a % b, |a, b| a % b)
            }

            // Igualdad — funciona con cualquier Value
            BinaryOp::Eq => Ok(Value::Bool(left == right)),
            BinaryOp::Neq => Ok(Value::Bool(left != right)),
            // Orden — solo tipos ordenables (numéricos y string)
            BinaryOp::Lt => {
                self.comparison_op(left, right, span, |ord| ord == std::cmp::Ordering::Less)
            }
            BinaryOp::Lte => {
                self.comparison_op(left, right, span, |ord| ord != std::cmp::Ordering::Greater)
            }
            BinaryOp::Gt => {
                self.comparison_op(left, right, span, |ord| ord == std::cmp::Ordering::Greater)
            }
            BinaryOp::Gte => {
                self.comparison_op(left, right, span, |ord| ord != std::cmp::Ordering::Less)
            }

            BinaryOp::And | BinaryOp::Or => unreachable!("handled in evaluate_expr"),
        }
    }

    fn numeric_op(
        &self,
        left: Value,
        right: Value,
        span: Span,
        op_name: &str,
        int_op: impl Fn(i64, i64) -> i64,
        float_op: impl Fn(f64, f64) -> f64,
    ) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(a, b))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(float_op(a, b))),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(float_op(a as f64, b))),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(float_op(a, b as f64))),
            (a, b) => Err(RuntimeError {
                message: format!(
                    "cannot {} '{}' and '{}'",
                    op_name,
                    a.type_name(),
                    b.type_name()
                ),
                span,
            }),
        }
    }

    fn comparison_op(
        &self,
        left: Value,
        right: Value,
        span: Span,
        cmp: impl Fn(std::cmp::Ordering) -> bool,
    ) -> Result<Value, RuntimeError> {
        let ordering = match (&left, &right) {
            (Value::Int(a), Value::Int(b)) => a.cmp(b),
            (Value::Float(a), Value::Float(b)) => {
                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
            }
            (Value::Int(a), Value::Float(b)) => (*a as f64)
                .partial_cmp(b)
                .unwrap_or(std::cmp::Ordering::Equal),
            (Value::Float(a), Value::Int(b)) => a
                .partial_cmp(&(*b as f64))
                .unwrap_or(std::cmp::Ordering::Equal),
            (Value::Str(a), Value::Str(b)) => a.cmp(b),
            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            _ => {
                return Err(RuntimeError {
                    message: format!(
                        "cannot compare '{}' and '{}'",
                        left.type_name(),
                        right.type_name()
                    ),
                    span,
                })
            }
        };
        Ok(Value::Bool(cmp(ordering)))
    }

    fn call_function(
        &mut self,
        name: &str,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        // 0. HOFs que necesitan &mut self (no pueden estar en native_functions)
        match name {
            "map" => return self.builtin_map(args, span),
            "filter" => return self.builtin_filter(args, span),
            "reduce" => return self.builtin_reduce(args, span),
            "sortBy" => return self.builtin_sort_by(args, span),
            _ => {}
        }

        // 1. Funciones declaradas con fn (tienen prioridad sobre built-ins)
        if let Some(func) = self.functions.get(name).cloned() {
            if args.len() != func.params.len() {
                return Err(RuntimeError {
                    message: format!(
                        "function '{}' expects {} arguments, got {}",
                        name,
                        func.params.len(),
                        args.len()
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
            return match result? {
                StmtResult::Normal => Ok(Value::Void),
                StmtResult::Return(val) => Ok(val),
            };
        }

        // 2. Funciones nativas (built-ins)
        if let Some(native_fn) = self.native_functions.get(name) {
            return native_fn(args);
        }

        // 3. Variable que contiene un Value::Func (primera clase)
        if let Ok(Value::Func {
            params,
            body,
            captured,
        }) = self.environment.get(name, span)
        {
            return self.call_func_value(params, body, captured, args, span);
        }

        Err(RuntimeError {
            message: format!("undefined function '{}'", name),
            span,
        })
    }

    // Ejecuta un Value::Func con los argumentos dados.
    // Inyecta primero el entorno capturado (closure) y luego los params
    // para que los params siempre tengan prioridad.
    fn call_func_value(
        &mut self,
        params: Vec<Param>,
        body: Block,
        captured: HashMap<String, Value>,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        if args.len() != params.len() {
            return Err(RuntimeError {
                message: format!(
                    "function expects {} arguments, got {}",
                    params.len(),
                    args.len()
                ),
                span,
            });
        }
        self.environment.push_scope();
        // Primero el entorno capturado (closure)
        for (name, val) in captured {
            self.environment.define(name, val, false);
        }
        // Luego los params (sobreescriben captured si hay colision)
        for (param, arg) in params.iter().zip(args) {
            self.environment.define(param.name.clone(), arg, false);
        }
        let result = self.execute_block_inner(&body);
        self.environment.pop_scope();
        match result? {
            StmtResult::Normal => Ok(Value::Void),
            StmtResult::Return(val) => Ok(val),
        }
    }

    // --- Higher-order built-ins (necesitan &mut self para ejecutar Value::Func) ---

    fn builtin_map(&mut self, args: Vec<Value>, span: Span) -> Result<Value, RuntimeError> {
        if args.len() != 2 {
            return Err(native_err_arg("map", 2, args.len()));
        }
        match (args[0].clone(), args[1].clone()) {
            (
                Value::List(items),
                Value::Func {
                    params,
                    body,
                    captured,
                },
            ) => {
                let mut result = Vec::new();
                for item in items {
                    let val = self.call_func_value(
                        params.clone(),
                        body.clone(),
                        captured.clone(),
                        vec![item],
                        span,
                    )?;
                    result.push(val);
                }
                Ok(Value::List(result))
            }
            (a, b) => Err(RuntimeError {
                message: format!(
                    "'map' expects a list and a function, got '{}' and '{}'",
                    a.type_name(),
                    b.type_name()
                ),
                span,
            }),
        }
    }

    fn builtin_filter(&mut self, args: Vec<Value>, span: Span) -> Result<Value, RuntimeError> {
        if args.len() != 2 {
            return Err(native_err_arg("filter", 2, args.len()));
        }
        match (args[0].clone(), args[1].clone()) {
            (
                Value::List(items),
                Value::Func {
                    params,
                    body,
                    captured,
                },
            ) => {
                let mut result = Vec::new();
                for item in &items {
                    let val = self.call_func_value(
                        params.clone(),
                        body.clone(),
                        captured.clone(),
                        vec![item.clone()],
                        span,
                    )?;
                    if val == Value::Bool(true) {
                        result.push(item.clone());
                    }
                }
                Ok(Value::List(result))
            }
            (a, b) => Err(RuntimeError {
                message: format!(
                    "'filter' expects a list and a function, got '{}' and '{}'",
                    a.type_name(),
                    b.type_name()
                ),
                span,
            }),
        }
    }

    fn builtin_reduce(&mut self, args: Vec<Value>, span: Span) -> Result<Value, RuntimeError> {
        if args.len() != 3 {
            return Err(native_err_arg("reduce", 3, args.len()));
        }
        let initial = args[2].clone();
        match (args[0].clone(), args[1].clone()) {
            (Value::List(items), Value::Func { params, body, captured }) => {
                let mut acc = initial;
                for item in items {
                    acc = self.call_func_value(params.clone(), body.clone(), captured.clone(), vec![acc, item], span)?;
                }
                Ok(acc)
            }
            (a, b) => Err(RuntimeError {
                message: format!("'reduce' expects a list, a function and an initial value, got '{}', '{}' and '{}'", a.type_name(), b.type_name(), initial.type_name()),
                span,
            }),
        }
    }

    fn builtin_sort_by(&mut self, args: Vec<Value>, span: Span) -> Result<Value, RuntimeError> {
        if args.len() != 2 {
            return Err(native_err_arg("sortBy", 2, args.len()));
        }
        match (args[0].clone(), args[1].clone()) {
            (
                Value::List(items),
                Value::Func {
                    params,
                    body,
                    captured,
                },
            ) => {
                // Precalculamos las claves para poder usar &mut self
                let mut keyed: Vec<(Value, Value)> = Vec::new();
                for item in &items {
                    let key = self.call_func_value(
                        params.clone(),
                        body.clone(),
                        captured.clone(),
                        vec![item.clone()],
                        span,
                    )?;
                    keyed.push((key, item.clone()));
                }
                keyed.sort_by(|(k1, _), (k2, _)| {
                    k1.partial_cmp(k2).unwrap_or(std::cmp::Ordering::Equal)
                });
                Ok(Value::List(keyed.into_iter().map(|(_, v)| v).collect()))
            }
            (a, b) => Err(RuntimeError {
                message: format!(
                    "'sortBy' expects a list and a function, got '{}' and '{}'",
                    a.type_name(),
                    b.type_name()
                ),
                span,
            }),
        }
    }

    fn check_type_compat(
        &self,
        value: &Value,
        type_ann: &TypeAnnotation,
        span: Span,
    ) -> Result<(), RuntimeError> {
        let compatible = match (value, type_ann) {
            (Value::Int(_), TypeAnnotation::Int) => true,
            (Value::Float(_), TypeAnnotation::Float) => true,
            (Value::Bool(_), TypeAnnotation::Bool) => true,
            (Value::Str(_), TypeAnnotation::StringType) => true,
            (Value::Void, TypeAnnotation::Void) => true,
            (Value::StructInstance { type_name, .. }, TypeAnnotation::UserDefined(name)) => {
                type_name == name
            }
            (Value::List(_), TypeAnnotation::List(_)) => true,
            (Value::List(_), TypeAnnotation::UserDefined(_)) => true, // Lists from SQL are loosely typed
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
            TypeAnnotation::Result(ok_t, err_t) => {
                format!(
                    "Result<{}, {}>",
                    Self::type_ann_name(ok_t),
                    Self::type_ann_name(err_t)
                )
            }
            TypeAnnotation::Option(inner) => format!("Option<{}>", Self::type_ann_name(inner)),
            TypeAnnotation::UserDefined(name) => name.clone(),
        }
    }
}

// ─── Native built-in functions ────────────────────────────────────────────────

fn native_err_arg(name: &str, expected: usize, got: usize) -> RuntimeError {
    RuntimeError {
        message: format!("'{}' expects {} argument(s), got {}", name, expected, got),
        span: Span {
            line: 0,
            column: 0,
            start: 0,
            end: 0,
        },
    }
}

fn native_type_of(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("typeOf", 1, args.len()));
    }
    Ok(Value::Str(args[0].type_name().to_string()))
}

fn native_len(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("len", 1, args.len()));
    }
    match &args[0] {
        Value::List(items) => Ok(Value::Int(items.len() as i64)),
        Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
        other => Err(RuntimeError {
            message: format!(
                "'len' expects a list or string, got '{}'",
                other.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_push(mut args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(native_err_arg("push", 2, args.len()));
    }
    let elem = args.pop().unwrap();
    let list = args.pop().unwrap();
    match list {
        Value::List(mut items) => {
            items.push(elem);
            Ok(Value::List(items))
        }
        other => Err(RuntimeError {
            message: format!(
                "'push' expects a list as first argument, got '{}'",
                other.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_pop(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("pop", 1, args.len()));
    }
    match &args[0] {
        Value::List(items) => {
            if items.is_empty() {
                return Err(RuntimeError {
                    message: "'pop' called on empty list".to_string(),
                    span: Span {
                        line: 0,
                        column: 0,
                        start: 0,
                        end: 0,
                    },
                });
            }
            let mut new_items = items.clone();
            new_items.pop();
            Ok(Value::List(new_items))
        }
        other => Err(RuntimeError {
            message: format!("'pop' expects a list, got '{}'", other.type_name()),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_to_string(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("toString", 1, args.len()));
    }
    Ok(Value::Str(args[0].to_display_string()))
}

fn native_parse_int(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("parseInt", 1, args.len()));
    }
    match &args[0] {
        Value::Str(s) => s
            .trim()
            .parse::<i64>()
            .map(Value::Int)
            .map_err(|_| RuntimeError {
                message: format!("'parseInt' cannot parse '{}' as int", s),
                span: Span {
                    line: 0,
                    column: 0,
                    start: 0,
                    end: 0,
                },
            }),
        Value::Int(i) => Ok(Value::Int(*i)),
        other => Err(RuntimeError {
            message: format!("'parseInt' expects a string, got '{}'", other.type_name()),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_parse_float(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("toFloat", 1, args.len()));
    }
    match &args[0] {
        Value::Str(s) => s
            .trim()
            .parse::<f64>()
            .map(Value::Float)
            .map_err(|_| RuntimeError {
                message: format!("'toFloat' cannot parse '{}' as float", s),
                span: Span {
                    line: 0,
                    column: 0,
                    start: 0,
                    end: 0,
                },
            }),
        Value::Float(f) => Ok(Value::Float(*f)),
        Value::Int(i) => Ok(Value::Float(*i as f64)),
        other => Err(RuntimeError {
            message: format!("'toFloat' expects a string, got '{}'", other.type_name()),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_ok(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("ok", 1, args.len()));
    }
    Ok(Value::Ok(Box::new(args.into_iter().next().unwrap())))
}

fn native_err(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("err", 1, args.len()));
    }
    Ok(Value::Err(Box::new(args.into_iter().next().unwrap())))
}

fn native_some(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("some", 1, args.len()));
    }
    Ok(Value::Some(Box::new(args.into_iter().next().unwrap())))
}

fn native_none(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if !args.is_empty() {
        return Err(native_err_arg("none", 0, args.len()));
    }
    Ok(Value::None)
}

fn native_unwrap(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("unwrap", 1, args.len()));
    }
    match args.into_iter().next().unwrap() {
        Value::Ok(inner) | Value::Some(inner) => Ok(*inner),
        Value::Err(e) => Err(RuntimeError {
            message: format!("unwrap called on err({})", e.to_display_string()),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
        Value::None => Err(RuntimeError {
            message: "unwrap called on none".to_string(),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
        other => Err(RuntimeError {
            message: format!(
                "'unwrap' expects Result or Option, got '{}'",
                other.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_is_ok(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("is_ok", 1, args.len()));
    }
    Ok(Value::Bool(matches!(args[0], Value::Ok(_))))
}

fn native_is_err(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("is_err", 1, args.len()));
    }
    Ok(Value::Bool(matches!(args[0], Value::Err(_))))
}

fn native_is_some(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("is_some", 1, args.len()));
    }
    Ok(Value::Bool(matches!(args[0], Value::Some(_))))
}

fn native_is_none(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("is_none", 1, args.len()));
    }
    Ok(Value::Bool(matches!(args[0], Value::None)))
}

fn native_split(args: Vec<Value>) -> Result<Value, RuntimeError> {
    match args.len() {
        1 => {
            let parts = match &args[0] {
                Value::Str(s) => s
                    .split(" ")
                    .map(|part| Value::Str(part.to_string()))
                    .collect(),
                other => {
                    return Err(RuntimeError {
                        message: format!(
                            "'split' expects a string as first argument, got '{}'",
                            other.type_name()
                        ),
                        span: Span {
                            line: 0,
                            column: 0,
                            start: 0,
                            end: 0,
                        },
                    })
                }
            };
            return Ok(Value::List(parts));
        } // default delimiter: space
        2 => match (&args[0], &args[1]) {
            (Value::Str(s), Value::Str(delim)) => {
                let parts = s
                    .split(delim)
                    .map(|part| Value::Str(part.to_string()))
                    .collect();
                Ok(Value::List(parts))
            }
            (a, b) => Err(RuntimeError {
                message: format!(
                    "'split' expects two strings, got '{}' and '{}'",
                    a.type_name(),
                    b.type_name()
                ),
                span: Span {
                    line: 0,
                    column: 0,
                    start: 0,
                    end: 0,
                },
            }),
        }, // ok
        _ => {
            return Err(native_err_arg("split", 2, args.len()));
        }
    }
}

fn native_trim(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("trim", 1, args.len()));
    }
    match &args[0] {
        Value::Str(s) => Ok(Value::Str(s.trim().to_string())),
        other => Err(RuntimeError {
            message: format!(
                "'trim' expects a string as first argument, got '{}'",
                other.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_contains(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(native_err_arg("contains", 2, args.len()));
    }
    match (&args[0], &args[1]) {
        (Value::Str(s), Value::Str(substr)) => Ok(Value::Bool(s.contains(substr))),
        (Value::List(items), item) => Ok(Value::Bool(items.iter().any(|i| i == item))),
        (a, b) => Err(RuntimeError {
            message: format!(
                "'contains' expects two values, got '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_lower(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("lower", 1, args.len()));
    }
    match &args[0] {
        Value::Str(s) => Ok(Value::Str(s.to_lowercase())),
        other => Err(RuntimeError {
            message: format!(
                "'lower' expects a string as first argument, got '{}'",
                other.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_upper(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("upper", 1, args.len()));
    }
    match &args[0] {
        Value::Str(s) => Ok(Value::Str(s.to_uppercase())),
        other => Err(RuntimeError {
            message: format!(
                "'upper' expects a string as first argument, got '{}'",
                other.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_replace(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(native_err_arg("replace", 3, args.len()));
    }
    match (&args[0], &args[1], &args[2]) {
        (Value::Str(s), Value::Str(from), Value::Str(to)) => Ok(Value::Str(s.replace(from, to))),
        (a, b, c) => Err(RuntimeError {
            message: format!(
                "'replace' expects three strings, got '{}', '{}' and '{}'",
                a.type_name(),
                b.type_name(),
                c.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_ends_with(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(native_err_arg("endsWith", 2, args.len()));
    }
    match (&args[0], &args[1]) {
        (Value::Str(s), Value::Str(suffix)) => Ok(Value::Bool(s.ends_with(suffix))),
        (a, b) => Err(RuntimeError {
            message: format!(
                "'endsWith' expects two strings, got '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_starts_with(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(native_err_arg("startsWith", 2, args.len()));
    }
    match (&args[0], &args[1]) {
        (Value::Str(s), Value::Str(prefix)) => Ok(Value::Bool(s.starts_with(prefix))),
        (a, b) => Err(RuntimeError {
            message: format!(
                "'startsWith' expects two strings, got '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_substring(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(native_err_arg("substring", 3, args.len()));
    }
    match (&args[0], &args[1], &args[2]) {
        (Value::Str(s), Value::Int(start), Value::Int(len)) => {
            let start = *start as usize;
            let len = *len as usize;
            if start >= s.len() {
                return Ok(Value::Str("".to_string()));
            }
            let end = std::cmp::min(start + len, s.len());
            Ok(Value::Str(s[start..end].to_string()))
        }
        (a, b, c) => Err(RuntimeError {
            message: format!(
                "'substring' expects a string and two ints, got '{}', '{}' and '{}'",
                a.type_name(),
                b.type_name(),
                c.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_abs(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("abs", 1, args.len()));
    }
    match &args[0] {
        Value::Int(i) => Ok(Value::Int(i.abs())),
        Value::Float(f) => Ok(Value::Float(f.abs())),
        other => Err(RuntimeError {
            message: format!("'abs' expects a number, got '{}'", other.type_name()),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}
fn native_pow(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(native_err_arg("pow", 2, args.len()));
    }
    match (&args[0], &args[1]) {
        (Value::Int(base), Value::Int(exp)) => Ok(Value::Int(base.pow(*exp as u32))),
        (Value::Float(base), Value::Float(exp)) => Ok(Value::Float(base.powf(*exp))),
        (Value::Int(base), Value::Float(exp)) => Ok(Value::Float((*base as f64).powf(*exp))),
        (Value::Float(base), Value::Int(exp)) => Ok(Value::Float(base.powf(*exp as f64))),
        (a, b) => Err(RuntimeError {
            message: format!(
                "'pow' expects two numbers, got '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}
fn native_powf(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(native_err_arg("powf", 2, args.len()));
    }
    match (&args[0], &args[1]) {
        (Value::Int(base), Value::Int(exp)) => Ok(Value::Float((*base as f64).powf(*exp as f64))),
        (Value::Float(base), Value::Float(exp)) => Ok(Value::Float(base.powf(*exp))),
        (Value::Int(base), Value::Float(exp)) => Ok(Value::Float((*base as f64).powf(*exp))),
        (Value::Float(base), Value::Int(exp)) => Ok(Value::Float(base.powf(*exp as f64))),
        (a, b) => Err(RuntimeError {
            message: format!(
                "'powf' expects two numbers, got '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_exp(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("exp", 1, args.len()));
    }
    match &args[0] {
        Value::Int(i) => Ok(Value::Float((*i as f64).exp())),
        Value::Float(f) => Ok(Value::Float(f.exp())),
        other => Err(RuntimeError {
            message: format!("'exp' expects a number, got '{}'", other.type_name()),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_ln(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("ln", 1, args.len()));
    }
    match &args[0] {
        Value::Int(i) => {
            if *i <= 0 {
                return Err(RuntimeError {
                    message: format!("'ln' cannot take non-positive number, got {}", i),
                    span: Span {
                        line: 0,
                        column: 0,
                        start: 0,
                        end: 0,
                    },
                });
            }
            Ok(Value::Float((*i as f64).ln()))
        }
        Value::Float(f) => {
            if *f <= 0.0 {
                return Err(RuntimeError {
                    message: format!("'ln' cannot take non-positive number, got {}", f),
                    span: Span {
                        line: 0,
                        column: 0,
                        start: 0,
                        end: 0,
                    },
                });
            }
            Ok(Value::Float(f.ln()))
        }
        other => Err(RuntimeError {
            message: format!("'ln' expects a number, got '{}'", other.type_name()),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_log10(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("log10", 1, args.len()));
    }
    match &args[0] {
        Value::Int(i) => {
            if *i <= 0 {
                return Err(RuntimeError {
                    message: format!("'log10' cannot take non-positive number, got {}", i),
                    span: Span {
                        line: 0,
                        column: 0,
                        start: 0,
                        end: 0,
                    },
                });
            }
            Ok(Value::Float((*i as f64).log10()))
        }
        Value::Float(f) => {
            if *f <= 0.0 {
                return Err(RuntimeError {
                    message: format!("'log10' cannot take non-positive number, got {}", f),
                    span: Span {
                        line: 0,
                        column: 0,
                        start: 0,
                        end: 0,
                    },
                });
            }
            Ok(Value::Float(f.log10()))
        }
        other => Err(RuntimeError {
            message: format!("'log10' expects a number, got '{}'", other.type_name()),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_log2(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("log2", 1, args.len()));
    }
    match &args[0] {
        Value::Int(i) => {
            if *i <= 0 {
                return Err(RuntimeError {
                    message: format!("'log2' cannot take non-positive number, got {}", i),
                    span: Span {
                        line: 0,
                        column: 0,
                        start: 0,
                        end: 0,
                    },
                });
            }
            Ok(Value::Float((*i as f64).log2()))
        }
        Value::Float(f) => {
            if *f <= 0.0 {
                return Err(RuntimeError {
                    message: format!("'log2' cannot take non-positive number, got {}", f),
                    span: Span {
                        line: 0,
                        column: 0,
                        start: 0,
                        end: 0,
                    },
                });
            }
            Ok(Value::Float(f.log2()))
        }
        other => Err(RuntimeError {
            message: format!("'log2' expects a number, got '{}'", other.type_name()),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_log(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("log", 1, args.len()));
    }
    match &args[0] {
        Value::Int(i) => {
            if *i <= 0 {
                return Err(RuntimeError {
                    message: format!("'log' cannot take non-positive number, got {}", i),
                    span: Span {
                        line: 0,
                        column: 0,
                        start: 0,
                        end: 0,
                    },
                });
            }
            Ok(Value::Float((*i as f64).ln()))
        }
        Value::Float(f) => {
            if *f <= 0.0 {
                return Err(RuntimeError {
                    message: format!("'log' cannot take non-positive number, got {}", f),
                    span: Span {
                        line: 0,
                        column: 0,
                        start: 0,
                        end: 0,
                    },
                });
            }
            Ok(Value::Float(f.ln()))
        }
        other => Err(RuntimeError {
            message: format!("'log' expects a number, got '{}'", other.type_name()),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}
fn native_sin(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("sin", 1, args.len()));
    }
    match &args[0] {
        Value::Int(i) => Ok(Value::Float((*i as f64).sin())),
        Value::Float(f) => Ok(Value::Float(f.sin())),
        other => Err(RuntimeError {
            message: format!("'sin' expects a number, got '{}'", other.type_name()),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}
fn native_cos(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("cos", 1, args.len()));
    }
    match &args[0] {
        Value::Int(i) => Ok(Value::Float((*i as f64).cos())),
        Value::Float(f) => Ok(Value::Float(f.cos())),
        other => Err(RuntimeError {
            message: format!("'cos' expects a number, got '{}'", other.type_name()),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}
fn native_tan(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("tan", 1, args.len()));
    }
    match &args[0] {
        Value::Int(i) => Ok(Value::Float((*i as f64).tan())),
        Value::Float(f) => Ok(Value::Float(f.tan())),
        other => Err(RuntimeError {
            message: format!("'tan' expects a number, got '{}'", other.type_name()),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_sqrt(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("sqrt", 1, args.len()));
    }
    match &args[0] {
        Value::Int(i) => {
            if *i < 0 {
                return Err(RuntimeError {
                    message: format!("'sqrt' cannot take negative number, got {}", i),
                    span: Span {
                        line: 0,
                        column: 0,
                        start: 0,
                        end: 0,
                    },
                });
            }
            Ok(Value::Float((*i as f64).sqrt()))
        }
        Value::Float(f) => {
            if *f < 0.0 {
                return Err(RuntimeError {
                    message: format!("'sqrt' cannot take negative number, got {}", f),
                    span: Span {
                        line: 0,
                        column: 0,
                        start: 0,
                        end: 0,
                    },
                });
            }
            Ok(Value::Float(f.sqrt()))
        }
        other => Err(RuntimeError {
            message: format!("'sqrt' expects a number, got '{}'", other.type_name()),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_max(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(native_err_arg("max", 2, args.len()));
    }
    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(std::cmp::max(*a, *b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.max(*b))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).max(*b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.max(*b as f64))),
        (a, b) => Err(RuntimeError {
            message: format!(
                "'max' expects two numbers, got '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_min(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(native_err_arg("min", 2, args.len()));
    }
    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(std::cmp::min(*a, *b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.min(*b))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).min(*b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.min(*b as f64))),
        (a, b) => Err(RuntimeError {
            message: format!(
                "'min' expects two numbers, got '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_range(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(native_err_arg("range", 2, args.len()));
    }
    match (&args[0], &args[1]) {
        (Value::Int(start), Value::Int(end)) => {
            if start > end {
                return Err(RuntimeError {
                    message: format!("'range' start must be <= end, got {} and {}", start, end),
                    span: Span {
                        line: 0,
                        column: 0,
                        start: 0,
                        end: 0,
                    },
                });
            }
            let range = (*start..*end).map(Value::Int).collect();
            Ok(Value::List(range))
        }
        (a, b) => Err(RuntimeError {
            message: format!(
                "'range' expects two ints, got '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_floor(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("floor", 1, args.len()));
    }
    match &args[0] {
        Value::Int(i) => Ok(Value::Int(*i)),
        Value::Float(f) => Ok(Value::Int(f.floor() as i64)),
        other => Err(RuntimeError {
            message: format!("'floor' expects a number, got '{}'", other.type_name()),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_round(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("round", 1, args.len()));
    }
    match &args[0] {
        Value::Int(i) => Ok(Value::Int(*i)),
        Value::Float(f) => Ok(Value::Int(f.round() as i64)),
        other => Err(RuntimeError {
            message: format!("'round' expects a number, got '{}'", other.type_name()),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_ceil(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("ceil", 1, args.len()));
    }
    match &args[0] {
        Value::Int(i) => Ok(Value::Int(*i)),
        Value::Float(f) => Ok(Value::Int(f.ceil() as i64)),
        other => Err(RuntimeError {
            message: format!("'ceil' expects a number, got '{}'", other.type_name()),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_index_of(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(native_err_arg("indexOf", 2, args.len()));
    }
    match (&args[0], &args[1]) {
        (Value::Str(s), Value::Str(substr)) => {
            Ok(Value::Int(s.find(substr).unwrap_or(usize::MAX) as i64))
        }
        (a, b) => Err(RuntimeError {
            message: format!(
                "'indexOf' expects two strings, got '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_last_index_of(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(native_err_arg("lastIndexOf", 2, args.len()));
    }
    match (&args[0], &args[1]) {
        (Value::Str(s), Value::Str(substr)) => {
            Ok(Value::Int(s.rfind(substr).unwrap_or(usize::MAX) as i64))
        }
        (a, b) => Err(RuntimeError {
            message: format!(
                "'lastIndexOf' expects two strings, got '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_join(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(native_err_arg("join", 2, args.len()));
    }
    match (&args[0], &args[1]) {
        (Value::List(items), Value::Str(delim)) => {
            let mut str_items = Vec::new();
            for item in items {
                match item {
                    Value::Str(s) => str_items.push(s.clone()),
                    other => {
                        return Err(RuntimeError {
                            message: format!(
                            "'join' expects a list of strings as first argument, got list of '{}'",
                            other.type_name()
                        ),
                            span: Span {
                                line: 0,
                                column: 0,
                                start: 0,
                                end: 0,
                            },
                        })
                    }
                }
            }
            Ok(Value::Str(str_items.join(delim)))
        }
        (a, b) => Err(RuntimeError {
            message: format!(
                "'join' expects a list and a string, got '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_reverse(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("reverse", 1, args.len()));
    }
    match &args[0] {
        Value::Str(s) => Ok(Value::Str(s.chars().rev().collect())),
        Value::List(items) => {
            let mut rev_items = items.clone();
            rev_items.reverse();
            Ok(Value::List(rev_items))
        }
        other => Err(RuntimeError {
            message: format!(
                "'reverse' expects a string or a list, got '{}'",
                other.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_slice(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(native_err_arg("slice", 3, args.len()));
    }
    match (&args[0], &args[1], &args[2]) {
        (Value::Str(s), Value::Int(start), Value::Int(len)) => {
            let start = *start as usize;
            let len = *len as usize;
            if start >= s.len() {
                return Ok(Value::Str("".to_string()));
            }
            let end = std::cmp::min(start + len, s.len());
            Ok(Value::Str(s[start..end].to_string()))
        }
        (Value::List(items), Value::Int(start), Value::Int(len)) => {
            let start = *start as usize;
            let len = *len as usize;
            if start >= items.len() {
                return Ok(Value::List(vec![]));
            }
            let end = std::cmp::min(start + len, items.len());
            Ok(Value::List(items[start..end].to_vec()))
        }
        (a, b, c) => Err(RuntimeError {
            message: format!(
                "'slice' expects a string or list and two ints, got '{}', '{}' and '{}'",
                a.type_name(),
                b.type_name(),
                c.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_sort(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("sort", 1, args.len()));
    }
    match &args[0] {
        Value::List(items) => {
            let mut has_numbers = false;
            let mut has_strings = false;
            for item in items {
                match item {
                    Value::Int(_) | Value::Float(_) => {
                        has_numbers = true;
                    }
                    Value::Str(_) => {
                        has_strings = true;
                    }
                    other => {
                        return Err(RuntimeError {
                            message: format!(
                                "'sort' only supports lists of numbers or strings, got list of '{}'",
                                other.type_name()
                            ),
                            span: Span {
                                line: 0,
                                column: 0,
                                start: 0,
                                end: 0,
                            },
                        });
                    }
                }
            }
            if has_numbers && has_strings {
                return Err(RuntimeError {
                    message: "'sort' cannot sort lists with mixed types".to_string(),
                    span: Span {
                        line: 0,
                        column: 0,
                        start: 0,
                        end: 0,
                    },
                });
            } else if has_numbers {
                let mut sorted_items = items.clone();
                sorted_items.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                return Ok(Value::List(sorted_items));
            } else if has_strings {
                let mut sorted_items = items.clone();
                sorted_items.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                return Ok(Value::List(sorted_items));
            } else {
                return Ok(Value::List(items.clone()));
            }
        }
        other => Err(RuntimeError {
            message: format!(
                "'sort' expects a list as first argument, got '{}'",
                other.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_zip(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(native_err_arg("zip", 2, args.len()));
    }
    match (&args[0], &args[1]) {
        (Value::List(list1), Value::List(list2)) => {
            let len = std::cmp::min(list1.len(), list2.len());
            let mut zipped = Vec::new();
            for i in 0..len {
                zipped.push(Value::List(vec![list1[i].clone(), list2[i].clone()]));
            }
            Ok(Value::List(zipped))
        }
        (a, b) => Err(RuntimeError {
            message: format!(
                "'zip' expects two lists, got '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_unzip(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("unzip", 1, args.len()));
    }
    match &args[0] {
        Value::List(pairs) => {
            let mut list1 = Vec::new();
            let mut list2 = Vec::new();
            for pair in pairs {
                match pair {
                    Value::List(items) if items.len() == 2 => {
                        list1.push(items[0].clone());
                        list2.push(items[1].clone());
                    }
                    other => {
                        return Err(RuntimeError {
                            message: format!(
                            "'unzip' expects a list of pairs (lists of length 2), got list of '{}'",
                            other.type_name()
                        ),
                            span: Span {
                                line: 0,
                                column: 0,
                                start: 0,
                                end: 0,
                            },
                        })
                    }
                }
            }
            Ok(Value::List(vec![Value::List(list1), Value::List(list2)]))
        }
        other => Err(RuntimeError {
            message: format!(
                "'unzip' expects a list as first argument, got '{}'",
                other.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_first(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("first", 1, args.len()));
    }
    match &args[0] {
        Value::List(items) => {
            if let Some(first) = items.first() {
                Ok(first.clone())
            } else {
                Err(RuntimeError {
                    message: "'first' cannot be called on an empty list".to_string(),
                    span: Span {
                        line: 0,
                        column: 0,
                        start: 0,
                        end: 0,
                    },
                })
            }
        }
        other => Err(RuntimeError {
            message: format!(
                "'first' expects a list as first argument, got '{}'",
                other.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_last(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(native_err_arg("last", 1, args.len()));
    }
    match &args[0] {
        Value::List(items) => {
            if let Some(last) = items.last() {
                Ok(last.clone())
            } else {
                Err(RuntimeError {
                    message: "'last' cannot be called on an empty list".to_string(),
                    span: Span {
                        line: 0,
                        column: 0,
                        start: 0,
                        end: 0,
                    },
                })
            }
        }
        other => Err(RuntimeError {
            message: format!(
                "'last' expects a list as first argument, got '{}'",
                other.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

fn native_concat(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(native_err_arg("concat", 2, args.len()));
    }
    match (&args[0], &args[1]) {
        (Value::List(list1), Value::List(list2)) => {
            let mut concatenated = list1.clone();
            concatenated.extend(list2.clone());
            Ok(Value::List(concatenated))
        }
        (a, b) => Err(RuntimeError {
            message: format!(
                "'concatenate' expects two lists, got '{}' and '{}'",
                a.type_name(),
                b.type_name()
            ),
            span: Span {
                line: 0,
                column: 0,
                start: 0,
                end: 0,
            },
        }),
    }
}

// ─── Integration tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::semantic::TypeChecker;

    fn run_ok(src: &str) -> Interpreter {
        let tokens = Lexer::new(src).tokenize().expect("lex failed");
        let program = Parser::new(tokens).parse().expect("parse failed");
        // Warnings are non-fatal; only errors fail the test
        if let Err(errors) = TypeChecker::check(&program, std::path::Path::new(".")) {
            panic!("type check failed: {:?}", errors);
        }
        let mut interp = Interpreter::new(PathBuf::from("."));
        interp.run(&program).expect("runtime failed");
        interp
    }

    fn run_err(src: &str) -> RuntimeError {
        let tokens = Lexer::new(src).tokenize().expect("lex failed");
        let program = Parser::new(tokens).parse().expect("parse failed");
        // type check may pass (runtime error only)
        let _ = TypeChecker::check(&program, std::path::Path::new("."));
        let mut interp = Interpreter::new(PathBuf::from("."));
        interp.run(&program).expect_err("expected runtime error")
    }

    fn get(interp: &Interpreter, name: &str) -> Value {
        let span = Span {
            line: 0,
            column: 0,
            start: 0,
            end: 0,
        };
        interp.environment.get(name, span).unwrap()
    }

    // ── Listas ───────────────────────────────────────────────────────────────

    #[test]
    fn test_list_empty() {
        let i = run_ok("let result = [];");
        assert!(matches!(get(&i, "result"), Value::List(ref v) if v.is_empty()));
    }

    #[test]
    fn test_list_literal() {
        let i = run_ok("let result = [1, 2, 3];");
        if let Value::List(items) = get(&i, "result") {
            assert_eq!(items.len(), 3);
            assert!(matches!(items[0], Value::Int(1)));
            assert!(matches!(items[2], Value::Int(3)));
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_list_index_read() {
        let i = run_ok("let arr = [10, 20, 30]; let result = arr[1];");
        assert!(matches!(get(&i, "result"), Value::Int(20)));
    }

    #[test]
    fn test_list_index_write() {
        let i = run_ok("let mut arr = [1, 2, 3]; arr[0] = 99; let result = arr[0];");
        assert!(matches!(get(&i, "result"), Value::Int(99)));
    }

    #[test]
    fn test_list_out_of_bounds() {
        run_err("let arr = [1, 2]; let x = arr[5];");
    }

    // ── Built-ins: len, push, pop ─────────────────────────────────────────────

    #[test]
    fn test_len_list() {
        let i = run_ok("let result = len([1, 2, 3]);");
        assert!(matches!(get(&i, "result"), Value::Int(3)));
    }

    #[test]
    fn test_len_string() {
        let i = run_ok("let result = len(\"hello\");");
        assert!(matches!(get(&i, "result"), Value::Int(5)));
    }

    #[test]
    fn test_push() {
        let i = run_ok("let result = push([1, 2], 3);");
        if let Value::List(items) = get(&i, "result") {
            assert_eq!(items.len(), 3);
            assert!(matches!(items[2], Value::Int(3)));
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_pop() {
        let i = run_ok("let result = pop([1, 2, 3]);");
        if let Value::List(items) = get(&i, "result") {
            assert_eq!(items.len(), 2);
        } else {
            panic!("expected list");
        }
    }

    // ── Built-ins: toString, parseInt, toFloat ────────────────────────────────

    #[test]
    fn test_to_string() {
        let i = run_ok("let result = toString(42);");
        assert!(matches!(get(&i, "result"), Value::Str(ref s) if s == "42"));
    }

    #[test]
    fn test_parse_int() {
        let i = run_ok("let result = parseInt(\"42\");");
        assert!(matches!(get(&i, "result"), Value::Int(42)));
    }

    #[test]
    fn test_to_float() {
        let i = run_ok("let result = toFloat(\"3.14\");");
        if let Value::Float(f) = get(&i, "result") {
            assert!((f - 3.14).abs() < 1e-9);
        } else {
            panic!("expected float");
        }
    }

    #[test]
    fn test_to_float_from_int() {
        let i = run_ok("let result = toFloat(5);");
        assert!(matches!(get(&i, "result"), Value::Float(f) if (f - 5.0).abs() < 1e-9));
    }

    // ── Result: ok, err ───────────────────────────────────────────────────────

    #[test]
    fn test_ok_constructor() {
        let i = run_ok("let result = ok(42);");
        assert!(matches!(get(&i, "result"), Value::Ok(_)));
    }

    #[test]
    fn test_err_constructor() {
        let i = run_ok("let result = err(\"oops\");");
        assert!(matches!(get(&i, "result"), Value::Err(_)));
    }

    #[test]
    fn test_is_ok_true() {
        let i = run_ok("let result = is_ok(ok(1));");
        assert!(matches!(get(&i, "result"), Value::Bool(true)));
    }

    #[test]
    fn test_is_ok_false() {
        let i = run_ok("let result = is_ok(err(\"x\"));");
        assert!(matches!(get(&i, "result"), Value::Bool(false)));
    }

    #[test]
    fn test_is_err_true() {
        let i = run_ok("let result = is_err(err(\"fail\"));");
        assert!(matches!(get(&i, "result"), Value::Bool(true)));
    }

    #[test]
    fn test_unwrap_ok() {
        let i = run_ok("let result = unwrap(ok(99));");
        assert!(matches!(get(&i, "result"), Value::Int(99)));
    }

    #[test]
    fn test_unwrap_err_panics() {
        run_err("let x = unwrap(err(\"oops\"));");
    }

    // ── Option: some, none ────────────────────────────────────────────────────

    #[test]
    fn test_some_constructor() {
        let i = run_ok("let result = some(5);");
        assert!(matches!(get(&i, "result"), Value::Some(_)));
    }

    #[test]
    fn test_none_constructor() {
        let i = run_ok("let result = none();");
        assert!(matches!(get(&i, "result"), Value::None));
    }

    #[test]
    fn test_is_some_true() {
        let i = run_ok("let result = is_some(some(1));");
        assert!(matches!(get(&i, "result"), Value::Bool(true)));
    }

    #[test]
    fn test_is_none_true() {
        let i = run_ok("let result = is_none(none());");
        assert!(matches!(get(&i, "result"), Value::Bool(true)));
    }

    #[test]
    fn test_unwrap_some() {
        let i = run_ok("let result = unwrap(some(7));");
        assert!(matches!(get(&i, "result"), Value::Int(7)));
    }

    #[test]
    fn test_unwrap_none_panics() {
        run_err("let x = unwrap(none());");
    }

    // ── match statement ───────────────────────────────────────────────────────

    #[test]
    fn test_match_ok_arm() {
        let i = run_ok("let mut result = 0; match ok(42) { ok(v) => { result = v; } err(e) => { result = -1; } }");
        assert!(matches!(get(&i, "result"), Value::Int(42)));
    }

    #[test]
    fn test_match_err_arm() {
        let i = run_ok("let mut result = 0; match err(\"fail\") { ok(v) => { result = 1; } err(e) => { result = -1; } }");
        assert!(matches!(get(&i, "result"), Value::Int(-1)));
    }

    #[test]
    fn test_match_some_arm() {
        let i = run_ok("let mut result = 0; match some(99) { some(x) => { result = x; } none => { result = -1; } }");
        assert!(matches!(get(&i, "result"), Value::Int(99)));
    }

    #[test]
    fn test_match_none_arm() {
        let i = run_ok("let mut result = 0; match none() { some(x) => { result = x; } none => { result = -2; } }");
        assert!(matches!(get(&i, "result"), Value::Int(-2)));
    }

    #[test]
    fn test_match_wildcard() {
        let i = run_ok("let mut result = 0; match ok(5) { _ => { result = 99; } }");
        assert!(matches!(get(&i, "result"), Value::Int(99)));
    }

    #[test]
    fn test_match_no_arm_error() {
        run_err("match ok(1) { err(e) => { } }");
    }

    // ── Integración con funciones y control de flujo ──────────────────────────

    #[test]
    fn test_function_with_list() {
        let src = "fn first(n: int) -> int { let arr = [10, 20, 30]; return arr[0]; } let result = first(0);";
        let i = run_ok(src);
        assert!(matches!(get(&i, "result"), Value::Int(10)));
    }

    #[test]
    fn test_push_in_loop() {
        let src = r#"
let mut arr = [];
for i in 0..3 {
    arr = push(arr, i);
}
let result = len(arr);
        "#;
        let i = run_ok(src);
        assert!(matches!(get(&i, "result"), Value::Int(3)));
    }

    // ── Lambdas ───────────────────────────────────────────────────────────────

    #[test]
    fn test_lambda_map() {
        let i = run_ok("let result = map([1, 2, 3], |x| x * 2);");
        if let Value::List(items) = get(&i, "result") {
            assert_eq!(items, vec![Value::Int(2), Value::Int(4), Value::Int(6)]);
        } else { panic!("expected list"); }
    }

    #[test]
    fn test_lambda_filter() {
        let i = run_ok("let result = filter([1, 2, 3, 4, 5], |x| x % 2 == 0);");
        if let Value::List(items) = get(&i, "result") {
            assert_eq!(items, vec![Value::Int(2), Value::Int(4)]);
        } else { panic!("expected list"); }
    }

    #[test]
    fn test_lambda_reduce() {
        let i = run_ok("let result = reduce([1, 2, 3, 4, 5], |acc, x| acc + x, 0);");
        assert!(matches!(get(&i, "result"), Value::Int(15)));
    }

    #[test]
    fn test_lambda_sort_by() {
        let i = run_ok("let result = sortBy([3, 1, 4, 1, 5], |x| x);");
        if let Value::List(items) = get(&i, "result") {
            assert_eq!(items[0], Value::Int(1));
            assert_eq!(items[4], Value::Int(5));
        } else { panic!("expected list"); }
    }

    #[test]
    fn test_lambda_closure_capture() {
        let i = run_ok("let factor = 3; let result = map([1, 2, 3], |x| x * factor);");
        if let Value::List(items) = get(&i, "result") {
            assert_eq!(items, vec![Value::Int(3), Value::Int(6), Value::Int(9)]);
        } else { panic!("expected list"); }
    }

    #[test]
    fn test_lambda_block_body() {
        let i = run_ok(r#"let result = map([1, 2, 3], |x| { return x + 10; });"#);
        if let Value::List(items) = get(&i, "result") {
            assert_eq!(items, vec![Value::Int(11), Value::Int(12), Value::Int(13)]);
        } else { panic!("expected list"); }
    }

    #[test]
    fn test_lambda_assigned_to_variable() {
        let i = run_ok("let double = |x| x * 2; let result = double(5);");
        assert!(matches!(get(&i, "result"), Value::Int(10)));
    }

    #[test]
    fn test_lambda_zero_params() {
        let i = run_ok("let f = || 42; let result = f();");
        assert!(matches!(get(&i, "result"), Value::Int(42)));
    }

    // ── Operador ? ──────────────────────────────────────────────────────────

    #[test]
    fn test_try_unwraps_ok() {
        // ok(v)? devuelve v directamente
        let src = r#"
fn get_value() -> int {
    let r = ok(42);
    let v = r?;
    return v;
}
let result = get_value();
"#;
        let i = run_ok(src);
        assert!(matches!(get(&i, "result"), Value::Int(42)));
    }

    #[test]
    fn test_try_propagates_err() {
        // err(e)? sale de la funcion devolviendo err(e)
        let src = r#"
fn might_fail() -> Result<int, string> {
    let r = err("oops");
    let v = r?;
    return ok(v + 1);
}
let result = might_fail();
"#;
        let i = run_ok(src);
        assert!(matches!(get(&i, "result"), Value::Err(_)));
        if let Value::Err(inner) = get(&i, "result") {
            assert!(matches!(*inner, Value::Str(ref s) if s == "oops"));
        }
    }

    #[test]
    fn test_try_unwraps_some() {
        // some(v)? devuelve v
        let src = r#"
fn get_opt() -> int {
    let o = some(7);
    let v = o?;
    return v;
}
let result = get_opt();
"#;
        let i = run_ok(src);
        assert!(matches!(get(&i, "result"), Value::Int(7)));
    }

    #[test]
    fn test_try_propagates_none() {
        // none? sale de la funcion devolviendo none
        let src = r#"
fn might_none() -> Option<int> {
    let o = none();
    let v = o?;
    return some(v + 1);
}
let result = might_none();
"#;
        let i = run_ok(src);
        assert!(matches!(get(&i, "result"), Value::None));
    }

    #[test]
    fn test_try_inline_on_call() {
        // ? sobre el resultado de una llamada directa
        let src = r#"
fn safe_div(a: int, b: int) -> Result<int, string> {
    if b == 0 {
        return err("div by zero");
    }
    return ok(a / b);
}
fn compute() -> Result<int, string> {
    let v = safe_div(10, 2)?;
    return ok(v * 3);
}
let result = compute();
"#;
        let i = run_ok(src);
        if let Value::Ok(inner) = get(&i, "result") {
            assert!(matches!(*inner, Value::Int(15)));
        } else {
            panic!("expected ok(15)");
        }
    }

    // ── F-strings ─────────────────────────────────────────────────────────────

    fn run_warn(src: &str) -> Vec<crate::utils::error::SemanticWarning> {
        let tokens = Lexer::new(src).tokenize().expect("lex failed");
        let program = Parser::new(tokens).parse().expect("parse failed");
        match TypeChecker::check(&program, std::path::Path::new(".")) {
            Ok(warnings) => warnings,
            Err(errors) => panic!("type check failed: {:?}", errors),
        }
    }

    #[test]
    fn test_fstring_plain_literal() {
        // f-string sin interpolaciones → igual que un string normal
        let i = run_ok(r#"let result = f"hello world";"#);
        assert!(matches!(get(&i, "result"), Value::Str(ref s) if s == "hello world"));
    }

    #[test]
    fn test_fstring_single_var() {
        // f"Hola {name}" con variable entera
        let i = run_ok(r#"let name = "mundo"; let result = f"Hola {name}";"#);
        assert!(matches!(get(&i, "result"), Value::Str(ref s) if s == "Hola mundo"));
    }

    #[test]
    fn test_fstring_int_interpolation() {
        // Interpolación de int → se convierte a string
        let i = run_ok(r#"let x = 42; let result = f"valor: {x}";"#);
        assert!(matches!(get(&i, "result"), Value::Str(ref s) if s == "valor: 42"));
    }

    #[test]
    fn test_fstring_float_interpolation() {
        // Interpolación de float
        let i = run_ok(r#"let pi = 3.14; let result = f"pi es {pi}";"#);
        assert!(matches!(get(&i, "result"), Value::Str(ref s) if s == "pi es 3.14"));
    }

    #[test]
    fn test_fstring_bool_interpolation() {
        let i = run_ok(r#"let ok = true; let result = f"es: {ok}";"#);
        assert!(matches!(get(&i, "result"), Value::Str(ref s) if s == "es: true"));
    }

    #[test]
    fn test_fstring_expression() {
        // Expresión aritmética dentro de {}
        let i = run_ok(r#"let a = 3; let b = 4; let result = f"suma: {a + b}";"#);
        assert!(matches!(get(&i, "result"), Value::Str(ref s) if s == "suma: 7"));
    }

    #[test]
    fn test_fstring_multiple_parts() {
        // Varios {expr} en el mismo f-string
        let i = run_ok(r#"let x = 1; let y = 2; let result = f"{x} + {y} = {x + y}";"#);
        assert!(matches!(get(&i, "result"), Value::Str(ref s) if s == "1 + 2 = 3"));
    }

    #[test]
    fn test_fstring_function_call_in_expr() {
        // Llamada a función dentro de {}
        let src = r#"
fn double(n: int) -> int { return n * 2; }
let result = f"doble de 5 es {double(5)}";
"#;
        let i = run_ok(src);
        assert!(matches!(get(&i, "result"), Value::Str(ref s) if s == "doble de 5 es 10"));
    }

    // ── User-defined Enums ────────────────────────────────────────────────────

    #[test]
    fn test_enum_variant_value() {
        // Color::Red produce un Value::EnumVariant
        let src = r#"
enum Color { Red, Green, Blue }
let result = Color::Red;
"#;
        let i = run_ok(src);
        if let Value::EnumVariant { enum_name, variant } = get(&i, "result") {
            assert_eq!(enum_name, "Color");
            assert_eq!(variant, "Red");
        } else {
            panic!("expected EnumVariant");
        }
    }

    #[test]
    fn test_enum_display() {
        // to_display_string() produce "Color::Red"
        let src = r#"
enum Color { Red, Green, Blue }
let result = Color::Green;
"#;
        let i = run_ok(src);
        assert_eq!(get(&i, "result").to_display_string(), "Color::Green");
    }

    #[test]
    fn test_enum_match_arm() {
        // match sobre enum variant
        let src = r#"
enum Dir { North, South, East, West }
let d = Dir::South;
let mut result = 0;
match d {
    Dir::North => { result = 1; }
    Dir::South => { result = 2; }
    Dir::East  => { result = 3; }
    _          => { result = 4; }
}
"#;
        let i = run_ok(src);
        assert!(matches!(get(&i, "result"), Value::Int(2)));
    }

    #[test]
    fn test_enum_equality() {
        // Dos variantes del mismo enum son iguales si tienen el mismo nombre
        let src = r#"
enum Status { Active, Inactive }
let a = Status::Active;
let b = Status::Active;
let result = a == b;
"#;
        let i = run_ok(src);
        assert!(matches!(get(&i, "result"), Value::Bool(true)));
    }

    #[test]
    fn test_enum_inequality() {
        let src = r#"
enum Status { Active, Inactive }
let a = Status::Active;
let b = Status::Inactive;
let result = a == b;
"#;
        let i = run_ok(src);
        assert!(matches!(get(&i, "result"), Value::Bool(false)));
    }

    #[test]
    fn test_enum_in_function() {
        // Función que recibe y devuelve enum variant
        let src = r#"
enum Light { Red, Yellow, Green }
fn next_light(l: Light) -> Light {
    match l {
        Light::Red    => { return Light::Green; }
        Light::Green  => { return Light::Yellow; }
        _             => { return Light::Red; }
    }
}
let result = next_light(Light::Red);
"#;
        let i = run_ok(src);
        if let Value::EnumVariant { variant, .. } = get(&i, "result") {
            assert_eq!(variant, "Green");
        } else {
            panic!("expected EnumVariant");
        }
    }

    #[test]
    fn test_enum_in_list() {
        // Lista de enum variants
        let src = r#"
enum Color { Red, Green, Blue }
let colors = [Color::Red, Color::Green, Color::Blue];
let result = len(colors);
"#;
        let i = run_ok(src);
        assert!(matches!(get(&i, "result"), Value::Int(3)));
    }

    // ── Match wildcard warning ────────────────────────────────────────────────

    #[test]
    fn test_match_wildcard_no_warning_with_wildcard() {
        // Con _ no debe haber warning
        let src = r#"
fn get() -> Result<int, string> { return ok(1); }
let r = get();
match r {
    ok(v) => { let a = v; }
    _ => { let a = 0; }
}
"#;
        let warnings = run_warn(src);
        assert!(
            warnings.iter().all(|w| !w.message.contains("wildcard") && !w.message.contains("catch-all")),
            "unexpected wildcard warning: {:?}", warnings
        );
    }

    #[test]
    fn test_match_wildcard_no_warning_with_ident_catch_all() {
        // Con un ident como catch-all tampoco debe haber warning
        let src = r#"
fn get() -> Result<int, string> { return ok(1); }
let r = get();
match r {
    ok(v) => { let a = v; }
    other => { let a = 0; }
}
"#;
        let warnings = run_warn(src);
        assert!(
            warnings.iter().all(|w| !w.message.contains("wildcard") && !w.message.contains("catch-all")),
            "unexpected wildcard warning: {:?}", warnings
        );
    }

    #[test]
    fn test_match_wildcard_warning_without_wildcard() {
        // Sin _ ni catch-all debe emitir un warning
        let src = r#"
fn get() -> Result<int, string> { return ok(1); }
let r = get();
match r {
    ok(v) => { let a = v; }
    err(e) => { let a = 0; }
}
"#;
        let warnings = run_warn(src);
        assert!(
            warnings.iter().any(|w| w.message.contains("wildcard") || w.message.contains("catch-all")),
            "expected wildcard warning but got: {:?}", warnings
        );
    }
}
