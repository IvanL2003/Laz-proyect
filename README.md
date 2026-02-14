# Laz

A statically-typed imperative programming language written in Rust.

## Features

- **Types**: `int`, `float`, `bool`, `string`, `void`, user-defined structs
- **Variables**: explicit type annotations, immutable by default (`let mut` for mutable)
- **Functions**: with typed parameters and return types
- **Control flow**: `if`/`else`, `while`, `for i in start..end`
- **Structs**: value-type structs with field access
- **Operators**: arithmetic (`+`, `-`, `*`, `/`, `%`), comparison, logical (`&&`, `||`, `!`)
- **String concatenation** with `+`
- **Comments**: `// single line`

## Build

```bash
cargo build --release
```

## Usage

```bash
cargo run -- examples/hello.lz
```

Or after building:

```bash
./target/release/laz examples/hello.lz
```

## Language Syntax

```
// Variables
let x: int = 42;
let mut name: string = "Laz";
let pi: float = 3.14;
let active: bool = true;

// Functions
fn add(a: int, b: int) -> int {
    return a + b;
}

// Structs
struct Point {
    x: float,
    y: float,
}

let p: Point = Point { x: 1.0, y: 2.0 };
print(p.x);

// Control flow
if x > 10 {
    print("big");
} else {
    print("small");
}

while x > 0 {
    x = x - 1;
}

for i in 0..10 {
    print(i);
}

// Entry point
fn main() -> void {
    print("Hello, Laz!");
}
```

## Project Structure

```
src/
  main.rs          - Entry point, orchestrates the pipeline
  lib.rs           - Module declarations
  cli.rs           - Command-line argument parsing
  lexer/           - Tokenizer (source text -> tokens)
  parser/          - Parser (tokens -> AST)
  semantic/        - Pre-execution validation
  codegen/         - Tree-walking interpreter
  utils/           - Error types and formatting
```
