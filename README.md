# Lorenz

Entorno de diseno y simulacion de computadoras. El usuario describe la arquitectura de una maquina
en un archivo declarativo, y a partir de esa descripcion se generan el ensamblador, el emulador,
el depurador y la documentacion de la ISA.

Incluye **SGT (Saggitarius)**, un lenguaje de nivel medio que conoce la maquina para la que compila.

## Estado

Fase 1 en curso: nucleo y formato de descripcion de arquitectura.

| Componente | Estado |
| --- | --- |
| `lorenz-arch` — formato y validacion | en desarrollo |
| `lorenz-emu` — emulador funcional | pendiente |
| `lorenz-asm` — ensamblador y desensamblador | pendiente |
| `lorenz-cli` — linea de comandos | pendiente |
| `sgt` — compilador del lenguaje | pendiente |

## Documentacion del proyecto

- `docs/BITACORA.md` — registro de lo que se va haciendo, sesion por sesion.
- `docs/GIT-WORKFLOW.md` — ramas, convencion de commits y politica de granularidad.
- `docs/ARQUITECTURA.md` — como se organiza el codigo del propio Lorenz.
- `docs/FORMATO-v0.1.md` — especificacion del formato de descripcion de arquitectura.

## Compilar

```sh
cargo build --workspace
cargo test --workspace
```

## Licencia

Por definir antes de la primera publicacion. Ver `docs/BITACORA.md`, decision pendiente D-003.
