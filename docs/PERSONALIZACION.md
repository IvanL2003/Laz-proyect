# Guia de Personalizacion de Laz

Esta guia te explica exactamente donde y como cambiar cada parte de la sintaxis del lenguaje Laz.

> **Regla de oro:** Las keywords (palabras clave) solo se cambian en UN archivo: `src/lexer/token.rs`. No hay que tocar ni el parser ni el interpreter porque internamente usan el enum de Rust (`TokenKind::Fn`, `TokenKind::Let`, etc.), no los strings.

---

## 1. Tabla de Keywords Actuales

Todas las keywords se definen en `src/lexer/token.rs`, funcion `lookup_keyword()` (lineas 139-161):

| Keyword actual | Token interno     | Linea | Ejemplo de cambio      |
|----------------|-------------------|-------|------------------------|
| `fn`           | `TokenKind::Fn`   | 141   | `"func"`, `"funcion"`  |
| `let`          | `TokenKind::Let`  | 142   | `"sea"`, `"var"`       |
| `mut`          | `TokenKind::Mut`  | 143   | `"mut"`, `"mutable"`   |
| `if`           | `TokenKind::If`   | 144   | `"si"`                 |
| `else`         | `TokenKind::Else` | 145   | `"sino"`, `"otro"`     |
| `while`        | `TokenKind::While`| 146   | `"mientras"`           |
| `for`          | `TokenKind::For`  | 147   | `"para"`               |
| `in`           | `TokenKind::In`   | 148   | `"en"`                 |
| `return`       | `TokenKind::Return`| 149  | `"retorna"`, `"devolver"` |
| `print`        | `TokenKind::Print`| 150   | `"imprimir"`, `"mostrar"` |
| `struct`       | `TokenKind::Struct`| 151  | `"estructura"`, `"tipo"` |
| `true`         | `BoolLiteral(true)`| 152  | `"verdadero"`, `"si"`  |
| `false`        | `BoolLiteral(false)`| 153 | `"falso"`, `"no"`      |
| `int`          | `TokenKind::IntType`| 154 | `"entero"`             |
| `float`        | `TokenKind::FloatType`| 155 | `"decimal"`          |
| `bool`         | `TokenKind::BoolType`| 156 | `"logico"`            |
| `string`       | `TokenKind::StringType`| 157 | `"texto"`            |
| `void`         | `TokenKind::VoidType`| 158 | `"nada"`, `"vacio"`   |

---

## 2. Como Cambiar una Keyword (Paso a Paso)

### Ejemplo: Cambiar `fn` por `func`

#### Paso 1: Editar `src/lexer/token.rs` linea 141

```rust
// ANTES (linea 141):
"fn" => Some(TokenKind::Fn),

// DESPUES:
"func" => Some(TokenKind::Fn),
```

Eso es todo para que el lexer reconozca `func` como la keyword de funciones.

#### Paso 2: (Opcional) Actualizar el Display para mensajes de error

En el mismo archivo, linea 93:

```rust
// ANTES (linea 93):
TokenKind::Fn => write!(f, "fn"),

// DESPUES:
TokenKind::Fn => write!(f, "func"),
```

Esto hace que los mensajes de error digan `expected 'func'` en lugar de `expected 'fn'`.

#### Paso 3: Actualizar los tests en `src/lexer/mod.rs`

Busca los tests que usan la keyword vieja. Por ejemplo, linea 366:

```rust
// ANTES:
let kinds = tokenize("fn add(a: int, b: int) -> int { }");

// DESPUES:
let kinds = tokenize("func add(a: int, b: int) -> int { }");
```

#### Paso 4: Actualizar `examples/hello.laz`

Cambia todas las ocurrencias de `fn` por `func` en el archivo de ejemplo.

#### NOTA IMPORTANTE

**NO hay que tocar estos archivos:**
- `src/parser/parser.rs` — usa `TokenKind::Fn`, no el string `"fn"`
- `src/codegen/interpreter.rs` — usa `FnDecl` del AST, no strings
- `src/semantic/type_checker.rs` — usa el AST, no strings
- `src/parser/ast.rs` — define tipos abstractos, no strings

---

## 3. Ejemplo Completo: Pasar a Espanol

Si quisieras que Laz use keywords en espanol, solo cambias `src/lexer/token.rs` linea 139-161:

```rust
pub fn lookup_keyword(ident: &str) -> Option<TokenKind> {
    match ident {
        "func"       => Some(TokenKind::Fn),
        "sea"        => Some(TokenKind::Let),
        "mut"        => Some(TokenKind::Mut),
        "si"         => Some(TokenKind::If),
        "sino"       => Some(TokenKind::Else),
        "mientras"   => Some(TokenKind::While),
        "para"       => Some(TokenKind::For),
        "en"         => Some(TokenKind::In),
        "retorna"    => Some(TokenKind::Return),
        "imprimir"   => Some(TokenKind::Print),
        "estructura" => Some(TokenKind::Struct),
        "verdadero"  => Some(TokenKind::BoolLiteral(true)),
        "falso"      => Some(TokenKind::BoolLiteral(false)),
        "int"        => Some(TokenKind::IntType),
        "float"      => Some(TokenKind::FloatType),
        "bool"       => Some(TokenKind::BoolType),
        "string"     => Some(TokenKind::StringType),
        "void"       => Some(TokenKind::VoidType),
        _ => None,
    }
}
```

Y tu programa se veria asi:

```
estructura Punto {
    x: float,
    y: float,
}

func principal() -> void {
    sea x: int = 42;
    sea mut nombre: string = "Laz";

    si x > 10 {
        imprimir("grande");
    } sino {
        imprimir("pequeno");
    }

    para i en 0..10 {
        imprimir(i);
    }

    mientras x > 0 {
        x = x - 1;
    }

    retorna;
}
```

> **Recuerda:** Tambien actualiza el Display (lineas 93-108) para que los mensajes de error usen tus keywords.

---

## 4. Como Cambiar Operadores

Los operadores son diferentes a las keywords porque se detectan **caracter por caracter** en `src/lexer/mod.rs`, funcion `scan_token()` (lineas 56-178).

### Ejemplo: Cambiar `&&` por `y`, `||` por `o`

Esto es mas complejo porque `y` y `o` son texto, no simbolos. Tienes dos opciones:

#### Opcion A: Hacerlos keywords (RECOMENDADO)

1. En `src/lexer/token.rs`, anade en `lookup_keyword()`:
```rust
"y" => Some(TokenKind::And),
"o" => Some(TokenKind::Or),
```

2. En `src/lexer/mod.rs`, elimina los bloques de `'&'` (lineas 130-140) y `'|'` (lineas 142-152).

3. Actualiza el Display:
```rust
TokenKind::And => write!(f, "y"),
TokenKind::Or => write!(f, "o"),
```

#### Opcion B: Cambiar el simbolo

Si solo quieres cambiar `&&` por otro simbolo (ej: `@@`), edita `src/lexer/mod.rs`:

```rust
// ANTES (linea 130):
'&' => {
    if self.peek() == '&' {

// DESPUES:
'@' => {
    if self.peek() == '@' {
```

---

## 5. Como Cambiar el Estilo de Comentarios

Los comentarios se detectan en `src/lexer/mod.rs`, lineas 33-37:

```rust
// ANTES:
if self.peek() == '/' && self.peek_next() == Some('/') {
    self.skip_line_comment();
    continue;
}
```

### Ejemplo: Cambiar `//` por `#`

```rust
// DESPUES:
if self.peek() == '#' {
    self.skip_line_comment();
    continue;
}
```

**IMPORTANTE:** Si usas `#`, tambien necesitas quitar la deteccion de `/` como operador de division en `scan_token()` si hay conflicto. Con `#` no hay conflicto porque `#` no se usa para nada mas.

Si quieres comentarios multilinea (ej: `/* ... */`), necesitas anadir una funcion nueva `skip_block_comment()` en el lexer.

---

## 6. Como Cambiar la Extension de Archivos

Si quieres usar `.lz` en vez de `.laz`:

1. **`src/cli.rs`** — Cambia los mensajes de uso (lineas 14, 23):
```rust
// ANTES:
"Usage: {} <filename.laz>\n..."

// DESPUES:
"Usage: {} <filename.lz>\n..."
```

2. **`README.md`** — Actualiza los ejemplos de uso.

3. **Renombra** `examples/hello.laz` a `examples/hello.lz`.

> Nota: El lenguaje no valida la extension del archivo, asi que esto es solo cosmetico.

---

## 7. Como Cambiar el Simbolo de Asignacion

Si quieres usar `:=` en vez de `=` para asignar:

1. En `src/lexer/mod.rs`, cambia el bloque de `:` (linea 72):
```rust
// ANTES:
':' => TokenKind::Colon,

// DESPUES:
':' => {
    if self.peek() == '=' {
        self.advance();
        TokenKind::Equal  // := es asignacion
    } else {
        TokenKind::Colon
    }
}
```

2. Elimina `=` como token de asignacion (o dejalo solo para `==`).

---

## 8. Como Anadir una Keyword Nueva

Si quieres anadir una keyword que no existe (ej: `const`):

#### Paso 1: Anadir el token en `src/lexer/token.rs`

En el enum `TokenKind` (despues de linea 37):
```rust
Const,  // nueva keyword
```

En `Display` (despues de linea 103):
```rust
TokenKind::Const => write!(f, "const"),
```

En `lookup_keyword()` (despues de linea 151):
```rust
"const" => Some(TokenKind::Const),
```

#### Paso 2: Usar el token en el parser `src/parser/parser.rs`

En `parse_statement()` (linea ~185), anade un nuevo caso:
```rust
TokenKind::Const => self.parse_const_stmt(),
```

Y crea la funcion `parse_const_stmt()`.

#### Paso 3: Anadir al AST `src/parser/ast.rs`

Anade una nueva variante al enum `Stmt`:
```rust
Const {
    name: String,
    type_ann: TypeAnnotation,
    value: Expr,
    span: Span,
},
```

#### Paso 4: Manejar en el interpreter `src/codegen/interpreter.rs`

En `execute_stmt()`, anade el caso para `Stmt::Const`.

---

## 9. Referencia Rapida de Archivos

| Que cambiar                | Archivo                          | Lineas      |
|---------------------------|----------------------------------|-------------|
| Keywords (texto)          | `src/lexer/token.rs`             | 139-161     |
| Keywords (display/error)  | `src/lexer/token.rs`             | 85-136      |
| Operadores de simbolos    | `src/lexer/mod.rs`               | 56-178      |
| Comentarios               | `src/lexer/mod.rs`               | 33-37       |
| Strings (escapes)         | `src/lexer/mod.rs`               | 181-230     |
| AST (nuevos nodos)        | `src/parser/ast.rs`              | todo        |
| Parser (nueva sintaxis)   | `src/parser/parser.rs`           | todo        |
| Ejecucion (nueva logica)  | `src/codegen/interpreter.rs`     | todo        |
| Validacion semantica      | `src/semantic/type_checker.rs`   | todo        |
| CLI y extension            | `src/cli.rs`                     | 14, 21-23   |
| Tests del lexer           | `src/lexer/mod.rs`               | 336-479     |
| Tests del parser          | `src/parser/parser.rs`           | 470+        |

---

## 10. Despues de Cada Cambio

Siempre ejecuta:

```bash
cargo build    # Verifica que compile
cargo test     # Verifica que los tests pasen
cargo run -- examples/hello.laz   # Verifica que funcione
```

Si `cargo build` falla, lee el error: Rust te dice exactamente la linea y el problema.
