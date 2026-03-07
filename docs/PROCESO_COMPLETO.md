# Proceso de Uso del Lenguaje Laz

Este documento describe el proceso completo para instalar, ejecutar y extender el lenguaje Laz.

## 1. Instalación
- Clona el repositorio o descarga el código fuente.
- Asegúrate de tener Rust instalado.
- Compila el proyecto:

```sh
cargo build --release
```

## 2. Ejecución de Programas
- Ejecuta un archivo `.lz` usando la CLI:

```sh
laz archivo.lz
```
- Ejemplo:
```sh
laz examples/hello.lz
```

## 3. Formateo de Código
- Usa el formateador integrado:

```sh
laz format archivo.lz
```

## 4. Uso de Archivos CSV
- Puedes leer y escribir archivos CSV directamente desde el lenguaje.

## 5. Extensión del Lenguaje
- Para agregar nuevas funcionalidades:
  1. Añade módulos en `src/` según la arquitectura.
  2. Implementa la lógica en el intérprete o backend correspondiente.
  3. Actualiza la CLI si es necesario.

## 6. Ejemplos
- Consulta la carpeta `examples/` para ver ejemplos de uso.

## 7. Documentación
- Revisa los archivos en `docs/` para más detalles sobre personalización e instalación.

---

*Este proceso cubre desde la instalación hasta la extensión del lenguaje. Actualiza este documento si el flujo cambia.*
