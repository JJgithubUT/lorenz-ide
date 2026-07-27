# Registro de cambios

## 0.8.1 — 2026-10-16 — Editor visual de arquitectura

- Panel «Editor visual de arquitectura»: la descripcion como diagrama de bloques (registros,
  banderas, memoria, dispositivos), con paleta que se arrastra o se pulsa, bloques movibles y
  formulario por bloque. Se maneja tambien con el teclado.
- El texto y el diagrama se mantienen en sincronia en los dos sentidos. Lo que se cambia en el
  diagrama se escribe en el texto con `toml_edit`: los comentarios, el orden y el formato quedan.
- Los errores del cargador se senalan en el bloque afectado (marca ⚠ y descripcion, no solo color)
  y llevan a su linea. «Deshacer» revierte el ultimo cambio hecho desde el diagrama.

## 0.8.0 — 2026-10-16 — Depuracion grafica avanzada

- Control deslizante sobre la traza y velocidad de ejecucion (la lenta respeta las rupturas).
- Camino del dato animado, mapa de memoria, editor hexadecimal en vivo y linea de tiempo de
  interrupciones, E/S y paradas.
- Panel de ordenes con todas las del depurador de linea de comandos; una prueba comprueba que cada
  orden tiene su equivalente en el IDE (`ide::paridad`).
- Visor de la unidad grafica en tiempo real (`/api/gpu`, panel Unidad grafica).

## 0.7.0 — 2026-10-16 — El IDE

`lorenz ide [carpeta]` abre una interfaz web local (solo `127.0.0.1`) sobre el proyecto. No hay
dependencias nuevas: el servidor, el JSON y la interfaz son del proyecto. Ver `docs/IDE.md`.

- Editor con resaltado de ensamblador (segun la arquitectura activa) y de SGT, autocompletado,
  codificacion en bits bajo el cursor, errores sobre la linea, ir a la definicion, buscar y
  reemplazar en el proyecto y documentacion de la arquitectura dentro del IDE.
- Paneles acoplables, separables y reordenables con la disposicion recordada; atajos reasignables.
- Depurador grafico: registros como lamparas, desensamblado con rupturas, memoria, paso atras,
  consola de teletipo y pantalla con efecto de monitor (cada efecto se apaga por separado).
- Temas por marcas (incluido el sobrio, sin efectos) y temas propios; sonido por familia, apagado
  por defecto; tamano de texto, alto contraste, depuracion solo con teclado y estados que no
  dependen del color.
- Guardado antes de compilar, recuperacion tras un cierre inesperado y progreso de operaciones
  largas.

## 0.6.0 — 2026-10-16 — El nucleo sin interfaz

Primer hito del sprint 4: 23 requisitos que no necesitan interfaz grafica.

### Proyectos y ejercicios

- `proyecto.toml` con arquitecturas, fuentes, dispositivos y tareas; `lorenz proyecto` y
  `lorenz tarea`. Plantillas de proyecto (`lorenz plantillas`) y lista de recientes. Ver
  `docs/PROYECTOS.md`.
- `lorenz ejercicio` (listar, ver con pistas, verificar) y doce ejercicios para `risc-16`;
  `lorenz desafio` con metricas (ciclos, instrucciones, palabras) y ranking. Ver `docs/EJERCICIOS.md`.
- `lorenz explicar`: la ejecucion contada instruccion a instruccion, sin modelos de lenguaje.

### Ecosistema

- `lorenz importar`: arquitecturas desde una tabla de instrucciones (JDH-8 y Astro-8 incluidas).
- `lorenz exportar pdf|verilog|logisim`. El Verilog se comprueba contra el emulador con `iverilog`.
  Ver `docs/EXPORTAR.md`.
- Dispositivos externos (`tipo = "externo"`) por proceso o TCP y modo seguro (`--seguro`). Ver
  `docs/EXTERNOS.md`.
- `lorenz paquete instalar URL --sha256 HUELLA`: instalacion desde la red sin TLS, con huella
  obligatoria.
- Lorenz OS gana un planificador de tareas movido por el reloj (`TAREAS`).
- Los textos generados llevan la marca "Generado por Lorenz".

## 0.5.0 — 2026-10-16 — Paquetes

Anade el sistema de paquetes y la exportacion de la documentacion a HTML. Todo funciona desde la
linea de comandos; la distribucion por red (RF-ECO-004) queda para mas adelante.

### Paquetes

- Crate `lorenz-paq`: un paquete es un solo archivo `.lpkg` con manifiesto (`nombre`, `version`,
  `autor`, `licencia`, `tipo`, dependencias), suma SHA-256 por archivo y suma total. El mismo
  contenido da siempre los mismos bytes.
- Versiones semanticas y requisitos (`=`, `>=`, `>`, `<=`, `<`, `^`, `*`).
- Rutas seguras y limites de tamano; un archivo alterado o truncado se rechaza.
- Almacen local `nombre/version/` con resolucion de dependencias desde carpetas de repositorio
  (version mas alta que cumple), deteccion de ciclos y faltantes, e instalacion atomica.
- `lorenz paquete crear|verificar|info|instalar|listar`. Ver `docs/PAQUETES.md`.

### Documentacion

- `lorenz documentar -o ARCHIVO.html` exporta la documentacion de la ISA como pagina autonoma.

## 0.4.0 — 2026-10-16 — Unidad grafica

Anade la unidad grafica (GPU) al nucleo, sin entorno grafico: lo unico que queda pendiente de ella
es el visor del framebuffer en tiempo real (RF-GPU-003), que es del IDE.

### Modo grafico

- `[gpu]`: framebuffer de 1x1 a 4096x4096 con color de 8, 16 o 24 bits y buffer de profundidad
  opcional; cola de comandos con profundidad configurable; costes en ciclos propios.
- Relleno, lineas, triangulos de color plano e interpolado y prueba de profundidad.
- La CPU manda comandos por un puerto, lee el estado por otro y recibe una interrupcion al terminar
  un lote. Los ciclos de la CPU y de la GPU se cuentan por separado.
- `lorenz gpu`: ejecuta un programa o un guion `.gpu`, da metricas y vuelca el framebuffer a PNG o
  PPM ampliado; describe pixeles (`--pixel`) y dibuja con caracteres (`--texto`).

### Modo de hilos

- `[gpu.hilos]`: hilos agrupados en warps con divergencia visible, unidades de ejecucion,
  latencias de memoria, memoria compartida por grupo y barreras. Lenguaje propio de kernels.
- `lorenz kernel` (con `--traza` que marca que hilos trabajan en cada instruccion de warp),
  `lorenz depurar-kernel` (paso a paso, hilo por hilo) y `lorenz comparar-gpu` (el mismo problema en
  la CPU y en la GPU, con resultados y ciclos).
- Maquina de ejemplo `ejemplos/gpu-16` con tres kernels y un guion grafico.

### Otros cambios

- Un dispositivo `generico` sin script registra lo que el programa escribe en sus puertos, en vez de
  descartarlo.
- Las pruebas de la CLI escriben sus archivos temporales de forma atomica.

Detalle en `docs/GPU.md`, `docs/FORMATO.md` y `docs/BITACORA.md`; el plan del sprint, en
`docs/SPRINTS.md`.

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
