# Registro de cambios

## 0.3.0 — 2026-10-16 — Cierre de la fase 6 sin IDE ni GPU

Completa lo opcional que quedaba en el nucleo, el modelo de ciclos y el lenguaje. Lo unico que sigue
pendiente de la ERS es el entorno grafico (IDE) y la unidad grafica (GPU).

### Varios nucleos (fase 6)

- `[nucleos]`: de 1 a 64 nucleos sobre memoria compartida, cada uno con su pipeline y su cache.
  Avanza el que menos ciclos lleva, de modo que el orden de las escrituras es el del tiempo
  simulado y el resultado se repite. Con `[bus] compartido` los nucleos se turnan en un bus comun.
- `[coherencia]`: caches coherentes con MSI. Se cuentan invalidaciones y entregas de lineas, y el
  nucleo que actua espera lo que cuestan (nueva causa de espera, `coherencia`).
- `lorenz nucleos maquina.toml programa...`.

### Restricciones y variantes de diseno (fase 6)

- `[[restriccion]]`: limites sobre ciclos, instrucciones, CPI, esperas, fallos de cache, uso del bus
  y tamano del codigo.
- `lorenz restricciones` comprueba un programa contra ellas; `lorenz variantes` prueba variantes de
  adelantamiento, prediccion, cache y bus, y estima las instrucciones nuevas que las cumplirian.
  Codigo de salida 5 cuando algo no se cumple.

### Lenguaje SGT

- Genericos sencillos sobre enteros: `funcion maximo<T>(a: T, b: T) -> T` y `maximo<u16>(1, 2)`,
  resueltos por monomorfizacion antes del analisis de tipos.

Detalle en `docs/CICLOS.md`, `docs/SGT.md`, `docs/FORMATO.md` y `docs/BITACORA.md`; el plan del
sprint, en `docs/SPRINTS.md`.

## 0.2.0 — 2026-10-16 — Fases 2 a 5 (sin IDE ni GPU)

Cubre herramientas alrededor del emulador, el lenguaje SGT y la simulacion ciclo a ciclo.
Los requisitos del entorno grafico (IDE) y de la unidad grafica (GPU) siguen pendientes.

### Herramientas (fase 2)

- Ensamblador, desensamblador y enlazador generados desde la descripcion de la maquina, con
  macros, inclusiones y avisos de banderas indeterminadas.
- Depurador de linea de comandos con rupturas, observaciones, traza, retroceso a cualquier
  instante, busqueda de quien escribio o cambio un valor, inyeccion de fallos (`inyectar`) y
  exploracion de ordenes de eventos (`explorar`).
- Pruebas como parte del proyecto: `probar`, conformidad, pruebas aleatorias, regresion y
  compatibilidad.
- Fallos reproducibles, documentacion generada de la arquitectura, `lorenz nuevo`.
- Bancos de registros (`[[banco]]`) y de memoria (`bancos` en `[[region]]`).

### Lenguaje SGT (fases 3 y 4)

- Compilador a cualquier maquina descrita: optimizacion de 0 a 3, costes en ciclos por funcion,
  biblioteca, enteros anchos, maquinas de 8 bits, dispositivos, interrupciones, modulos y
  disposicion fijada de estructuras (`@ n`).
- Ejemplo de sistema operativo en varios modulos (Lorenz OS).

### Ciclos, cache y bus (fase 5 y parte de la 6)

- `lorenz ciclos`: pipeline configurable con riesgos de datos y de control, predictores, cache,
  latencias por region, vista y diagrama ciclo a ciclo, perfil, mapa de calor, historico por
  version de la arquitectura y comparacion entre descripciones.
- Controlador DMA (`tipo = "dma"`) y bus compartido con contencion (`[bus]`).
- `lorenz sugerir`: instrucciones nuevas que acortarian los programas.

Detalle en `docs/CICLOS.md`, `docs/FORMATO.md` y `docs/BITACORA.md`.

## 0.1.0 — 2026-10-16 — Fase 1: formato de arquitectura y emulador

Primera version utilizable. Cubre los 46 requisitos RF-EMU de la ERS y la parte de RF-ARQ que el
emulador necesita para leer una maquina.

### Arquitectura (`lorenz-arch`)

- Descripcion de la maquina en TOML: registros, banderas, instrucciones con codificacion de bits,
  regiones de memoria con permisos, dispositivos e interrupciones.
- Validacion con reglas numeradas y mensajes que citan el requisito.
- La `semantica` de cada instruccion se analiza una sola vez a un arbol (D-009).
- Dispositivos: pantalla, teclado, UART, temporizador, almacenamiento por bloques y genericos
  con script.

### Emulador (`lorenz-emu`)

- Ejecucion instruccion por instruccion, por lotes o hasta detenerse, con limite de instrucciones.
- Pausa, reanudacion y avance por tiempo emulado a una frecuencia en hercios.
- Memoria por regiones con permisos, acceso privilegiado, seguimiento de la ultima escritura y
  aviso de lecturas sin inicializar.
- Puertos, interrupciones con prioridad y habilitacion global, dispositivos mapeados en memoria.
- Instantaneas del estado y ejecuciones repetibles.
- Comparacion de dos implementaciones con la misma entrada.

### Linea de comandos (`lorenz-cli`)

- `lorenz validar`, `lorenz ejecutar` (traza legible o de maquina, pantalla, volcados, teclas,
  serie, interrupciones a mano, guiones de entrada) y `lorenz comparar`.
- Codigos de salida: 0 detenida, 1 fallo, 2 uso, 3 limite, 4 las maquinas difieren.

### Pendiente para la fase 2

- Ensamblador y desensamblador generados desde la arquitectura.
- Depurador de linea de comandos y pruebas como parte del proyecto.
