# Instalacion de Laz como comando global

## Instalar

Desde la carpeta del proyecto:

```bash
cd C:\Users\ivanl\Desktop\Ilaz\laz
cargo install --path .
```

Esto instala `laz.exe` en `C:\Users\ivanl\.cargo\bin\`, que ya esta en tu PATH.

## Uso

Desde cualquier lugar:

```bash
laz examples/hello.lz
laz mi_programa.lz
laz --help
laz --version
```

## Actualizar despues de cambios

Si modificas el codigo fuente (keywords, features, etc.), reinstala con:

```bash
cd C:\Users\ivanl\Desktop\Ilaz\laz
cargo install --path .
```

## Desinstalar

```bash
cargo uninstall laz
```
