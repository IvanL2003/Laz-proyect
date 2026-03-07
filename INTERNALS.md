# Laz — Internos del Interprete

Documentacion detallada de cada archivo, struct, enum y funcion del proyecto.

---

## Indice

1. [Estructura de archivos](#estructura-de-archivos)
2. [Flujo global de ejecucion](#flujo-global-de-ejecucion)
3. [src/main.rs](#srcmainrs)
4. [src/lib.rs](#srclibrs)
5. [src/cli.rs](#srccilrs)
6. [src/lexer/](#srclexer)
7. [src/parser/](#srcparser)
8. [src/semantic/](#srcsemantic)
9. [src/codegen/](#srccodegen)
10. [src/formatter/](#srcformatter)
11. [src/utils/](#srcutils)

---

## Estructura de archivos

```
laz/src/
 ├── main.rs                  Entry point: carga args, llama pipeline
 ├── lib.rs                   Declara todos los modulos publicos
 ├── cli.rs                   Parseo de argumentos de linea de comandos
 │
 ├── lexer/
 │   ├── mod.rs               Lexer: convierte source string en tokens
 │   └── token.rs             Token, TokenKind, Span, Comment, lookup_keyword
 │
 ├── parser/
 │   ├── ast.rs               Nodos del AST (tipos de datos puros)
 │   └── parser.rs            Parser recursive descent
 │
 ├── semantic/
 │   └── type_checker.rs      Analisis semantico pre-ejecucion
 │
 ├── codegen/
 │   └── interpreter.rs       Interprete tree-walking (ejecuta el AST)
 │
 ├── formatter/
 │   └── formatter.rs         Pretty-printer con preservacion de comentarios
 │
 └── utils/
     ├── error.rs             Tipos de error y formateador de mensajes
     └── csv.rs               DataTable: carga y guardado de archivos CSV/JSON
```

---

## Flujo global de ejecucion

### `laz programa.lz`

```
main()
  │
  ├─ cli::parse_args()          Detecta Command::Run
  │
  └─ run_program(config)
        │
        ├─ Lexer::new(source).tokenize()
        │     └── Vec<Token>
        │
        ├─ Parser::new(tokens).parse()
        │     └── Program (AST)
        │
        ├─ TypeChecker::check(&program)
        │     └── Ok(()) o Vec<SemanticError>
        │
        └─ Interpreter::new(base_dir).run(&program)
              └── stdout / RuntimeError
```

### `laz fmt programa.lz [--write]`

```
main()
  │
  ├─ cli::parse_args()          Detecta Command::Format
  │
  └─ format_file(config)
        │
        ├─ Lexer::new(source).tokenize_with_comments()
        │     └── (Vec<Token>, Vec<Comment>)
        │
        ├─ Parser::new(tokens).parse()
        │     └── Program (AST)
        │
        └─ Formatter::new(comments).format(&program)
              └── String formateada → stdout o sobreescribe archivo
```

---

## src/main.rs

Punto de entrada del binario. Orquesta los dos pipelines.

### `fn main()`

```
Llama cli::parse_args()
  ├── Command::Run    → run_program(config)
  └── Command::Format → format_file(config)
```

### `fn run_program(config: RunConfig)`

Ejecuta un programa `.lz`. Pipeline completo: lex → parse → typecheck → interpret.

```
Pasos:
  1. Calcula base_dir a partir del path del archivo
     (para resolver rutas relativas de CSV)
  2. Lexer::new(source).tokenize()
     → error L001 si falla
  3. Parser::new(tokens).parse()
     → error P001 si falla
  4. TypeChecker::check(&program)
     → errores S001 si falla
  5. Interpreter::new(base_dir).run(&program)
     → error R001 si falla
```

**base_dir**: directorio del archivo fuente. Se usa como raiz para resolver
rutas relativas en `connect file "users.csv"` y `file("datos.csv")`.

### `fn format_file(config: FormatConfig)`

Formatea un archivo `.lz`.

```
Pasos:
  1. Lexer::new(source).tokenize_with_comments()
     → obtiene tokens Y lista de comentarios separada
  2. Parser::new(tokens).parse()
  3. Formatter::new(comments).format(&program)
     → String formateada
  4. Si --write: fs::write(filename, formatted)
     Si no: print! a stdout
```

La diferencia clave con `run_program`: usa `tokenize_with_comments()` en lugar
de `tokenize()` para preservar los comentarios en el output.

---

## src/lib.rs

Solo declara los modulos del crate para que sean accesibles desde `main.rs`
y los tests.

```rust
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod codegen;
pub mod formatter;
pub mod utils;
pub mod cli;
```

---

## src/cli.rs

Parseo de argumentos de linea de comandos. Sin dependencias externas.

### `enum Command`

```
Command
  ├── Run(RunConfig)     laz programa.lz
  └── Format(FormatConfig)  laz fmt programa.lz [--write]
```

### `struct RunConfig`

```
RunConfig {
  filename: String,   path al archivo (para mensajes de error)
  source:   String,   contenido del archivo (ya leido)
}
```

### `struct FormatConfig`

```
FormatConfig {
  filename:       String,
  source:         String,
  write_in_place: bool,   true si se paso --write
}
```

### `fn parse_args() -> Result<Command, String>`

```
Lee env::args()

  args[1] == "--help" | "-h"
    → println! ayuda y std::process::exit(0)

  args[1] == "--version" | "-v"
    → println! version y exit(0)

  args[1] == "fmt"
    → busca filename (primer arg sin "--")
    → detecta si hay "--write"
    → lee archivo con fs::read_to_string
    → Ok(Command::Format(...))

  args[1] == cualquier otra cosa
    → trata como filename
    → lee archivo
    → Ok(Command::Run(...))
```

---

## src/lexer/

### token.rs

Definiciones de tipos de tokens. Sin logica de parseo.

#### `struct Span`

Posicion de un token en el codigo fuente.

```
Span {
  line:   usize,   numero de linea (1-indexed)
  column: usize,   numero de columna (1-indexed)
  start:  usize,   offset absoluto (inicio del token)
  end:    usize,   offset absoluto (fin del token)
}
```

Usado por: todos los nodos del AST para reportar errores con contexto.

#### `struct Token`

```
Token {
  kind: TokenKind,   tipo del token
  span: Span,        posicion en el fuente
}
```

#### `enum ConnectType`

```
ConnectType
  ├── File   "file"
  ├── Db     "db"
  └── Api    "api"
```

#### `enum TokenKind`

Todos los posibles tokens del lenguaje:

```
TokenKind
  │
  ├── Literales
  │     IntLiteral(i64)       42, -7
  │     FloatLiteral(f64)     3.14
  │     StringLiteral(String) "hello"
  │     BoolLiteral(bool)     true, false
  │
  ├── Identificador
  │     Ident(String)          x, name, myVar
  │
  ├── Keywords generales
  │     Fn, Let, Mut, If, Else, While, For, In
  │     Return, Print, Struct, Connect, File, Db, Api, As
  │
  ├── Keywords de tipos
  │     IntType, FloatType, BoolType, StringType, VoidType, ListType
  │
  ├── Operadores aritmeticos
  │     Plus(+), Minus(-), Star(*), Slash(/), Percent(%)
  │
  ├── Operadores de comparacion
  │     EqualEqual(==), BangEqual(!=)
  │     Less(<), LessEqual(<=), Greater(>), GreaterEqual(>=)
  │
  ├── Operadores logicos
  │     And(&&), Or(||), Bang(!)
  │
  ├── Asignacion
  │     Equal(=)
  │
  ├── Puntuacion
  │     LeftParen, RightParen       (  )
  │     LeftBrace, RightBrace       {  }
  │     LeftBracket, RightBracket   [  ]
  │     Comma(,), Dot(.), Semicolon(;)
  │     Colon(:), Arrow(->), DotDot(..)
  │
  ├── SQL
  │     Hash(#), Select, Single, From, Where
  │     Insert, Into, Values
  │
  └── Especial
        Eof
```

#### `struct Comment`

Comentarios recolectados por el lexer en un canal separado.

```
Comment {
  text:      String,   contenido sin el "//"
  line:      usize,
  column:    usize,
  is_inline: bool,     true si esta en la misma linea que codigo
}
```

#### `fn lookup_keyword(ident: &str) -> Option<TokenKind>`

Convierte un string en keyword si coincide, None si es identificador de usuario.

```
"fn"      → Some(Fn)
"let"     → Some(Let)
"mut"     → Some(Mut)
"if"      → Some(If)
"else"    → Some(Else)
"while"   → Some(While)
"for"     → Some(For)
"in"      → Some(In)
"return"  → Some(Return)
"print"   → Some(Print)
"struct"  → Some(Struct)
"connect" → Some(Connect)
"file"    → Some(File)
"as"      → Some(As)
"true"    → Some(BoolLiteral(true))
"false"   → Some(BoolLiteral(false))
"int"     → Some(IntType)
"float"   → Some(FloatType)
"bool"    → Some(BoolType)
"string"  → Some(StringType)
"void"    → Some(VoidType)
"list"    → Some(ListType)
"SELECT"  → Some(Select)
"SINGLE"  → Some(Single)
"FROM"    → Some(From)
"WHERE"   → Some(Where)
"INSERT"  → Some(Insert)
"INTO"    → Some(Into)
"VALUES"  → Some(Values)
_         → None (es un Ident)
```

---

### lexer/mod.rs

El lexer convierte el codigo fuente (String) en `Vec<Token>`.

#### `struct Lexer`

```
Lexer {
  source:          Vec<char>,   codigo como slice de chars
  pos:             usize,       posicion actual
  line:            usize,       linea actual (1-indexed)
  column:          usize,       columna actual (1-indexed)
  comments:        Vec<Comment>,canal para comentarios
  last_token_line: usize,       linea del ultimo token (para is_inline)
}
```

#### Flujo de tokenizacion

```
tokenize() / tokenize_with_comments()
  │
  └── loop: mientras !is_at_end()
        │
        ├── skip_whitespace()     avanza pos, actualiza line/column
        │
        ├── Si es "//":
        │     tokenize()              → skip_line_comment() (descarta)
        │     tokenize_with_comments()→ collect_comment() (guarda en Vec<Comment>)
        │
        └── scan_token()
              │
              ├── char por char:
              │     '('→LeftParen, ')'→RightParen, '{'→LeftBrace, etc.
              │     '#'→ Hash
              │     '-'→ Minus, '->'→ Arrow
              │     '.'→ Dot, '..'→ DotDot
              │     '='→ Equal, '=='→ EqualEqual
              │     '!'→ Bang, '!='→ BangEqual
              │     '<'→ Less, '<='→ LessEqual
              │     '>'→ Greater, '>='→ GreaterEqual
              │     '&'+'&'→ And, '|'+'|'→ Or
              │
              ├── '"'→ scan_string()
              │         lee hasta '"' de cierre, maneja \"
              │
              ├── '0'..'9'→ scan_number()
              │         lee digitos, si hay '.' lee parte decimal
              │         → IntLiteral o FloatLiteral
              │
              └── 'a'..'z','A'..'Z','_'→ scan_ident()
                        lee chars alfanumericos/_
                        lookup_keyword() → keyword o Ident
```

#### `fn tokenize(&mut self) -> Result<Vec<Token>, LexerError>`

Pipeline principal para ejecucion. Descarta comentarios.

#### `fn tokenize_with_comments(&mut self) -> Result<(Vec<Token>, Vec<Comment>), LexerError>`

Pipeline para el formatter. Recolecta comentarios en paralelo en `self.comments`.
Devuelve `(tokens, comments)`.

---

## src/parser/

### ast.rs

Tipos de datos puros que forman el AST. No contiene logica.

#### `struct Program`

```
Program {
  declarations: Vec<Declaration>
}
```

La raiz del AST. Un programa es una lista de declaraciones top-level.

#### `enum Declaration`

```
Declaration
  ├── Function(FnDecl)     fn name(...) -> T { ... }
  ├── Struct(StructDecl)   struct Name { ... }
  ├── Connect(ConnectDecl) connect file "..." as alias;
  └── Statement(Stmt)      statements a nivel top-level
```

#### `struct ConnectDecl`

```
ConnectDecl {
  connect_type: ConnectType,  File | Db | Api
  file_path:    String,       "users.csv"
  alias:        String,       "users"
  span:         Span,
}
```
Ejemplo: `connect file "users.csv" as users;`

#### `struct FnDecl`

```
FnDecl {
  name:        String,
  params:      Vec<Param>,
  return_type: TypeAnnotation,
  body:        Block,
  span:        Span,
}
```
Ejemplo: `fn distance(p1: Point, p2: Point) -> float { ... }`

#### `struct Param`

```
Param {
  name:     String,
  type_ann: TypeAnnotation,
  span:     Span,
}
```
Un parametro en la firma de una funcion. Tipo siempre requerido.

#### `struct StructDecl`

```
StructDecl {
  name:   String,
  fields: Vec<StructField>,
  span:   Span,
}
```
Ejemplo: `struct User { name: string, age: int }`

#### `struct StructField`

```
StructField {
  name:     String,
  type_ann: TypeAnnotation,
  span:     Span,
}
```

#### `enum TypeAnnotation`

```
TypeAnnotation
  ├── Int               int
  ├── Float             float
  ├── Bool              bool
  ├── StringType        string
  ├── Void              void
  ├── List(Box<T>)      list<T>   ej: list<User>, list<list<string>>
  └── UserDefined(name) nombre de struct   ej: User, Point
```

#### `struct Block`

```
Block {
  statements: Vec<Stmt>,
  span:       Span,
}
```

Un bloque delimitado por `{ }`. Puede contener cualquier numero de statements.

#### `enum Stmt`

```
Stmt
  │
  ├── Let { name, mutable, type_ann: Option<TypeAnnotation>, initializer, span }
  │     let x: int = 42;   (type_ann=Some, mutable=false)
  │     let mut n = 0;     (type_ann=None, mutable=true)
  │
  ├── Assign { target: AssignTarget, value, span }
  │     x = 5;             (AssignTarget::Variable)
  │     p.x = 1.0;         (AssignTarget::FieldAccess)
  │
  ├── If { condition, then_block, else_block: Option<Block>, span }
  │     if cond { ... }
  │     if c1 { } else if c2 { } else { }
  │     ("else if" es azucar: else_block = Some(Block{[Stmt::If{...}]}))
  │
  ├── While { condition, body, span }
  │     while n > 0 { ... }
  │
  ├── For { variable, start, end, body, span }
  │     for i in 1..10 { ... }   (end exclusivo)
  │
  ├── Return { value: Option<Expr>, span }
  │     return 42;   (value=Some)
  │     return;      (value=None → Void)
  │
  ├── Print { args: Vec<Expr>, span }
  │     print("hello", x);
  │
  └── Expression { expr, span }
        fn_call();   (valor descartado)
```

#### `enum Expr` — 13 variantes

```
Expr
  ├── IntLiteral    { value: i64 }
  ├── FloatLiteral  { value: f64 }
  ├── StringLiteral { value: String }
  ├── BoolLiteral   { value: bool }
  ├── Identifier    { name: String }
  │
  ├── BinaryOp { left, op: BinaryOp, right }
  │     BinaryOp: Add Sub Mul Div Mod
  │               Eq Neq Lt Lte Gt Gte
  │               And Or
  │
  ├── UnaryOp { op: UnaryOp, operand }
  │     UnaryOp: Neg(-x)  Not(!x)
  │
  ├── FnCall     { callee: String, args: Vec<Expr> }
  ├── FieldAccess{ object: Box<Expr>, field: String }
  ├── StructInit { name: String, fields: Vec<(String, Expr)> }
  ├── Grouped    { expr: Box<Expr> }
  │
  ├── SqlSelect {
  │     columns:   Vec<String>,           ["*"] o ["name","age"]
  │     table_ref: SqlTableRef,           Alias o Inline
  │     condition: Option<Box<Expr>>,     WHERE
  │     single:    bool,                  SELECT SINGLE
  │   }
  │
  └── SqlInsert {
        table_ref: SqlTableRef,
        values:    Vec<Expr>,
      }
```

#### `enum SqlTableRef`

```
SqlTableRef
  ├── Alias("users")      FROM users        (usa alias de connect)
  └── Inline("data.csv")  FROM file("x.csv") (ruta directa)
```

#### `enum AssignTarget`

```
AssignTarget
  ├── Variable(String)                        x = ...
  └── FieldAccess { object: Box<Expr>, field } p.x = ...
```

---

### parser.rs

Parser recursive descent. Convierte `Vec<Token>` en `Program`.

#### `struct Parser`

```
Parser {
  tokens: Vec<Token>,
  pos:    usize,       posicion actual en el Vec
}
```

#### Helpers basicos

| Funcion | Descripcion |
|---------|-------------|
| `peek()` | Devuelve `&Token` en pos actual sin avanzar |
| `peek_kind()` | Devuelve `&TokenKind` en pos actual |
| `advance()` | Devuelve Token actual y avanza pos |
| `check(kind)` | `true` si peek_kind() == kind (por discriminante) |
| `match_token(kind)` | Si check(), avanza y devuelve `true`; si no, `false` |
| `expect(kind)` | Como match_token pero devuelve `Err` si no coincide |
| `expect_ident()` | Espera `Ident(name)`, devuelve `(String, Span)` |
| `is_at_end()` | `true` si peek_kind() == Eof |

#### Precedencia de expresiones (de menor a mayor)

```
parse_expression()
  └── parse_or()           ||
        └── parse_and()    &&
              └── parse_equality()      ==  !=
                    └── parse_comparison()   <  <=  >  >=
                          └── parse_addition()    +  -
                                └── parse_multiplication()  *  /  %
                                      └── parse_unary()  -x  !x
                                            └── parse_call_or_field()
                                                  └── parse_primary()
```

Cada nivel intenta parsear el nivel inferior y luego busca su operador.
Si lo encuentra, envuelve en `Expr::BinaryOp` y repite (left-associative).

#### Funciones de parseo de declaraciones

| Funcion | Produce | Sintaxis |
|---------|---------|---------|
| `parse()` | `Program` | Bucle hasta Eof llamando `parse_declaration` |
| `parse_declaration()` | `Declaration` | Despacha segun primer token |
| `parse_fn_decl()` | `FnDecl` | `fn name(params) -> type { block }` |
| `parse_struct_decl()` | `StructDecl` | `struct Name { field: type, ... }` |
| `parse_connect()` | `ConnectDecl` | `connect file "path" as alias;` |
| `parse_type()` | `TypeAnnotation` | `int`, `list<User>`, `User`, etc. |
| `parse_block()` | `Block` | `{ stmt* }` |

#### Funciones de parseo de statements

| Funcion | Produce | Sintaxis |
|---------|---------|---------|
| `parse_statement()` | `Stmt` | Despacha segun primer token |
| `parse_let_stmt()` | `Stmt::Let` | `let [mut] name [: type] = expr;` |
| `parse_if_stmt()` | `Stmt::If` | `if expr { } [else if/else { }]*` |
| `parse_while_stmt()` | `Stmt::While` | `while expr { }` |
| `parse_for_stmt()` | `Stmt::For` | `for var in expr..expr { }` |
| `parse_return_stmt()` | `Stmt::Return` | `return [expr];` |
| `parse_print_stmt()` | `Stmt::Print` | `print(args);` |
| `parse_assign_or_expr_stmt()` | `Stmt::Assign` o `Stmt::Expression` | Lee expr, si sigue `=` es asignacion |

#### Desazucarado de `else if`

```
parse_if_stmt():
  consume 'if'
  condition = parse_expression()
  then_block = parse_block()

  si ve 'else':
    si ve 'if' a continuacion:
      inner_if = parse_if_stmt()        ← recursion
      else_block = Some(Block { [inner_if] })   ← DESUGAR
    si no:
      else_block = Some(parse_block())          ← else normal
  si no ve 'else':
    else_block = None
```

El `else if` queda como `else_block = Some(Block{[Stmt::If{...}]})`.
El AST no necesita un nodo `ElseIf` — es una simplificacion deliberada.

#### Funciones de parseo de expresiones

| Funcion | Operador | Ejemplo |
|---------|----------|---------|
| `parse_or()` | `\|\|` | `a \|\| b` |
| `parse_and()` | `&&` | `a && b` |
| `parse_equality()` | `==` `!=` | `a == b` |
| `parse_comparison()` | `<` `<=` `>` `>=` | `n < 10` |
| `parse_addition()` | `+` `-` | `x + 1` |
| `parse_multiplication()` | `*` `/` `%` | `n * 2` |
| `parse_unary()` | `-` `!` | `-x`, `!flag` |
| `parse_call_or_field()` | `.` `(` | `obj.field`, `fn(args)` |
| `parse_primary()` | literales, id, `(`, `#` | `42`, `"hi"`, `#SELECT` |

#### `parse_primary()`

```
peek_kind() ?

  IntLiteral(v)     → Expr::IntLiteral
  FloatLiteral(v)   → Expr::FloatLiteral
  StringLiteral(v)  → Expr::StringLiteral
  BoolLiteral(v)    → Expr::BoolLiteral

  Ident(name)
    si sigue '{':    → parse_struct_init() → Expr::StructInit
    si no:           → Expr::Identifier

  LeftParen         → parse_expression() + expect ')' → Expr::Grouped

  Hash (#)
    si sigue SELECT → parse_sql_select() → Expr::SqlSelect
    si sigue INSERT → parse_sql_insert() → Expr::SqlInsert

  File / Ident("file")
    si sigue '(':    → lee string literal → SqlTableRef::Inline
```

---

## src/semantic/

### type_checker.rs

Analisis semantico pre-ejecucion. Detecta errores estructurales sin ejecutar el codigo.
No realiza inferencia de tipos completa.

#### `struct TypeChecker`

```
TypeChecker {
  functions: HashMap<String, usize>,      nombre → numero de parametros
  structs:   HashMap<String, Vec<String>>,nombre → lista de nombres de campos
  errors:    Vec<SemanticError>,
}
```

#### Flujo general

```
TypeChecker::check(program)
  │
  ├── collect_declarations(program)
  │     Lee todas las FnDecl y StructDecl para poblar
  │     self.functions y self.structs antes de validar
  │     (permite referencias forward)
  │
  └── validate_program(program)
        Para cada Declaration:
          Function → validate_block(body, in_function=true)
          Struct   → (nada, ya recolectado)
          Connect  → (nada)
          Statement→ validate_stmt(stmt, in_function=false)
```

#### `fn collect_declarations()`

```
Para cada funcion:
  ¿Ya existe en seen_fns? → error "duplicate function"
  Inserta nombre → param_count en self.functions

Para cada struct:
  ¿Ya existe en seen_structs? → error "duplicate struct"
  Inserta nombre → [field_names] en self.structs
```

#### `fn validate_stmt(stmt, in_function)`

| Stmt | Validaciones |
|------|-------------|
| `Let` | Si hay type_ann → validate_type(); validate_expr(initializer) |
| `Assign` | validate_expr(value) |
| `If` | validate_expr(cond); validate_block(then); validate_block(else_block) si existe |
| `While` | validate_expr(cond); validate_block(body) |
| `For` | validate_expr(start); validate_expr(end); validate_block(body) |
| `Return` | Si !in_function → error "return outside function"; validate_expr(value) si existe |
| `Print` | validate_expr() para cada arg |
| `Expression` | validate_expr(expr) |

#### `fn validate_expr(expr)`

```
FnCall { callee, args }:
  BUILTINS = [("typeOf", 1)]

  1. ¿callee esta en self.functions?
     → verifica arg count
  2. ¿callee es built-in?
     → verifica arg count especifico del built-in
  3. Ninguno de los dos
     → error "undefined function"

  Luego: validate_expr() para cada arg

StructInit { name, fields }:
  ¿name esta en self.structs?
  → verifica que no falte ningun campo esperado
  → verifica que no haya campos desconocidos

BinaryOp, UnaryOp, FieldAccess, Grouped:
  → validate_expr() recursivo sobre sub-expresiones

SqlSelect { condition }:
  → validate_expr(condition) si existe

SqlInsert { values }:
  → validate_expr() para cada value

Literales e Identifier:
  → nada (se validan en runtime)
```

#### `fn validate_type(type_ann)`

```
UserDefined(name):
  Verifica que name exista en self.structs
  (actualmente no genera error, se deja para runtime)

List(inner):
  → validate_type(inner) recursivo

Primitivos:
  → nada
```

#### Errores generados (prefijo S001)

```
"duplicate function 'name'"
"duplicate struct 'name'"
"function 'f' expects N arguments, got M"
"built-in 'typeOf' expects 1 argument(s), got M"
"undefined function 'name'"
"missing field 'x' in struct 'S'"
"unknown field 'x' in struct 'S'"
"undefined struct 'S'"
"return statement outside of function"
```

---

## src/codegen/

### interpreter.rs

El interprete tree-walking. Recorre el AST y ejecuta cada nodo.

#### `fn native_type_of(args: Vec<Value>) -> Result<Value, RuntimeError>`

Funcion built-in registrada en `native_functions`.
Devuelve el tipo del argumento como `Value::Str`.

```
typeOf(42)     → Value::Str("int")
typeOf(3.14)   → Value::Str("float")
typeOf("hi")   → Value::Str("string")
typeOf(true)   → Value::Str("bool")
typeOf(p1)     → Value::Str("Point")
typeOf(lista)  → Value::Str("list")
```

---

#### `enum Value`

Valores en tiempo de ejecucion:

```
Value
  ├── Int(i64)
  ├── Float(f64)
  ├── Bool(bool)
  ├── Str(String)
  ├── StructInstance { type_name: String, fields: HashMap<String, Value> }
  ├── List(Vec<Value>)
  └── Void
```

#### `impl Value`

| Metodo | Descripcion |
|--------|-------------|
| `type_name()` | Devuelve `&str` con el nombre del tipo: "int", "float", "bool", "string", nombre_struct, "list", "void" |
| `to_display_string()` | Convierte a String para `print()`. Float se muestra con `.1` si es entero (ej: `1.0`). Struct: `Nombre { campo: val, ... }`. List: `[v1, v2, ...]` |

---

#### `struct Variable`

```
Variable {
  value:   Value,
  mutable: bool,   true si fue declarada con let mut
}
```

#### `struct Environment`

Pila de scopes (HashMaps) para manejar el alcance de variables.

```
Environment {
  scopes: Vec<HashMap<String, Variable>>
}
```

```
Ejemplo de estado del scope stack durante ejecucion:

  [0] scope_global:   { x: Int(42), pi: Float(3.14) }
  [1] scope_main():   { greeting: Str("Hello") }
  [2] scope_if:       { resultado: Bool(true) }
      ↑ scope mas interno (se busca primero en get/set)
```

| Metodo | Descripcion |
|--------|-------------|
| `new()` | Crea con un scope global inicial |
| `push_scope()` | Entra en un bloque/funcion (push HashMap vacio) |
| `pop_scope()` | Sale del bloque/funcion actual (destruye sus variables) |
| `define(name, value, mutable)` | Declara variable en el scope mas interno |
| `get(name, span)` | Busca de interno a externo; error si no existe |
| `set(name, value, span)` | Busca de interno a externo; error si no existe o no es mutable |

---

#### `enum StmtResult`

Mecanismo de propagacion de `return`:

```
StmtResult
  ├── Normal          statement ejecutado sin return
  └── Return(Value)   un return fue encontrado; se propaga hacia arriba
```

Cuando `execute_block_inner` encuentra `StmtResult::Return(v)`, lo devuelve
inmediatamente sin ejecutar el resto de statements. `call_function` lo extrae
para devolver el valor al llamador.

---

#### `struct Interpreter`

```
Interpreter {
  environment:      Environment,
  functions:        HashMap<String, FnDecl>,
  structs:          HashMap<String, StructDecl>,
  alias:            HashMap<String, String>,   alias → file_path
  base_dir:         PathBuf,
  native_functions: HashMap<String, fn(Vec<Value>) → Result<Value, RuntimeError>>,
}
```

#### `fn new(base_dir: PathBuf) -> Interpreter`

Construye el interpreter. Registra `typeOf` en `native_functions`.

#### `fn run(program) -> Result<(), RuntimeError>`

```
run(program)
  │
  ├── Fase 1: Registro
  │     Para cada Declaration:
  │       Function → self.functions.insert(name, FnDecl)
  │       Struct   → self.structs.insert(name, StructDecl)
  │       Connect  → self.alias.insert(alias, file_path)
  │       Statement→ (ignorado en esta fase)
  │
  ├── Fase 2: Ejecucion top-level
  │     Para cada Declaration::Statement:
  │       execute_stmt(stmt)
  │
  └── Fase 3: Llamar main si existe
        si self.functions.contains("main"):
          call_function("main", [], dummy_span)
```

#### `fn execute_block(block) -> Result<StmtResult, RuntimeError>`

Wrapper: push_scope → execute_block_inner → pop_scope.

#### `fn execute_block_inner(block)`

Itera statements. Si alguno devuelve `StmtResult::Return`, para y lo propaga.

---

#### `fn execute_stmt(stmt)` — Statement por statement

```
Stmt::Let:
  ┌─ initializer es SqlSelect?
  │   Si: determine use_string_mode segun type_ann
  │       ejecuta execute_sql_select(use_string_mode)
  │   No: evaluate_expr(initializer)
  └─ Si type_ann es Some: check_type_compat(value, type_ann)
     define(name, value, mutable) en environment

Stmt::Assign:
  val = evaluate_expr(value)
  ┌─ target es Variable?
  │   environment.set(name, val)
  └─ target es FieldAccess?
      get struct del environment
      modifica campo en el HashMap
      set struct de vuelta (es inmutable struct reference → clone y replace)

Stmt::If:
  cond = evaluate_expr(condition)  → debe ser Bool
  ┌─ cond es true?  → execute_block(then_block)
  └─ cond es false?
      ┌─ else_block existe? → execute_block(else_block)
      │   (si else_block es un "else if" desazucarado, el
      │    Stmt::If interior se ejecuta recursivamente)
      └─ no existe?        → Normal

Stmt::While:
  loop:
    cond = evaluate_expr(condition)  → debe ser Bool
    si false: break
    result = execute_block(body)
    si result es Return: propagar

Stmt::For:
  start_i = evaluate_expr(start) → debe ser Int
  end_i   = evaluate_expr(end)   → debe ser Int
  para i en start_i..end_i:
    push_scope
    define(variable, Int(i), false)
    execute_block_inner(body)
    pop_scope
    si return: propagar

Stmt::Return:
  val = evaluate_expr(value) o Void
  → StmtResult::Return(val)

Stmt::Print:
  Evalua cada arg → to_display_string()
  println!(args.join(" "))

Stmt::Expression:
  evaluate_expr(expr)  → valor descartado
```

---

#### `fn evaluate_expr(expr) -> Result<Value, RuntimeError>`

```
Expr::IntLiteral    → Value::Int(v)
Expr::FloatLiteral  → Value::Float(v)
Expr::StringLiteral → Value::Str(v)
Expr::BoolLiteral   → Value::Bool(v)

Expr::Identifier    → environment.get(name)

Expr::Grouped       → evaluate_expr(inner)

Expr::UnaryOp (Neg) → evalua operand → negar (solo Int/Float)
Expr::UnaryOp (Not) → evalua operand → negar (solo Bool)

Expr::BinaryOp (And):          Expr::BinaryOp (Or):
  left_val = evaluate_expr(left)   left_val = evaluate_expr(left)
  si left es false → false         si left es true → true
  right_val = evaluate_expr(right) right_val = evaluate_expr(right)
  devuelve right_val (Bool)        devuelve right_val (Bool)
  (short-circuit: right no         (short-circuit: right no
   se evalua si no es necesario)    se evalua si no es necesario)

Expr::BinaryOp (resto) →
  left_val = evaluate_expr(left)
  right_val = evaluate_expr(right)
  eval_binary_op(op, left, right)

Expr::FnCall →
  arg_values = [evaluate_expr(a) for a in args]
  call_function(callee, arg_values)

Expr::FieldAccess →
  obj = evaluate_expr(object)
  si obj es StructInstance: fields.get(field)
  si no: error "cannot access field on TYPE"

Expr::StructInit →
  1. busca StructDecl en self.structs
  2. evalua cada (field_name, expr)
  3. verifica que esten todos los campos obligatorios
  4. devuelve Value::StructInstance { type_name, fields }

Expr::SqlSelect → execute_sql_select(... use_string_mode=false)

Expr::SqlInsert → execute_sql_insert(...)
```

---

#### `fn call_function(name, args, span)`

```
call_function(name, args, span)
  │
  ├── 1. ¿name esta en native_functions?
  │       → native_fn(args)   (ej: typeOf)
  │
  ├── 2. ¿name esta en self.functions?
  │       verifica arg count
  │       push_scope
  │       define param_name → arg_value para cada parametro
  │       execute_block_inner(func.body)
  │       pop_scope
  │       Normal  → Void
  │       Return(v) → v
  │
  └── 3. Ninguno → error "undefined function"
```

---

#### SQL execution — funciones privadas

##### `fn resolve_file_path(table_ref, span) -> (PathBuf, String)`

```
SqlTableRef::Alias("users")
  → busca en self.alias["users"] → "users.csv"
  → base_dir.join("users.csv")

SqlTableRef::Inline("data.csv")
  → base_dir.join("data.csv")
  → nombre = file_stem ("data")
```

##### `fn cell_to_value(s: &str) -> Value`

```
"42"    → Int(42)
"3.14"  → Float(3.14)
"true"  → Bool(true)
"false" → Bool(false)
_       → Str(s)
```

##### `fn row_to_struct(struct_name, headers, row, columns) -> Value`

Convierte una fila del CSV en un `Value::StructInstance`.
Si `columns == ["*"]` usa todas las columnas; si no, solo las indicadas.

##### `fn row_to_string_list(headers, row, columns) -> Value`

Convierte una fila en `Value::List(Vec<Value::Str>)`.
Usado cuando el tipo es `list<list<string>>` (string mode).

##### `fn is_primitive_list_type(type_ann) -> bool`

```
list<list<string>>  → true   (string mode)
list<string>        → true
list<int>           → true
list<float>         → true
list<bool>          → true
list<User>          → false  (struct mode)
```

##### `fn find_matching_struct(headers, columns) -> Option<String>`

Busca en `self.structs` cual struct coincide con las columnas del resultado SQL.
Primero intenta coincidencia exacta (todos los campos); si no, coincidencia parcial.

##### `fn execute_sql_select(columns, table_ref, condition, single, span, use_string_mode)`

```
1. resolve_file_path(table_ref) → PathBuf
2. DataTable::from_file(path)
3. Valida que las columnas pedidas existan en el archivo
4. Encuentra struct coincidente (si struct mode)
5. Para cada fila:
     a. push_scope temporal
     b. define cada columna como variable (para WHERE)
     c. evaluate_expr(condition) → Bool
     d. pop_scope
     e. Si coincide:
          use_string_mode? → row_to_string_list → Value::List(Str)
          no?              → row_to_struct       → Value::StructInstance
        Si single=true: devuelve inmediatamente el primer match
6. Si single=true y ningun match → error
7. Devuelve Value::List(resultados)
```

##### `fn execute_sql_insert(table_ref, value_exprs, span)`

```
1. resolve_file_path(table_ref)
2. DataTable::from_file(path)
3. Evalua cada value_expr → convierte a String
4. table.append_row(row_values)
5. table.save_to_file(path)
6. Devuelve Value::Bool(true)
```

---

#### `fn eval_binary_op(op, left, right, span)`

| Operador | Tipos soportados | Comportamiento especial |
|----------|-----------------|------------------------|
| `Add(+)` | Int+Int, Float+Float, Int+Float, Float+Int, Str+Str | Concatenacion con strings |
| `Sub(-)` | Int, Float, mixto | `numeric_op` |
| `Mul(*)` | Int, Float, mixto | `numeric_op` |
| `Div(/)` | Int, Float, mixto | Error si divisor es 0 |
| `Mod(%)` | Int | Error si divisor es 0 |
| `Eq(==)` | Int, Float, Bool, Str | Igualdad estricta |
| `Neq(!=)` | idem | |
| `Lt(<)` `Lte(<=)` `Gt(>)` `Gte(>=)` | Int, Float, mixto | Orden numerico |

#### `fn check_type_compat(value, type_ann, span)`

Solo se llama cuando `let` tiene tipo explicito. Tabla de compatibilidad:

```
Int    ↔ Int        ✓
Float  ↔ Float      ✓
Int    → Float      ✓ (promocion implicita)
Bool   ↔ Bool       ✓
Str    ↔ StringType ✓
Void   ↔ Void       ✓
StructInstance(T) ↔ UserDefined(T) ✓ (mismo nombre)
List   ↔ List(any)  ✓
List   ↔ UserDefined ✓ (listas SQL con tipo struct)
_      → error "type mismatch"
```

#### `fn type_ann_name(t) -> String`

Convierte `TypeAnnotation` a string legible para mensajes de error.
`list<list<string>>` → `"list<list<string>>"`.

---

## src/formatter/

### formatter.rs

Pretty-printer que regenera codigo Laz formateado a partir del AST,
preservando comentarios en su posicion relativa.

#### `struct Formatter`

```
Formatter {
  comments:         Vec<Comment>,   lista ordenada por linea
  output:           String,         buffer de salida
  indent_level:     usize,          nivel de indentacion actual
  next_comment_idx: usize,          puntero al siguiente comentario a emitir
}
```

**Invariante**: `comments` esta ordenado por linea (sort en `new()`).
`next_comment_idx` avanza monotonicamente — cada comentario se emite exactamente una vez.

#### `fn new(comments: Vec<Comment>) -> Formatter`

Ordena comentarios por linea, inicializa output vacio e indent=0.

#### `fn format(self, program: &Program) -> String`

```
format_program(program)
emit_remaining_comments()   ← comentarios al final del archivo
output.trim_end() + "\n"    ← exactamente un newline al final
```

---

#### Funciones de comentarios

##### `fn emit_leading_comments(before_line: usize)`

Emite todos los comentarios NO-inline cuya linea < `before_line`.
Se llama antes de cada declaracion para imprimir los comentarios que la preceden.

```
// este es un comentario leading
fn foo() { ... }   ← before_line = linea de 'fn'
```

##### `fn emit_inline_comment(on_line: usize)`

Si el siguiente comentario es inline y esta en `on_line`, lo agrega
al final de la linea actual (quita el `\n` final, agrega ` // text\n`).

```
let x = 42; // comentario inline
```

##### `fn emit_remaining_comments()`

Emite comentarios que quedaron despues de la ultima declaracion.

---

#### Funciones de formateo de declaraciones

##### `fn format_program(program)`

```
Para cada Declaration (con indice i):
  1. Calcula si necesita blank line entre declaraciones
     (entre tipos distintos, o entre funciones)
  2. emit_leading_comments(decl_line)
  3. format_declaration(decl, next_decl_line)
```

##### `fn format_declaration(decl, next_decl_line)`

Despacha a:
- `format_fn_decl()` para funciones
- `format_struct_decl()` para structs
- `format_connect_decl()` para connect
- `format_statement()` para statements top-level

##### `fn format_fn_decl(f)`

```
fn name(p1: T1, p2: T2) -> RetType {
    [body statements]
}
```

##### `fn format_struct_decl(s)`

```
struct Name {
    field1: Type1,
    field2: Type2,
}
```

##### `fn format_connect_decl(c)`

```
connect file "path" as alias;
connect db "path" as alias;
connect api "path" as alias;
```

---

#### `fn format_statement(stmt)`

| Stmt | Output |
|------|--------|
| `Let` con tipo | `let [mut] name: type = expr;` |
| `Let` sin tipo | `let [mut] name = expr;` |
| `Assign` Variable | `name = expr;` |
| `Assign` FieldAccess | `obj.field = expr;` |
| `If` | `if cond { } [else if/else { }]` via `format_else_block` |
| `While` | `while cond { }` |
| `For` | `for var in start..end { }` |
| `Return` con valor | `return expr;` |
| `Return` sin valor | `return;` |
| `Print` | `print(arg1, arg2, ...);` |
| `Expression` | `expr;` |

---

#### `fn format_else_block(else_block: Option<&Block>)`

Detecta el patron `else if` desazucarado y lo reconstruye:

```
format_else_block(else_block):
  │
  ├── None → "}\n"   (cierra el if sin else)
  │
  └── Some(block) →
        ¿block tiene exactamente 1 statement Y es Stmt::If?
        │
        ├── Si: ELSE IF detectado
        │     emite "} else if cond {\n"
        │     indenta y formatea then_block
        │     format_else_block(inner_else_block)   ← recursion
        │
        └── No: else normal
              emite "} else {\n"
              indenta y formatea block
              emite "}\n"
```

---

#### `fn format_expr(&self, expr: &Expr) -> String`

Convierte una expresion a String. Sin efectos secundarios (toma `&self`).

| Expr | Output |
|------|--------|
| IntLiteral | `"42"` |
| FloatLiteral | `"3.14"` (o `"1.0"` si tiene parte decimal) |
| StringLiteral | `"\"hello\""` |
| BoolLiteral | `"true"` / `"false"` |
| Identifier | `"name"` |
| BinaryOp | `"left op right"` con espacios |
| UnaryOp Neg | `"-expr"` |
| UnaryOp Not | `"!expr"` |
| FnCall | `"name(arg1, arg2)"` |
| FieldAccess | `"obj.field"` |
| StructInit | `"Name { f1: e1, f2: e2 }"` |
| Grouped | `"(expr)"` |
| SqlSelect | `"#SELECT cols FROM ref [WHERE cond]"` o `"#SELECT SINGLE ..."` |
| SqlInsert | `"#INSERT INTO ref VALUES (v1, v2)"` |

---

## src/utils/

### error.rs

Tipos de error y formateo de mensajes con contexto.

#### `enum NovaError`

```
NovaError
  ├── Lexer(LexerError)       → prefijo L001
  ├── Parse(ParseError)       → prefijo P001
  ├── Semantic(SemanticError) → prefijo S001
  └── Runtime(RuntimeError)   → prefijo R001
```

Implementa `From<T>` para cada variante (`.into()` automatico en `main.rs`).

#### Tipos de error especificos

| Tipo | Campos |
|------|--------|
| `LexerError` | `message: String`, `span: Span` |
| `ParseError` | `message: String`, `expected: String`, `found: String`, `span: Span` |
| `SemanticError` | `message: String`, `span: Span` |
| `RuntimeError` | `message: String`, `span: Span` |

#### `fn format_error(error: &NovaError, source: &str, filename: &str) -> String`

Produce un mensaje de error con contexto visual:

```
error[R001]: undefined variable 'x'
  --> examples/hello.lz:5:10
   |
 5 | print(x);
   |       ^
```

```
Proceso:
  1. Extrae (prefix, message, span) segun variante de NovaError
  2. Obtiene la linea del source con span.line
  3. Calcula el numero de '^' = span.end - span.start (minimo 1)
  4. Calcula el padding de columna para posicionar el '^'
  5. Formatea con numero de linea y caret
```

---

### csv.rs

Carga, parseo y guardado de archivos de datos.

#### `enum DataFormat`

```
DataFormat
  ├── Csv    → archivos .csv
  └── Json   → archivos .json (preparado, no implementado)
```

##### `fn from_extension(path: &Path) -> Result<DataFormat, String>`

```
".csv"  → Ok(DataFormat::Csv)
".json" → Ok(DataFormat::Json)
otra    → Err("unsupported file format: .ext")
sin ext → Err("file has no extension")
```

---

#### `struct DataTable`

```
DataTable {
  headers: Vec<String>,        nombres de columnas (primera linea del CSV)
  rows:    Vec<Vec<String>>,   datos: cada fila es un Vec de celdas
  format:  DataFormat,         formato detectado al cargar
}
```

---

#### Metodos de `DataTable`

##### `fn from_file(path: &Path) -> Result<DataTable, String>`

```
1. DataFormat::from_extension(path)
2. fs::read_to_string(path)
3. Segun formato:
     Csv  → parse_csv(content)
     Json → Err (no implementado)
```

##### `fn parse_csv(content: &str) -> Result<DataTable, String>`

```
1. Lee primera linea → headers via parse_csv_line()
2. Para cada linea restante:
     parse_csv_line(line) → fields
     Verifica fields.len() == headers.len()
     Empuja a rows
3. Ok(DataTable { headers, rows, format: Csv })
```

##### `fn column_index(name: &str) -> Option<usize>`

Devuelve el indice de la columna por nombre. Usado para validar columnas en SQL.

##### `fn row_as_map(row_idx: usize) -> HashMap<String, String>`

Devuelve una fila como `HashMap<columna, valor>`.

##### `fn append_row(values: &[String]) -> Result<(), String>`

Agrega una nueva fila. Verifica que `values.len() == headers.len()`.
Usado por `#INSERT INTO`.

##### `fn save_to_file(path: &Path) -> Result<(), String>`

Guarda el DataTable en el archivo. Despacha a `save_as_csv`.

##### `fn save_as_csv(path: &Path)` (privada)

Serializa headers y rows como CSV, con escape de campos que contengan
comas, comillas o saltos de linea.

---

#### Funciones privadas de parseo CSV

##### `fn parse_csv_line(line: &str) -> Vec<String>`

Parser de una linea CSV que maneja comillas:

```
Estado: in_quotes = false/true

  char por char:
    in_quotes=false:
      ','  → fin de campo, push a fields
      '"'  → in_quotes = true
      otro → append a current

    in_quotes=true:
      '"' seguido de '"' → append '"' (escaped quote "")
      '"' solo           → in_quotes = false
      otro               → append a current

  Al final: push ultimo campo
```

##### `fn escape_csv_field(field: &str) -> String`

Si el campo contiene `,`, `"` o `\n`, lo envuelve en comillas y escapa
las comillas internas con `""`. Si no, lo devuelve sin cambios.

---

## Resumen de funciones por archivo

| Archivo | Funciones publicas | Funciones privadas |
|---------|-------------------|-------------------|
| `main.rs` | `main` | `run_program`, `format_file` |
| `cli.rs` | `parse_args` | — |
| `lexer/mod.rs` | `tokenize`, `tokenize_with_comments` | `scan_token`, `scan_string`, `scan_number`, `scan_ident`, `skip_whitespace`, `skip_line_comment`, `collect_comment`, `peek`, `peek_next`, `advance`, `is_at_end`, `make_span` |
| `lexer/token.rs` | `lookup_keyword` | — |
| `parser/parser.rs` | `new`, `parse` | `parse_declaration`, `parse_fn_decl`, `parse_struct_decl`, `parse_connect`, `parse_type`, `parse_block`, `parse_statement`, `parse_let_stmt`, `parse_if_stmt`, `parse_while_stmt`, `parse_for_stmt`, `parse_return_stmt`, `parse_print_stmt`, `parse_assign_or_expr_stmt`, `parse_expression`, `parse_or`, `parse_and`, `parse_equality`, `parse_comparison`, `parse_addition`, `parse_multiplication`, `parse_unary`, `parse_call_or_field`, `parse_primary`, `parse_sql_select`, `parse_sql_insert`, `parse_struct_init`, `peek`, `peek_kind`, `advance`, `check`, `match_token`, `expect`, `expect_ident`, `is_at_end` |
| `semantic/type_checker.rs` | `check` | `collect_declarations`, `validate_program`, `validate_block`, `validate_stmt`, `validate_expr`, `validate_type` |
| `codegen/interpreter.rs` | `new`, `run` | `execute_block`, `execute_block_inner`, `execute_stmt`, `evaluate_expr`, `eval_binary_op`, `numeric_op`, `comparison_op`, `call_function`, `check_type_compat`, `type_ann_name`, `resolve_file_path`, `cell_to_value`, `row_to_struct`, `row_to_string_list`, `is_primitive_list_type`, `find_matching_struct`, `execute_sql_select`, `execute_sql_insert` |
| `formatter/formatter.rs` | `new`, `format` | `format_program`, `format_declaration`, `format_fn_decl`, `format_struct_decl`, `format_connect_decl`, `format_statement`, `format_else_block`, `format_block_inner`, `format_expr`, `format_type`, `format_sql_table_ref`, `indent`, `emit_leading_comments`, `emit_inline_comment`, `emit_remaining_comments`, `decl_span`, `stmt_span` |
| `utils/error.rs` | `format_error` | — |
| `utils/csv.rs` | `from_extension`, `from_file`, `parse_csv`, `column_index`, `row_as_map`, `append_row`, `save_to_file` | `save_as_csv`, `parse_csv_line`, `escape_csv_field`, `native_type_of` |
