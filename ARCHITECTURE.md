# Laz - Arquitectura del Interprete

## Pipeline General

```
 archivo.lz (codigo fuente)
       |
       v
 +===========+
 |   LEXER   |  src/lexer/
 +===========+
       |
       | Vec<Token> + Vec<Comment>
       v
 +===========+
 |  PARSER   |  src/parser/
 +===========+
       |
       | Program (AST)
       v
 +=============+
 | TYPE CHECKER|  src/semantic/
 +=============+
       |
       | Program (validado)
       v
 +=============+
 | INTERPRETER |  src/codegen/interpreter.rs
 +=============+
       |
       v
    Output (stdout)
```

Pipeline alternativo para formateo:

```
 archivo.lz
       |
       v
 +===========+          +===========+
 |   LEXER   |--------->|  PARSER   |
 +===========+          +===========+
       |                      |
  Vec<Comment>            Program (AST)
       |                      |
       +----------+-----------+
                  |
                  v
           +=============+
           |  FORMATTER  |  src/formatter/
           +=============+
                  |
                  v
          Codigo formateado
```

## Fase 1: Lexer

**Archivos:** `src/lexer/mod.rs`, `src/lexer/token.rs`

Convierte el codigo fuente (string) en una secuencia de tokens.

### Entrada / Salida

```
"let x: int = 42;"  -->  [Let, Ident("x"), Colon, IntType, Equal, IntLiteral(42), Semicolon, Eof]
```

### Token

```
Token {
    kind: TokenKind,     // tipo del token
    span: Span {         // posicion en el codigo fuente
        line: usize,
        column: usize,
        start: usize,    // offset absoluto inicio
        end: usize,      // offset absoluto fin
    }
}
```

### Categorias de TokenKind

```
TokenKind
 |
 +-- Literales:     IntLiteral(i64), FloatLiteral(f64), StringLiteral(String), BoolLiteral(bool)
 +-- Identificador: Ident(String)
 +-- Keywords:      Fn, Let, Mut, If, Else, While, For, In, Return, Print, Struct, Connect, File, Db, Api, As
 +-- Tipos:         IntType, FloatType, BoolType, StringType, VoidType, ListType
 +-- Aritmeticos:   Plus, Minus, Star, Slash, Percent
 +-- Comparacion:   EqualEqual, BangEqual, Less, LessEqual, Greater, GreaterEqual
 +-- Logicos:       And, Or, Bang
 +-- Asignacion:    Equal
 +-- Puntuacion:    LeftParen, RightParen, LeftBrace, RightBrace, Comma, Dot, Semicolon, Colon, Arrow, DotDot
 +-- SQL:           Hash, Select, Single, From, Where, Insert, Into, Values
 +-- Especial:      Eof
```

### Comentarios

Los comentarios `//` se recolectan en un canal separado (`Vec<Comment>`) y no generan tokens. Se preservan para el formatter.

```
Comment {
    text: String,      // contenido sin el //
    line: usize,
    column: usize,
    is_inline: bool,   // true si esta en la misma linea que un token
}
```

### lookup_keyword

La funcion `lookup_keyword(ident)` mapea strings a keywords. Si no encuentra match, el string se trata como `Ident(String)`.

## Fase 2: Parser

**Archivos:** `src/parser/parser.rs`, `src/parser/ast.rs`

Parser recursive descent que convierte tokens en un AST (Abstract Syntax Tree).

### Estructura del AST

```
Program
  |
  +-- declarations: Vec<Declaration>
        |
        +-- Function(FnDecl)    --> fn name(params) -> type { body }
        +-- Struct(StructDecl)  --> struct Name { fields }
        +-- Connect(ConnectDecl)--> connect file "path" as alias;
        +-- Statement(Stmt)     --> top-level statements
```

### Declaraciones

```
FnDecl                          StructDecl
 +-- name: String                +-- name: String
 +-- params: Vec<Param>          +-- fields: Vec<StructField>
 |    +-- name: String           |    +-- name: String
 |    +-- type_ann: TypeAnnotation    +-- type_ann: TypeAnnotation
 +-- return_type: TypeAnnotation +-- span
 +-- body: Block
 +-- span

ConnectDecl
 +-- connect_type: ConnectType   // File | Db | Api
 +-- file_path: String
 +-- alias: String
 +-- span
```

### Statements (Stmt)

```
Stmt
 +-- Let       { name, mutable, type_ann: Option<TypeAnnotation>, initializer: Expr, span }
 +-- Assign    { target: AssignTarget, value: Expr, span }
 +-- If        { condition: Expr, then_block: Block, else_branch: Option<ElseBranch>, span }
 +-- While     { condition: Expr, body: Block, span }
 +-- For       { variable: String, start: Expr, end: Expr, body: Block, span }
 +-- Return    { value: Option<Expr>, span }
 +-- Print     { args: Vec<Expr>, span }
 +-- Expression{ expr: Expr, span }
```

### Expresiones (Expr) — 14 variantes

```
Expr
 +-- IntLiteral    { value: i64, span }
 +-- FloatLiteral  { value: f64, span }
 +-- StringLiteral { value: String, span }
 +-- BoolLiteral   { value: bool, span }
 +-- Identifier    { name: String, span }
 +-- BinaryOp      { left, op: BinaryOp, right, span }
 +-- UnaryOp       { op: UnaryOp, operand, span }
 +-- FnCall        { callee: String, args: Vec<Expr>, span }
 +-- FieldAccess   { object, field: String, span }
 +-- StructInit    { name: String, fields: Vec<(String, Expr)>, span }
 +-- Grouped       { expr, span }
 +-- SqlSelect     { columns, table_ref: SqlTableRef, condition, single: bool, span }
 +-- SqlInsert     { table_ref: SqlTableRef, values: Vec<Expr>, span }
```

### TypeAnnotation

```
TypeAnnotation
 +-- Int
 +-- Float
 +-- Bool
 +-- StringType
 +-- Void
 +-- List(Box<TypeAnnotation>)    // list<T>
 +-- UserDefined(String)          // nombre de struct
```

### Parsing de let (tipo opcional)

```
let x: int = 5;     -->  type_ann = Some(Int)
let x = 5;          -->  type_ann = None
```

Si hay `:` despues del nombre, se parsea el tipo. Si no, se salta directo al `=`.

## Fase 3: Analisis Semantico

**Archivo:** `src/semantic/type_checker.rs`

Validacion pre-ejecucion del AST. NO realiza inferencia de tipos completa, pero detecta errores estructurales.

### Verificaciones

```
+----------------------------------+---------------------------------------------+
| Verificacion                     | Error                                       |
+----------------------------------+---------------------------------------------+
| Funciones duplicadas             | "duplicate function 'name'"                 |
| Structs duplicados               | "duplicate struct 'name'"                   |
| Funcion no definida              | "undefined function 'name'"                 |
| Numero de argumentos incorrecto  | "function 'f' expects N arguments, got M"   |
| Campo de struct desconocido      | "unknown field 'x' in struct 'S'"           |
| Campo de struct faltante         | "missing field 'x' in struct 'S'"           |
| Return fuera de funcion          | "return statement outside of function"       |
+----------------------------------+---------------------------------------------+
```

### Funciones built-in

El type checker reconoce funciones nativas (`typeOf`) y las valida sin marcarlas como indefinidas:

```
BUILTINS: [("typeOf", 1)]   // nombre, numero de argumentos esperados
```

## Fase 4: Interprete

**Archivo:** `src/codegen/interpreter.rs`

Interprete tree-walking que recorre el AST y ejecuta las operaciones.

### Valores en Runtime (Value)

```
Value
 +-- Int(i64)
 +-- Float(f64)
 +-- Bool(bool)
 +-- Str(String)
 +-- StructInstance { type_name: String, fields: HashMap<String, Value> }
 +-- List(Vec<Value>)
 +-- Void
```

Cada valor tiene un metodo `type_name()` que devuelve su tipo como string:

```
Int(42)                    --> "int"
Float(3.14)                --> "float"
Bool(true)                 --> "bool"
Str("hello")               --> "string"
StructInstance { "User" }   --> "User"
List([...])                 --> "list"
Void                        --> "void"
```

### Environment (Scope Stack)

```
Environment {
    scopes: Vec<HashMap<String, Variable>>
}

Variable {
    value: Value,
    mutable: bool,
}
```

Funcionamiento:

```
Scope global:  { x: 42, pi: 3.14 }
     |
     +-- Scope funcion:  { n: 5, result: 120 }
           |
           +-- Scope bloque:  { i: 3 }
```

- `push_scope()` al entrar en un bloque/funcion
- `pop_scope()` al salir
- `get()` busca de scope mas interno al mas externo
- `set()` busca la variable y verifica mutabilidad
- `define()` crea variable en el scope actual

### Estado del Interprete

```
Interpreter {
    environment: Environment,                // variables
    functions: HashMap<String, FnDecl>,      // funciones del usuario
    structs: HashMap<String, StructDecl>,    // definiciones de structs
    alias: HashMap<String, String>,          // alias -> ruta de archivo
    base_dir: PathBuf,                       // directorio base del .lz
    native_functions: HashMap<String, fn>,   // funciones built-in (typeOf)
}
```

### Flujo de Ejecucion

```
run(program)
  |
  +-- 1. Registrar funciones, structs y connects
  |
  +-- 2. Ejecutar statements top-level
  |
  +-- 3. Llamar main() si existe
        |
        +-- execute_stmt() para cada statement
              |
              +-- evaluate_expr() para cada expresion
                    |
                    +-- call_function() para llamadas
                          |
                          +-- Busca en native_functions primero
                          +-- Luego en functions del usuario
                          +-- push_scope, ejecutar body, pop_scope
```

### Funciones Nativas

```
call_function(name, args, span)
  |
  +-- native_functions.get(name)?  -->  Si: ejecutar directamente
  |
  +-- functions.get(name)?         -->  Si: push scope, bind params, execute body
  |
  +-- Error: "undefined function"
```

Nativas registradas: `typeOf(value) -> string`

### Ejecucion SQL

```
SQL Query (#SELECT ...)
      |
      v
  resolve_file_path()
      |
      +-- SqlTableRef::Alias("users") --> busca en self.alias --> "users.csv"
      +-- SqlTableRef::Inline("data.csv") --> usa directamente
      |
      v
  DataTable::from_file(path)
      |
      +-- Detecta formato por extension (.csv, .json)
      +-- Parsea el archivo a DataTable { headers, rows, format }
      |
      v
  Filtrar filas (WHERE)
      |
      +-- Crea un scope temporal
      +-- Define las columnas como variables
      +-- Evalua la condicion para cada fila
      |
      v
  Convertir a Value
      |
      +-- Modo struct:  fila --> Value::StructInstance (match con StructDecl)
      +-- Modo string:  fila --> Value::List(Vec<Value::Str>)
      +-- SINGLE:       devuelve primer resultado (no lista)
      |
      v
  Result<Value, RuntimeError>
```

### Conversion de celdas CSV (cell_to_value)

```
String del CSV --> intentar parsear como:
  1. i64        -->  Value::Int
  2. f64        -->  Value::Float
  3. true/false -->  Value::Bool
  4. fallback   -->  Value::Str
```

### Verificacion de tipos en runtime (check_type_compat)

Se ejecuta solo cuando `let` tiene tipo explicito:

```
let x: int = 42;        // check_type_compat(Int(42), Int) --> OK
let x: string = 42;     // check_type_compat(Int(42), String) --> Error
let x = 42;             // Sin verificacion (type_ann = None)
```

## Fase 5: Formatter

**Archivo:** `src/formatter/formatter.rs`

Pretty-printer que regenera codigo formateado a partir del AST, preservando comentarios.

### Funcionamiento

```
AST + Vec<Comment>
      |
      v
  Recorrer cada Declaration
      |
      +-- Antes de cada nodo: emitir comentarios previos (por numero de linea)
      +-- Formatear el nodo con indentacion correcta
      +-- Despues de cada linea: emitir comentario inline si existe
      |
      v
  String formateado
```

### Reglas de formato:
- Indentacion: 4 espacios
- Un newline entre declaraciones top-level
- Preservacion de comentarios `//` en su posicion relativa
- `let` sin tipo: `let x = expr;`
- `let` con tipo: `let x: type = expr;`

## CLI

**Archivo:** `src/cli.rs`

```
Command
 +-- Run(RunConfig)         // laz programa.lz
 |    +-- filename: String
 |    +-- source: String
 |
 +-- Format(FormatConfig)   // laz fmt programa.lz [--write]
      +-- filename: String
      +-- source: String
      +-- write_in_place: bool
```

## Errores

**Archivo:** `src/utils/error.rs`

```
NovaError
 +-- Lexer(LexerError)      // Prefijo: L001
 +-- Parse(ParseError)      // Prefijo: P001
 +-- Semantic(SemanticError) // Prefijo: S001
 +-- Runtime(RuntimeError)   // Prefijo: R001
```

Formato de salida:

```
error[R001]: undefined variable 'x'
 --> examples/hello.lz:5:10
  |
5 | print(x);
  |       ^
```

## DataTable

**Archivo:** `src/utils/csv.rs`

```
DataFormat
 +-- Csv       // .csv
 +-- Json      // .json (preparado, no implementado)

DataTable {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    format: DataFormat,
}
```

Metodos:
- `from_file(path)` — carga archivo, detecta formato por extension
- `parse_csv(content)` — parsea CSV
- `save_to_file(path)` — guarda en el formato original
- `column_index(name)` — indice de una columna por nombre
- `row_as_map(row_idx)` — fila como HashMap
- `append_row(values)` — agrega fila (para INSERT)

## Mapa de Archivos

```
laz/
 +-- src/
 |    +-- main.rs                    // Entry point, pipeline
 |    +-- cli.rs                     // Parsing de argumentos
 |    +-- lib.rs                     // Re-exports de modulos
 |    +-- lexer/
 |    |    +-- mod.rs                // Lexer (tokenizacion)
 |    |    +-- token.rs              // Token, TokenKind, Span, Comment
 |    +-- parser/
 |    |    +-- mod.rs                // Re-export
 |    |    +-- parser.rs             // Recursive descent parser
 |    |    +-- ast.rs                // Nodos del AST
 |    +-- semantic/
 |    |    +-- mod.rs                // Re-export
 |    |    +-- type_checker.rs       // Analisis semantico
 |    +-- codegen/
 |    |    +-- mod.rs                // Re-export
 |    |    +-- interpreter.rs        // Interprete tree-walking
 |    +-- formatter/
 |    |    +-- mod.rs                // Re-export
 |    |    +-- formatter.rs          // Pretty-printer
 |    +-- utils/
 |         +-- mod.rs                // Re-export
 |         +-- error.rs              // Tipos de error, format_error
 |         +-- csv.rs                // DataTable, DataFormat
 +-- examples/
 |    +-- hello.lz                   // Ejemplo basico
 |    +-- sql_demo.lz                // Demo de SQL embebido
 |    +-- users.csv                  // Datos de prueba
 +-- LANGUAGE.md                     // Referencia del lenguaje
 +-- ARCHITECTURE.md                 // Este archivo
```

## Tests

44 tests distribuidos en 4 modulos:

| Modulo              | Tests | Que cubren                              |
|---------------------|-------|-----------------------------------------|
| `lexer/mod.rs`      | 10    | Tokenizacion de todos los tipos         |
| `parser/parser.rs`  | 13    | Parsing de todas las construcciones     |
| `formatter/formatter.rs` | 15 | Formateo, idempotencia, comentarios  |
| `utils/csv.rs`      | 6     | Carga, parsing, guardado de CSV         |
