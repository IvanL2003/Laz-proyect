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
    Enum(EnumDecl),
    Connect(ConnectDecl),
    Statement(Stmt),
    // package math;
    Package { name: String, span: Span },
    // import "path.lz";  /  import math;  /  import { cos } from math;  etc.
    Import { kind: ImportKind, span: Span },
}

// ─── Import kinds ──────────────────────────────────────────────────────────────

/// Qué se importa y cómo
#[derive(Debug, Clone)]
pub enum ImportKind {
    // import "path.lz";
    // import "path.lz" as alias;
    Path { path: String, alias: Option<String> },

    // import math;
    // import math as m;
    Named { package: String, alias: Option<String> },

    // import { cos, sin } from math;
    // import { cos, sin } from "math.lz";
    // import { cos as coseno } from math;
    Selective { source: ImportSource, items: Vec<ImportItem> },
}

/// De dónde viene un import selectivo
#[derive(Debug, Clone)]
pub enum ImportSource {
    Path(String),   // from "math.lz"
    Named(String),  // from math
}

/// Un ítem en un import selectivo: cos  o  cos as coseno
#[derive(Debug, Clone)]
pub struct ImportItem {
    pub name: String,
    pub alias: Option<String>,
}

// enum Color { Red, Green, Blue }
// name="Color"  variants=["Red", "Green", "Blue"]
#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<String>,
    pub span: Span,
}

// connect file "users.csv" as users;
// connect_type=File  file_path="users.csv"  alias="users"
//
// connect db "data.db" as mydb { User from users, Product from products };
// connect_type=Db  file_path="data.db"  alias="mydb"
// mappings=[DbMapping{struct_name="User", table_name="users"}, ...]
#[derive(Debug, Clone)]
pub struct ConnectDecl {
    pub connect_type: ConnectType, // file | db | api
    pub file_path: String,         // ruta al archivo
    pub alias: String,             // nombre con el que se referencia en SQL
    pub mappings: Vec<DbMapping>,  // solo para db: struct <-> tabla
    pub span: Span,
}

// Un mapping dentro de connect db:  User from users
// struct_name="User"  table_name="users"
#[derive(Debug, Clone)]
pub struct DbMapping {
    pub struct_name: String, // nombre del struct Laz a auto-generar
    pub table_name: String,  // nombre de la tabla en el archivo .db
}

// fn distance(p1: Point, p2: Point) -> float { ... }
// name="distance"  params=[p1:Point, p2:Point]  return_type=Float
//
// fn identity<T>(x: T) -> T { ... }
// name="identity"  type_params=["T"]  params=[x:UserDefined("T")]  return_type=UserDefined("T")
#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: String,
    pub type_params: Vec<String>,        // [] = no generics, ["T"] = one, ["A","B"] = two, etc.
    pub params: Vec<Param>,
    pub return_type: TypeAnnotation,
    pub body: Block,
    pub span: Span,
}

// p1: Point   (un parametro dentro de la declaracion de funcion)
// name="p1"   type_ann=UserDefined("Point")
#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub span: Span,
}

// struct User { name: string, age: int }
// name="User"  fields=[{name:string}, {age:int}]
//
// struct Pair<A, B> { first: A, second: B }
// name="Pair"  type_params=["A","B"]  fields=[{first:UserDefined("A")}, {second:UserDefined("B")}]
#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub type_params: Vec<String>,        // [] = concrete struct, ["A","B"] = generic struct
    pub fields: Vec<StructField>,
    pub span: Span,
}

// name: string   (un campo dentro de la definicion de struct)
// name="name"    type_ann=StringType
#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub span: Span,
}

// Representacion de un tipo en el codigo fuente
//   Int                  -->  int
//   Float                -->  float
//   Bool                 -->  bool
//   StringType           -->  string
//   Void                 -->  void
//   List(T)              -->  list<T>   ej: list<User>, list<list<string>>
//   Result(T, E)         -->  Result<T, E>  ej: Result<int, string>
//   Option(T)            -->  Option<T>     ej: Option<float>
//   UserDefined(String)  -->  nombre de struct o parametro de tipo   ej: User, Point, T
//   Generic(Name, args)  -->  instanciacion generica  ej: Pair<int, string>, Stack<User>
#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnotation {
    Int,
    Float,
    Bool,
    StringType,
    Void,
    List(Box<TypeAnnotation>),                                    // list<T>
    Dict(Box<TypeAnnotation>, Box<TypeAnnotation>),              // dict<K, V>
    Result(Box<TypeAnnotation>, Box<TypeAnnotation>),             // Result<T, E>
    Option(Box<TypeAnnotation>),                                  // Option<T>
    UserDefined(String),                                          // nombre del struct o tipo param
    Generic(String, Vec<TypeAnnotation>),                         // Pair<int, string>
}

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    // let x: int = 42;   mutable=false, type_ann=Some(Int), initializer=IntLiteral(42)
    // let mut count = 0;  mutable=true,  type_ann=None,      initializer=IntLiteral(0)
    //
    // initializer es cualquier Expr (incluido SqlSelect, FnCall, etc.)
    // Si hay type_ann, se verifica compatibilidad despues de evaluar el initializer.
    Let {
        name: String,
        mutable: bool,
        type_ann: Option<TypeAnnotation>, // None = infiere del initializer
        initializer: Expr,
        span: Span,
    },

    // x = 5;       target=Variable("x")
    // p.x = 1.0;   target=FieldAccess { object=Identifier("p"), field="x" }
    Assign {
        target: AssignTarget,
        value: Expr,
        span: Span,
    },

    // if cond { ... }
    // if cond { ... } else { ... }
    // if cond1 { ... } else if cond2 { ... } else if cond3 { ... } else { ... }
    //
    // "else if" es azucar sintactica — el parser lo desazucara a else { if ... }:
    //
    //   if c1 { ... } else if c2 { ... } else { ... }
    //
    //   Stmt::If { c1, else_block: Some(Block {
    //     statements: [ Stmt::If { c2, else_block: Some(Block { ... }) } ]
    //   })}
    //
    // Cadenas arbitrariamente largas funcionan por la misma recursion.
    // else_block=None = sin rama else; else_block=Some = else o else-if desazucarado.
    If {
        condition: Expr,
        then_block: Block,
        else_block: Option<Block>, // None = sin else
        span: Span,
    },

    // while n > 0 { ... }
    While {
        condition: Expr,
        body: Block,
        span: Span,
    },

    // for i in 1..10 { ... }
    // variable="i"  start=IntLiteral(1)  end=IntLiteral(10)  [extremo final exclusivo]
    For {
        variable: String,
        start: Expr,
        end: Expr,
        body: Block,
        span: Span,
    },
    ForEach {
        variable: Vec<String>,
        iterable: Box<Expr>,
        body: Block,
        span: Span,
    },

    // return 42;   value=Some(IntLiteral(42))
    // return;      value=None  -->  devuelve Void implicitamente
    Return {
        value: Option<Expr>,
        span: Span,
    },

    // print("hello", x, 42);
    // args=[StringLiteral("hello"), Identifier("x"), IntLiteral(42)]
    // los argumentos se imprimen separados por espacio
    Print {
        args: Vec<Expr>,
        span: Span,
    },

    // factorial(5);  o cualquier expresion usada como statement (resultado descartado)
    Expression {
        expr: Expr,
        span: Span,
    },

    // match expr { pattern => { ... } ... }
    // Cada arm vincula variables del pattern en su body.
    // ej: match result { ok(v) => { print(v); } err(e) => { print(e); } }
    Match {
        subject: Expr,
        arms: Vec<MatchArm>,
        span: Span,
    },

    // break; — sale del bucle más cercano (while / for / for..in)
    Break { span: Span },

    // continue; — salta a la siguiente iteracion del bucle más cercano
    Continue { span: Span },
}

// x = 5;      -->  Variable("x")
// p.x = 1.0;  -->  FieldAccess { object=Identifier("p"), field="x" }
// arr[i] = v; -->  Index { object="arr", index=Identifier("i") }
#[derive(Debug, Clone)]
pub enum AssignTarget {
    Variable(String),
    FieldAccess { object: Box<Expr>, field: String },
    Index { object: String, index: Box<Expr> },
}

// Un arm de match: pattern => { body }
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Block,
}

// Patron de match
//   Ok(x)   --> ok(x)    extrae el valor Ok y lo bindea como x
//   Err(x)  --> err(x)   extrae el valor Err y lo bindea como x
//   Some(x) --> some(x)  extrae el valor Some y lo bindea como x
//   None    --> none      sin binding
//   Wildcard --> _        case default, sin binding
//   Ident(x) --> x        bindea cualquier valor como x (como wildcard con nombre)
//   EnumVariant --> Color::Red   variant of a user-defined enum
#[derive(Debug, Clone)]
pub enum Pattern {
    Ok(String),
    Err(String),
    Some(String),
    None,
    Wildcard,
    Ident(String),
    EnumVariant { enum_name: String, variant: String },
}

/// A segment of an f-string after parsing.
#[derive(Debug, Clone)]
pub enum AstFStringPart {
    Literal(String),    // plain text
    Expr(Box<Expr>),    // parsed expression
}

#[derive(Debug, Clone)]
pub enum Expr {
    // 42, -7
    IntLiteral { value: i64, span: Span },

    // 3.14, 1.0
    FloatLiteral { value: f64, span: Span },

    // "hello world"
    StringLiteral { value: String, span: Span },

    // f"Hola {name}, tienes {age} años"
    FString { parts: Vec<AstFStringPart>, span: Span },

    // true  |  false
    BoolLiteral { value: bool, span: Span },

    // x, name, users  (referencia a variable — se resuelve buscando en el Environment)
    Identifier { name: String, span: Span },

    // left op right
    // ej: x + 1,  a == b,  p && q,  n % 2
    // op es BinaryOp (Add/Sub/Mul/Div/Mod/Eq/Neq/Lt/Lte/Gt/Gte/And/Or)
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        span: Span,
    },

    // op operand
    // -x   (Neg, solo int/float)
    // !flag (Not, solo bool)
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },

    // nombre(arg1, arg2, ...)
    // ej: factorial(5),  typeOf(x),  greet("Bob", true)
    // callee es el nombre de la funcion (user-defined o built-in)
    FnCall {
        callee: String,
        args: Vec<Expr>,
        span: Span,
    },

    // objeto.campo
    // ej: p.x,  user.name,  bob.age
    // object es la expresion antes del punto (normalmente Identifier)
    FieldAccess {
        object: Box<Expr>,
        field: String,
        span: Span,
    },

    // NombreStruct { campo1: expr1, campo2: expr2 }
    // ej: Point { x: 1.0, y: 2.0 }
    // fields es Vec de pares (nombre_campo, expresion_valor)
    StructInit {
        name: String,
        fields: Vec<(String, Expr)>,
        span: Span,
    },

    // (expr)
    // ej: (x + 1) * 2   -->  Grouped { expr=BinaryOp(x+1) }
    Grouped {
        expr: Box<Expr>,
        span: Span,
    },

    // [e1, e2, e3]
    // ej: [1, 2, 3]  -->  ListLiteral { elements=[IntLiteral(1), IntLiteral(2), IntLiteral(3)] }
    // lista vacia: []
    ListLiteral {
        elements: Vec<Expr>,
        span: Span,
    },

    DictLiteral {
        entries: Vec<(Expr, Expr)>, // Vec de pares (clave, valor)
        span: Span,
    },

    // objeto[indice]
    // ej: arr[0],  users[i],  matrix[j]
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },

    // Funcion anonima (closure / lambda)
    // |x, y| { return x + y; }
    // |n|    { return n % 2 == 0; }
    // || { return 42; }   (cero params)
    //
    // params  -> nombres de los parametros (sin anotacion de tipo)
    // body    -> bloque de codigo; `|x| expr` se desazucara a `|x| { return expr; }`
    Lambda {
        params: Vec<String>,
        body:   Block,
        span:   Span,
    },

    // #SELECT cols FROM tabla WHERE cond
    // #SELECT SINGLE * FROM users WHERE name == "Bob"
    // #SELECT name, age FROM file("data.csv") WHERE age > 18
    //
    // columns: ["*"] = todas, o nombres explícitos ej: ["name", "age"]
    // table_ref: Alias("users") = usa alias de connect | Inline("data.csv") = ruta directa
    // condition: None = sin WHERE | Some(expr) = condicion (usa variables del scope temporal)
    // single: true = SELECT SINGLE devuelve un solo valor (no lista)
    SqlSelect {
        columns: Vec<String>,         // column names (* = all)
        table_ref: SqlTableRef,       // alias or csv("file")
        condition: Option<Box<Expr>>, // WHERE clause (reuses normal Expr)
        single: bool,                 // true for #SELECT SINGLE
        span: Span,
    },

    // #INSERT INTO tabla VALUES (expr1, expr2, ...)
    // ej: #INSERT INTO users VALUES ("Frank", 40, "Bilbao")
    // values son las expresiones en el mismo orden que las columnas del archivo
    SqlInsert {
        table_ref: SqlTableRef,       // alias or csv("file")
        values: Vec<Expr>,            // VALUES (...)
        span: Span,
    },

    // expr?
    // Operador de propagacion para Result y Option:
    //   ok(v)?    -->  devuelve v (unwrap)
    //   err(e)?   -->  return err(e) desde la funcion actual
    //   some(v)?  -->  devuelve v (unwrap)
    //   none?     -->  return none desde la funcion actual
    Try {
        expr: Box<Expr>,
        span: Span,
    },

    // Color::Red   (acceso a variante de enum user-defined)
    // enum_name="Color"  variant="Red"
    EnumVariant {
        enum_name: String,
        variant: String,
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
            | Expr::ListLiteral { span, .. }
            | Expr::DictLiteral { span, .. }
            | Expr::Index { span, .. }
            | Expr::Lambda { span, .. }
            | Expr::FString { span, .. }
            | Expr::SqlSelect { span, .. }
            | Expr::SqlInsert { span, .. }
            | Expr::Try { span, .. }
            | Expr::EnumVariant { span, .. } => *span,
        }
    }
}

// FROM users           -->  Alias("users")      usa el alias definido en connect
// FROM file("x.csv")   -->  Inline("x.csv")     ruta directa al archivo
#[derive(Debug, Clone)]
pub enum SqlTableRef {
    Alias(String),              // FROM users (uses connect alias)
    Inline(String),             // FROM file("users.csv") (direct file path)
}

// Operadores binarios (entre dos expresiones)
//   Add --> +          Sub --> -
//   Mul --> *          Div --> /          Mod --> %
//   Eq  --> ==         Neq --> !=
//   Lt  --> <          Lte --> <=
//   Gt  --> >          Gte --> >=
//   And --> &&  (short-circuit: no evalua right si left es false)
//   Or  --> ||  (short-circuit: no evalua right si left es true)
#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Lte, Gt, Gte,
    And, Or,
}

// Operadores unarios (sobre una expresion)
//   Neg --> -x   (solo int/float)
//   Not --> !x   (solo bool)
#[derive(Debug, Clone)]
pub enum UnaryOp {
    Neg, // -x
    Not, // !x
}
