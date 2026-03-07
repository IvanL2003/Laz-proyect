# Laz - Referencia del Lenguaje

Laz es un lenguaje de programacion imperativo con sintaxis inspirada en C y Rust. Se ejecuta mediante un interprete tree-walking escrito en Rust.

Archivos fuente: `.lz`

## Tipos

### Primitivos

| Tipo     | Descripcion              | Ejemplo          |
|----------|--------------------------|------------------|
| `int`    | Entero de 64 bits        | `42`, `-7`       |
| `float`  | Punto flotante de 64 bits| `3.14`, `1.0`    |
| `bool`   | Booleano                 | `true`, `false`  |
| `string` | Cadena de texto          | `"hola"`         |
| `void`   | Sin valor                | (retorno implicito) |

### Compuestos

| Tipo          | Descripcion                      | Ejemplo              |
|---------------|----------------------------------|----------------------|
| `list<T>`     | Lista de elementos de tipo T     | `list<User>`, `list<list<string>>` |
| `Struct`       | Tipos definidos por el usuario   | `User`, `Point`      |

## Variables

```laz
// Con tipo explicito
let x: int = 42;
let name: string = "Laz";

// Con tipo inferido (el tipo se deduce de la expresion)
let x = 42;           // int
let pi = 3.14;        // float
let msg = "hello";    // string
let ok = true;        // bool

// Variable mutable
let mut counter: int = 0;
counter = counter + 1;
```

Las variables son inmutables por defecto. Usar `let mut` para permitir reasignacion.

## Funciones

```laz
fn nombre(param1: tipo1, param2: tipo2) -> tipo_retorno {
    // cuerpo
    return valor;
}
```

Ejemplo:

```laz
fn factorial(n: int) -> int {
    if n <= 1 {
        return 1;
    }
    return n * factorial(n - 1);
}

fn greet(name: string) -> void {
    print("Hello, ", name);
}
```

- Los parametros siempre requieren anotacion de tipo
- El tipo de retorno es obligatorio (usar `void` si no retorna nada)
- Soporta recursion
- `main()` se ejecuta automaticamente si existe

## Structs

### Definicion

```laz
struct Point {
    x: float,
    y: float,
}
```

### Inicializacion

```laz
let p = Point { x: 1.0, y: 2.0 };
```

### Acceso a campos

```laz
print(p.x);      // 1.0
print(p.y);      // 2.0
```

### Asignacion a campos (si la variable es mutable)

```laz
let mut p = Point { x: 0.0, y: 0.0 };
p.x = 5.0;
```

## Control de Flujo

### if / else if / else

```laz
if condition {
    // ...
} else if other_condition {
    // ...
} else {
    // ...
}
```

Las condiciones deben ser de tipo `bool`.

### while

```laz
let mut n: int = 10;
while n > 0 {
    print(n);
    n = n - 1;
}
```

### for (rango)

```laz
for i in 1..10 {
    print(i);    // 1, 2, 3, ..., 9 (exclusivo)
}
```

El rango `start..end` es exclusivo en el extremo superior. Ambos limites deben ser `int`.

## Operadores

### Aritmeticos

| Operador | Descripcion    | Tipos soportados   |
|----------|----------------|--------------------|
| `+`      | Suma / Concat  | int, float, string |
| `-`      | Resta          | int, float         |
| `*`      | Multiplicacion | int, float         |
| `/`      | Division       | int, float         |
| `%`      | Modulo         | int                |

La suma `+` con strings realiza concatenacion.

### Comparacion

| Operador | Descripcion      |
|----------|------------------|
| `==`     | Igual            |
| `!=`     | Distinto         |
| `<`      | Menor            |
| `<=`     | Menor o igual    |
| `>`      | Mayor            |
| `>=`     | Mayor o igual    |

### Logicos

| Operador | Descripcion | Evaluacion     |
|----------|-------------|----------------|
| `&&`     | AND logico  | Short-circuit  |
| `\|\|`   | OR logico   | Short-circuit  |
| `!`      | NOT logico  | Unario         |

### Unarios

| Operador | Descripcion      | Tipos        |
|----------|------------------|--------------|
| `-`      | Negacion         | int, float   |
| `!`      | Negacion logica  | bool         |

## Print

`print` es un statement (no una funcion). Acepta multiples argumentos separados por coma:

```laz
print("El resultado es:", resultado);
print(42);
print("Nombre:", user.name, "Edad:", user.age);
```

Los argumentos se imprimen separados por espacios.

## typeOf

Funcion built-in que devuelve el tipo de un valor como string:

```laz
let x = 42;
print(typeOf(x));         // "int"
print(typeOf(3.14));      // "float"
print(typeOf("hello"));   // "string"
print(typeOf(true));      // "bool"
```

Valores posibles de retorno: `"int"`, `"float"`, `"bool"`, `"string"`, `"list"`, `"void"`, o el nombre del struct (ej: `"User"`, `"Point"`).

## Conexiones a Archivos

```laz
connect file "ruta/archivo.csv" as alias;
connect db "connection_string" as alias;    // futuro
connect api "url" as alias;                 // futuro
```

Actualmente soporta archivos `.csv`. El formato se detecta automaticamente por extension.

## SQL Embebido

Laz permite consultas SQL sobre archivos conectados. Las queries se prefijan con `#`.

### SELECT

```laz
// Todas las filas
let users: list<User> = #SELECT * FROM users;

// Columnas especificas con WHERE
let result: list<User> = #SELECT name, age FROM users WHERE age > 25;

// SINGLE devuelve un solo elemento (no lista)
let bob: User = #SELECT SINGLE * FROM users WHERE name == "Bob";
```

### FROM: dos formas

```laz
// 1. Usando alias (requiere connect previo)
let data = #SELECT * FROM users;

// 2. Usando file() inline (ruta directa)
let data = #SELECT * FROM file("datos.csv");
```

### INSERT

```laz
let ok: bool = #INSERT INTO users VALUES ("Frank", 40, "Bilbao");
```

Devuelve `true` si la insercion fue exitosa.

### Modos de tipado SQL

| Tipo de variable        | Modo           | Comportamiento                        |
|-------------------------|----------------|---------------------------------------|
| `list<User>`            | Struct         | Cada fila se convierte en un struct   |
| `list<list<string>>`    | String         | Cada fila es una lista de strings     |
| Sin tipo (`let x = ...`)| Struct (default)| Usa modo struct por defecto          |

El modo string se activa cuando el tipo es `list<list<string>>` o cualquier `list<primitivo>`.

### Condiciones WHERE

Las condiciones WHERE reusan las expresiones normales del lenguaje:

```laz
WHERE age > 25
WHERE name == "Bob"
WHERE city == "Madrid" && age > 20
```

## Comentarios

Solo se soportan comentarios de linea:

```laz
// Esto es un comentario
let x = 42; // Comentario inline
```

## Herramientas CLI

```bash
# Ejecutar un programa
laz programa.lz

# Formatear (mostrar en stdout)
laz fmt programa.lz

# Formatear y sobreescribir el archivo
laz fmt --write programa.lz

# Version
laz --version

# Ayuda
laz --help
```

## Ejemplo Completo

```laz
connect file "users.csv" as users;

struct User {
    name: string,
    age: int,
    city: string,
}

fn is_adult(user: User) -> bool {
    return user.age >= 18;
}

fn main() -> void {
    // Variables con y sin tipo
    let greeting = "Bienvenido a Laz";
    let limit: int = 30;

    print(greeting);

    // Query SQL
    let adults: list<User> = #SELECT * FROM users WHERE age >= 18;
    print("Adultos:", adults);

    // SELECT SINGLE
    let bob: User = #SELECT SINGLE * FROM users WHERE name == "Bob";
    print("Bob:", bob.name, "edad:", bob.age);

    // typeOf
    print(typeOf(bob));       // "User"
    print(typeOf(limit));     // "int"

    // Bucle
    for i in 1..5 {
        print("Iteracion:", i);
    }
}
```
