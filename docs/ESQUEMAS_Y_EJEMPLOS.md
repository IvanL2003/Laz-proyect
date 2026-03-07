# Esquemas y ejemplos del lenguaje Laz

## Ejemplo de uso de `typeOf`

```lz
let x = 42;
let y = 3.14;
let s = "hola";
let b = true;
struct Persona { nombre: string, edad: int }
let p = Persona { nombre: "Ana", edad: 30 };

print(typeOf(x)); // int
print(typeOf(y)); // float
print(typeOf(s)); // string


print(typeOf(b)); // bool
print(typeOf(p)); // Persona
```

## Esquema de declaración de estructuras

```lz
struct NombreEstructura {
    campo1: tipo1,
    campo2: tipo2,
    // ...
}

// Ejemplo
struct Usuario {
    nombre: string,
    edad: int,
    activo: bool,
}
```

## Esquema de declaración de funciones

```lz
fn nombre_funcion(param1: tipo1, param2: tipo2) -> tipo_retorno {
    // cuerpo
}

// Ejemplo
fn suma(a: int, b: int) -> int {
    return a + b;
}
```

## Esquema de uso de condicionales y bucles

```lz
if condicion {
    // bloque si verdadero
} else {
    // bloque si falso
}

while condicion {
    // bloque mientras
}

for i in 1..10 {
    print(i);
}
```

## Esquema de conexión a CSV

```lz
connect file "archivo.csv" as alias;
let fila = #SELECT SINGLE * FROM alias WHERE condicion;
```

---

*Consulta este archivo para ejemplos rápidos de sintaxis y uso de las principales características del lenguaje.*
