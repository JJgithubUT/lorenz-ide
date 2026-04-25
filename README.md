# Lorenz

Entorno de diseno y simulacion de computadoras. El usuario describe la arquitectura de una maquina
en un archivo declarativo, y a partir de esa descripcion se generan el ensamblador, el emulador,
el depurador y la documentacion de la ISA.

Incluye **SGT (Saggitarius)**, un lenguaje de nivel medio que conoce la maquina para la que compila.

## Estado

Version 0.3.0 publicada; la unidad grafica (0.4.0) esta en desarrollo. Pendiente: el entorno grafico
(IDE) y lo del ecosistema (paquetes, version web, exportaciones).

| Crate | Que hace | Estado |
| --- | --- | --- |
| `lorenz-arch` | Formato de descripcion y validacion, herencia, documentacion de la ISA | hecho |
| `lorenz-emu` | Emulador funcional: memoria, puertos, interrupciones, DMA, dispositivos | hecho |
| `lorenz-ens` | Ensamblador, desensamblador y enlazador generados desde la descripcion | hecho |
| `lorenz-dbg` | Depurador de linea de comandos, historia, trazas, viaje en el tiempo | hecho |
| `lorenz-tst` | Pruebas de programas, conformidad de la ISA, cobertura, fuzzing | hecho |
| `lorenz-sgt` | Compilador del lenguaje SGT | hecho |
| `lorenz-cic` | Simulacion ciclo a ciclo: pipeline, cache, varios nucleos, restricciones | hecho |
| `lorenz-gpu` | Unidad grafica: framebuffer, comandos, hilos en warps | en desarrollo (0.4.0) |
| `lorenz-cli` | La orden `lorenz` | hecho |

## Documentacion del proyecto

- `docs/BITACORA.md` — registro de lo que se va haciendo, sesion por sesion.
- `docs/SPRINTS.md` — plan y cierre de cada sprint.
- `docs/TRAZABILIDAD.md` — estado de cada requisito de la ERS.
- `docs/GIT-WORKFLOW.md` — ramas, convencion de commits y politica de granularidad.
- `docs/ARQUITECTURA.md` — como se organiza el codigo del propio Lorenz.
- `docs/FORMATO.md` — especificacion del formato de descripcion de arquitectura.
- `docs/SEMANTICA.md`, `docs/SGT.md`, `docs/CICLOS.md`, `docs/GPU.md` — el lenguaje de semantica, el
  lenguaje SGT, la simulacion de ciclos y la unidad grafica.

## Compilar

```sh
cargo build --workspace
cargo test --workspace
```

## Licencia

Por definir antes de la primera publicacion. Ver `docs/BITACORA.md`, decision pendiente D-003.
