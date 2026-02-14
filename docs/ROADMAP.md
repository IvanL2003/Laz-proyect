# Roadmap de Laz - Features Futuras

Este documento describe las features que se pueden anadir a Laz y exactamente que archivos y que cambios requiere cada una.

---

## Estado Actual (v0.1)

- [x] Tipos basicos: `int`, `float`, `bool`, `string`, `void`
- [x] Variables con `let` / `let mut`
- [x] Funciones con `fn`
- [x] Structs
- [x] Control de flujo: `if`/`else`, `while`, `for..in`
- [x] Operadores aritmeticos, comparacion, logicos
- [x] Concatenacion de strings con `+`
- [x] Comentarios `//`
- [x] Print built-in
- [x] Analisis semantico basico
- [x] Mensajes de error con linea y columna

---

## Feature 1: Arrays / Listas

**Prioridad:** Alta
**Complejidad:** Media
**Sintaxis propuesta:**

```
sea nums: array[int] = [1, 2, 3, 4, 5];
imprimir(nums[0]);        // 1
imprimir(nums.len());     // 5
nums[2] = 99;
```

### Que cambiar:

#### 1. `src/lexer/token.rs` — Nuevos tokens
```rust
// Anadir al enum TokenKind:
LeftBracket,   // [
RightBracket,  // ]
```

En `Display`:
```rust
TokenKind::LeftBracket => write!(f, "["),
TokenKind::RightBracket => write!(f, "]"),
```

En `lookup_keyword()`:
```rust
"array" => Some(TokenKind::ArrayType),  // si quieres keyword "array"
```

#### 2. `src/lexer/mod.rs` — Reconocer `[` y `]`
En `scan_token()`, anadir:
```rust
'[' => TokenKind::LeftBracket,
']' => TokenKind::RightBracket,
```

#### 3. `src/parser/ast.rs` — Nuevos nodos AST
```rust
// Nuevo tipo:
TypeAnnotation::Array(Box<TypeAnnotation>),  // array[int]

// Nuevas expresiones:
Expr::ArrayLiteral {
    elements: Vec<Expr>,
    span: Span,
},
Expr::IndexAccess {
    array: Box<Expr>,
    index: Box<Expr>,
    span: Span,
},

// Nuevo target de asignacion:
AssignTarget::Index {
    array: Box<Expr>,
    index: Box<Expr>,
},
```

#### 4. `src/parser/parser.rs` — Parsear arrays
- En `parse_type()`: reconocer `array[int]`, `array[float]`, etc.
- En `parse_primary()`: reconocer `[expr, expr, ...]` como ArrayLiteral
- En `parse_call()`: reconocer `expr[index]` como IndexAccess (similar a `expr.field`)
- En `parse_assign_or_expr_stmt()`: manejar `arr[i] = value;`

#### 5. `src/codegen/interpreter.rs` — Ejecutar arrays
```rust
// Nuevo Value:
Value::Array(Vec<Value>),

// En evaluate_expr():
// - ArrayLiteral: evaluar cada elemento, crear Value::Array
// - IndexAccess: obtener el Value::Array, verificar indice, retornar elemento

// En execute_stmt():
// - Asignacion a indice: obtener array, modificar elemento, guardar de vuelta

// Built-in methods (en parse_call o como metodos especiales):
// - .len() -> Int
// - .push(value) -> modifica el array
// - .pop() -> retorna y elimina el ultimo
```

#### 6. `src/semantic/type_checker.rs`
- Validar que el indice sea una expresion valida
- Validar tipos de elementos si es posible

---

## Feature 2: Enums

**Prioridad:** Media
**Complejidad:** Media-Alta
**Sintaxis propuesta:**

```
enum Color {
    Rojo,
    Verde,
    Azul,
}

sea c: Color = Color::Rojo;

si c == Color::Rojo {
    imprimir("Es rojo!");
}
```

### Que cambiar:

#### 1. `src/lexer/token.rs`
```rust
// Nuevo token:
Enum,           // keyword enum
ColonColon,     // ::
```

En `lookup_keyword()`:
```rust
"enum" => Some(TokenKind::Enum),
```

#### 2. `src/lexer/mod.rs`
En `scan_token()`, modificar el caso de `:`:
```rust
':' => {
    if self.peek() == ':' {
        self.advance();
        TokenKind::ColonColon
    } else {
        TokenKind::Colon
    }
}
```

#### 3. `src/parser/ast.rs`
```rust
// Nueva declaracion:
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<String>,
    pub span: Span,
}

// En Declaration:
Declaration::Enum(EnumDecl),

// Nueva expresion:
Expr::EnumVariant {
    enum_name: String,
    variant: String,
    span: Span,
},
```

#### 4. `src/parser/parser.rs`
- En `parse_declaration()`: reconocer `TokenKind::Enum` y llamar `parse_enum_decl()`
- `parse_enum_decl()`: parsear `enum Name { Variante1, Variante2 }`
- En `parse_primary()` o `parse_call()`: reconocer `Name::Variante`

#### 5. `src/codegen/interpreter.rs`
```rust
// Nuevo Value:
Value::EnumVariant {
    enum_name: String,
    variant: String,
},

// En run(): registrar enums igual que structs
// En evaluate_expr(): crear EnumVariant
// En comparison_op(): permitir comparar EnumVariants por igualdad
```

### Futuro: Pattern Matching
```
match c {
    Color::Rojo => imprimir("rojo"),
    Color::Verde => imprimir("verde"),
    Color::Azul => imprimir("azul"),
}
```
Esto requiere un nuevo statement `Stmt::Match` con `MatchArm { pattern, body }`.

---

## Feature 3: Imports / Modulos

**Prioridad:** Media
**Complejidad:** Media
**Sintaxis propuesta:**

```
import "matematicas.laz";

func principal() -> void {
    sea resultado: float = raiz_cuadrada(25.0);
    imprimir(resultado);
}
```

### Que cambiar:

#### 1. `src/lexer/token.rs`
```rust
// Nueva keyword:
Import,
```

En `lookup_keyword()`:
```rust
"import" => Some(TokenKind::Import),
```

#### 2. `src/parser/ast.rs`
```rust
// Nueva declaracion:
Declaration::Import {
    path: String,   // ruta del archivo
    span: Span,
},
```

#### 3. `src/parser/parser.rs`
En `parse_declaration()`:
```rust
TokenKind::Import => self.parse_import(),
```

`parse_import()`:
```rust
fn parse_import(&mut self) -> Result<Declaration, ParseError> {
    let token = self.advance(); // consume 'import'
    let path_token = self.expect(&TokenKind::StringLiteral("".into()))?;
    // extraer el path del string
    self.expect(&TokenKind::Semicolon)?;
    Ok(Declaration::Import { path, span: token.span })
}
```

#### 4. `src/main.rs` o `src/codegen/interpreter.rs`
La logica de imports:
1. Leer el archivo importado
2. Lexear y parsear el archivo
3. Registrar sus funciones y structs en el interpreter
4. Manejar imports circulares (guardar set de archivos ya importados)

#### 5. Resolucion de paths
- Relativo al archivo actual: `import "utils.laz";`
- Relativo al proyecto: `import "lib/math.laz";`

---

## Feature 4: Closures / Funciones Anonimas

**Prioridad:** Baja
**Complejidad:** Alta
**Sintaxis propuesta:**

```
sea doble: func = |x: int| -> int { retorna x * 2; };
sea resultado: int = doble(5);  // 10
```

### Que cambiar:
- `token.rs`: tokens `|` (Pipe)
- `ast.rs`: `Expr::Closure { params, return_type, body }`
- `interpreter.rs`: `Value::Closure { params, body, captured_env }` — requiere capturar el environment en el momento de creacion
- `parser.rs`: parsear la sintaxis `|params| -> type { body }`

> **Nota:** Esta feature es compleja porque requiere capturar variables del scope padre (closures). En v1, las funciones NO capturan variables externas.

---

## Feature 5: Traits / Interfaces

**Prioridad:** Baja
**Complejidad:** Alta
**Sintaxis propuesta:**

```
trait Imprimible {
    func to_string() -> string;
}

estructura Punto {
    x: float,
    y: float,
}

impl Imprimible para Punto {
    func to_string() -> string {
        retorna "(" + x + ", " + y + ")";
    }
}
```

### Que cambiar:
- Nuevas keywords: `trait`, `impl`, `para` (o `for`)
- Nuevos AST nodes: `TraitDecl`, `ImplBlock`
- Method dispatch en el interpreter: buscar metodos por tipo
- Type checker: verificar que las implementaciones cumplan el trait

---

## Feature 6: Error Handling (try/catch)

**Prioridad:** Media
**Complejidad:** Media
**Sintaxis propuesta:**

```
func dividir(a: int, b: int) -> int {
    si b == 0 {
        error("Division por cero!");
    }
    retorna a / b;
}

intentar {
    sea x: int = dividir(10, 0);
} capturar e {
    imprimir("Error:", e);
}
```

### Que cambiar:
- Keywords: `intentar`/`try`, `capturar`/`catch`, `error`/`throw`
- AST: `Stmt::TryCatch { try_block, catch_var, catch_block }`
- Interpreter: `StmtResult::Error(String)` que propaga como Return pero se captura en try/catch
- Nuevo built-in statement `error("mensaje")` que lanza un error

---

## Feature 7: REPL Interactivo

**Prioridad:** Media
**Complejidad:** Baja
**Uso:**

```bash
$ laz
Laz v0.1.0 - REPL interactivo
>>> sea x: int = 42;
>>> imprimir(x);
42
>>> x + 8
50
```

### Que cambiar:
- `src/cli.rs`: si no se pasa archivo, entrar en modo REPL
- Nuevo archivo `src/repl.rs`:
  - Loop: leer linea → lex → parse → interpret → mostrar resultado
  - Mantener el environment entre lineas
  - Manejar expresiones sueltas (imprimir el resultado automaticamente)
  - Historial con readline (crate `rustyline` como dependencia)

---

## Feature 8: Comentarios Multilinea

**Prioridad:** Baja
**Complejidad:** Baja
**Sintaxis:**

```
/* Este es un
   comentario multilinea */
```

### Que cambiar:
Solo `src/lexer/mod.rs`:

En `tokenize()`, despues de la deteccion de `//` (linea 34):
```rust
// Comentarios multilinea
if self.peek() == '/' && self.peek_next() == Some('*') {
    self.skip_block_comment()?;
    continue;
}
```

Nueva funcion:
```rust
fn skip_block_comment(&mut self) -> Result<(), LexerError> {
    self.advance(); // consume /
    self.advance(); // consume *
    loop {
        if self.is_at_end() {
            return Err(LexerError {
                message: "unterminated block comment".to_string(),
                span: ...,
            });
        }
        if self.peek() == '*' && self.peek_next() == Some('/') {
            self.advance(); // consume *
            self.advance(); // consume /
            return Ok(());
        }
        self.advance();
    }
}
```

---

## Orden de Implementacion Recomendado

1. **Comentarios multilinea** — Cambio minimo, solo lexer
2. **Arrays** — Feature mas util, complejidad moderada
3. **REPL** — Mejora mucho la experiencia de desarrollo
4. **Enums** — Necesario para programas mas complejos
5. **Imports** — Permite organizar proyectos grandes
6. **Error handling** — Hace el lenguaje mas robusto
7. **Closures** — Feature avanzada
8. **Traits** — Feature avanzada, requiere closures primero

---

## Arquitectura: Donde Vive Cada Cosa

```
Codigo fuente (.laz)
       |
       v
   LEXER (src/lexer/)
   Lee caracteres, produce tokens
   Aqui se definen keywords y operadores
       |
       v
   PARSER (src/parser/)
   Lee tokens, produce AST (arbol de sintaxis)
   Aqui se define la gramatica (que es valido escribir)
       |
       v
   SEMANTIC (src/semantic/)
   Valida el AST antes de ejecutar
   Aqui se verifican errores que no son de sintaxis
       |
       v
   INTERPRETER (src/codegen/)
   Recorre el AST y ejecuta el programa
   Aqui se define que hace cada instruccion
```

Para cualquier feature nueva, el flujo es siempre:
1. Anadir tokens en `lexer/token.rs`
2. Anadir nodos AST en `parser/ast.rs`
3. Parsear la sintaxis en `parser/parser.rs`
4. (Opcional) Validar en `semantic/type_checker.rs`
5. Ejecutar en `codegen/interpreter.rs`
6. Escribir tests
7. `cargo build && cargo test`
