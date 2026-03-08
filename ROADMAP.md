# Laz Language Roadmap

Current status: **217 tests passing** · interpreter + formatter + type checker + stdlib

## ✅ Already implemented

| Feature | Status |
|---|---|
| Primitives: `int`, `float`, `bool`, `string`, `void` | ✅ |
| Variables: `let`, `let mut`, type inference, type annotation | ✅ |
| Functions: `fn name(params) -> ret`, recursion | ✅ |
| Structs: definition, field access `.`, initialisation | ✅ |
| Enums: definition, variants, `match` | ✅ |
| Control flow: `if / else if / else`, `while`, `for i in a..b` | ✅ |
| For-each: `for x in list`, `for (a, b, c) in list_of_structs` | ✅ |
| `break` / `continue` | ✅ |
| Operators: arithmetic, comparison, logical, unary `-` | ✅ |
| String concatenation with `+` | ✅ |
| `print(args...)` | ✅ |
| List literals `[1, 2, 3]`, indexing `list[i]` | ✅ |
| Lambdas / closures: `\|x\| expr`, `\|x\| { body }` | ✅ |
| Higher-order functions: `map`, `filter`, `reduce`, `sortBy` | ✅ |
| Dicts: `{k: v}`, `keys`, `values`, `get`, `push(d,k,v)`, `remove` | ✅ |
| Result / Option: `ok`, `err`, `some`, `none`, `unwrap`, `is_ok`… | ✅ |
| Try operator `?` for early return propagation | ✅ |
| `match` expressions with patterns, wildcard `_` | ✅ |
| First-class functions (assign lambda to variable) | ✅ |
| F-strings: `` `Hello {name}!` `` | ✅ |
| Generics: `fn identity<T>(x: T) -> T`, `struct Pair<A, B>` | ✅ |
| Module system: `package name;`, `import path;`, `import math as m;`, `import { fn } from pkg;` | ✅ |
| Built-in stdlib (embedded): `math`, `collections`, `strings` | ✅ |
| SQL: `#SELECT`, `#SELECT SINGLE`, `#INSERT INTO` with `WHERE` | ✅ |
| Connections: `connect file "f.csv" as alias;`, `connect db "f.db"` | ✅ |
| Type checker (semantic analysis pre-execution) | ✅ |
| Formatter: `laz fmt <file.lz> [--write]` | ✅ |
| Rich errors: L001/P001/S001/R001 with caret display | ✅ |
| Native built-ins: `len`, `push`, `pop`, `sort`, `zip`, `slice`, `concat`, `reverse`… | ✅ |
| Native math: `abs`, `sqrt`, `pow`, `sin`, `cos`, `tan`, `log`, `floor`, `ceil`, `round`… | ✅ |
| Native strings: `trim`, `lower`, `upper`, `replace`, `split`, `join`, `startsWith`, `endsWith`… | ✅ |

---

## 🔲 Roadmap (prioritised)

### Priority 1 — Core language completeness

#### 1.1 Tuples
```laz
let point = (1, 2);
let (x, y) = point;        // destructuring
fn swap<A, B>(pair: (A, B)) -> (B, A) { ... }
```
- Parser: `(a, b, c)` literal, `(A, B)` type, destructuring in `let`
- AST: `Expr::Tuple`, `TypeAnnotation::Tuple(Vec<TypeAnnotation>)`, `Pattern::Tuple`
- Interpreter: `Value::Tuple(Vec<Value>)`
- Enables `enumerate` (index, value) pairs and `zip_with`

#### 1.2 String interpolation improvements
```laz
let n = 42;
let s = f"The answer is {n:05}";   // width/padding format specifiers
```
- Currently f-strings work but without format specifiers
- Add basic width/precision/padding syntax `{expr:fmt}`

#### 1.3 `while let` / `if let`
```laz
while let some(x) = next_item() { ... }
if let ok(val) = result { ... } else { ... }
```
- Desugars to `match` with a single arm + else
- Parser: `TokenKind::While` + `TokenKind::Let` lookahead

#### 1.4 Nested pattern matching
```laz
match point {
    Point { x: 0, y } => print(y),
    Point { x, y: 0 } => print(x),
    _ => {}
}
```
- Currently `match` supports `ok(x)`, `err(e)`, `some(v)`, `none`, `_`, `Enum::Variant`, literal values
- Add: struct field patterns, tuple patterns, guard clauses `arm if condition =>`

#### 1.5 Multiple return values (via tuple)
```laz
fn divmod(a: int, b: int) -> (int, int) {
    return (a / b, a % b);
}
let (q, r) = divmod(17, 5);
```
- Depends on 1.1 (Tuples)

---

### Priority 2 — Type system improvements

#### 2.1 Type aliases
```laz
type Point = { x: float, y: float };   // struct alias
type Callback = fn(int) -> bool;        // function type alias
type Matrix = list<list<float>>;        // compound type alias
```
- Parser: `type Name = TypeAnnotation;`
- Type checker: expand aliases before checking
- Formatter: emit `type Name = ...;`

#### 2.2 `fn` type annotation for parameters
```laz
fn apply<F>(f: F, x: int) -> int { return f(x); }
// OR with explicit fn type:
fn apply(f: fn(int) -> int, x: int) -> int { return f(x); }
```
- Parser: `fn(T1, T2) -> R` type syntax
- AST: `TypeAnnotation::Fn(Vec<TypeAnnotation>, Box<TypeAnnotation>)`
- Makes HOF type-safe at the static analysis level

#### 2.3 Optional type annotation inference improvements
- Currently: let variable type is inferred at runtime
- Goal: infer types at the type-checker level to enable better static errors
- Note: significant effort, may not be needed if runtime errors are clear

#### 2.4 `const` declarations
```laz
const PI: float = 3.14159265358979;
const MAX_SIZE: int = 1024;
```
- Parser: `const Name: Type = expr;`
- Interpreter: register as immutable, error on assign
- Type checker: validate that `expr` is a literal/constant expression

---

### Priority 3 — Stdlib expansion

#### 3.1 `math.lz` additions
- `is_perfect(n)` — is n a perfect number (sum of divisors = n)
- `phi(n)` — Euler's totient function
- `combinations(n, k)` — C(n, k) = n! / (k! * (n-k)!)
- `permutations(n, k)` — P(n, k) = n! / (n-k)!
- `next_prime(n)` — smallest prime > n

#### 3.2 `collections.lz` additions
- `zip_with(xs, ys, f)` — map over pairs (needs generic lambda type)
- `partition(pred, xs)` — split list into (matching, not-matching)
- `group_by(key_fn, xs)` — group list elements by key function result
- `unique(xs)` — remove duplicate elements (needs equality)
- `enumerate(xs)` — returns (index, value) pairs (needs tuples)
- `flat_map(f, xs)` — map then flatten

#### 3.3 `strings.lz` additions
- `char_at(s, i)` — character at index as string
- `char_code(c)` — Unicode code point of character
- `from_char_code(n)` — character from code point
- `format_int(n, width)` — right-aligned integer with padding

#### 3.4 `io.lz` (file I/O)
```laz
import io;
let content = io::read_file("data.txt");
io::write_file("output.txt", content);
let lines = io::read_lines("data.txt");
```
- Needs native function hooks (Rust side) since file I/O can't be written in pure Laz
- Alternative: add `read_file(path)` and `write_file(path, content)` as native built-ins

#### 3.5 `datetime.lz`
- Date/time operations: current timestamp, formatting, arithmetic
- Needs native functions (wraps Rust's `std::time`)

---

### Priority 4 — Language features

#### 4.1 Trait / interface system
```laz
trait Printable {
    fn display(self) -> string;
}

impl Printable for Point {
    fn display(self) -> string {
        return f"({self.x}, {self.y})";
    }
}
```
- Major feature: requires method dispatch, `self` parameter, impl blocks
- Type checker: verify impl covers all trait methods
- Interpreter: method lookup on `Value::StructInstance` by type name

#### 4.2 Iterators / lazy evaluation
```laz
let evens = range(0, 100).filter(|x| x % 2 == 0).take(5);
// → [0, 2, 4, 6, 8]  (lazy, only evaluates what's needed)
```
- Chain-style method calls on lists
- Potentially lazy (generator-based) for large sequences
- Alternative: just make chaining eager (simple, no iterator protocol)

#### 4.3 Error handling improvements
```laz
fn read_int(s: string) -> Result<int, string> {
    return parseInt(s) |> ok_or("not a number");
}
let n = read_int("42")?;
```
- `ok_or(val, err_msg)` — convert Option to Result
- `map_err(result, f)` — transform error type
- `and_then(result, f)` — monadic bind for Result

#### 4.4 Variadic functions
```laz
fn sum_all(nums: ...int) -> int { ... }
sum_all(1, 2, 3, 4, 5);
```
- Parser: `...type` in last param position
- Interpreter: collect remaining args into a list

#### 4.5 Default parameter values
```laz
fn greet(name: string, greeting: string = "Hello") -> string {
    return f"{greeting}, {name}!";
}
greet("Alice");           // → "Hello, Alice!"
greet("Bob", "Hi");       // → "Hi, Bob!"
```

#### 4.6 Named arguments
```laz
draw_rect(x: 10, y: 20, width: 100, height: 50);
```
- Improves readability for functions with many parameters
- Parser: `name: expr` in argument position

---

### Priority 5 — Tooling

#### 5.1 REPL (Read-Eval-Print Loop)
```
$ laz
>>> let x = 42;
>>> print(x + 1);
43
>>> fn square(n: int) -> int { return n * n; }
>>> square(7)
49
```
- Interactive mode: `laz` with no arguments
- Multi-line support (detect incomplete input)
- History (readline integration)

#### 5.2 LSP (Language Server Protocol)
- Enables IDE integration (VS Code, Neovim, etc.)
- Features: hover types, go-to-definition, error squiggles, autocomplete
- Built on top of existing parser + type checker
- Significant effort, but major quality-of-life improvement

#### 5.3 `laz check` command
```
$ laz check myfile.lz
```
- Runs only the type checker (no execution)
- Useful for CI validation without running side effects

#### 5.4 Watch mode
```
$ laz watch myfile.lz
```
- Reruns the file on every save
- Useful for iterative development

#### 5.5 Test framework
```laz
import test;

test::describe("math module", || {
    test::it("factorial(5) == 120", || {
        test::assert_eq(math::factorial(5), 120);
    });
});
```
- Built on the existing module system
- `laz test <file.lz>` runner
- TAP or JUnit XML output for CI integration

#### 5.6 Package registry / laz.toml
```toml
[package]
name = "myapp"
version = "0.1.0"

[dependencies]
mathlib = "^1.0"
```
- `laz install` to fetch packages
- Versioned packages with semantic versioning
- Long-term goal (significant infrastructure)

---

### Priority 6 — Performance & backends

#### 6.1 Bytecode compiler + VM
- Replace tree-walking interpreter with a bytecode VM
- ~10-50x performance improvement for compute-heavy programs
- AST → bytecode → stack-based VM
- Preserve same semantics, add `laz run --vm` flag

#### 6.2 LLVM backend (experimental)
- `c_backend.rs` and `llvm.rs` already have stubs
- `laz compile myfile.lz -o myapp` → native binary
- Requires type inference to be complete for code generation
- Major engineering effort

#### 6.3 WASM target
- Compile Laz to WebAssembly for browser execution
- Useful for embedding Laz scripts in web apps

---

## Suggested next steps (short term)

1. **Tuples** — enables many patterns (enumerate, divmod, zip_with), relatively self-contained
2. **`while let` / `if let`** — quality-of-life, small implementation effort
3. **`fn` type annotations** — completes the generic/HOF story
4. **REPL** — great for demos and learning the language
5. **`laz check` command** — easy win for tooling
6. **`io.lz` stdlib** — adds file I/O, very practical
