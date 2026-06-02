# Sprints

Cada sprint tiene un objetivo, unas historias con su criterio de hecho y un orden. Al cerrarlo se
anota aqui lo que quedo y lo que no, y la bitacora recoge las decisiones.

## Sprint 1 — Cierre de la fase 6 (sin IDE ni GPU)

**Fecha:** 2026-10-16. **Version que lo cierra:** 0.3.0.

**Objetivo:** que no quede ningun requisito opcional de la fase 6 sin hacer en el nucleo, el modelo
de ciclos y el lenguaje, de modo que lo unico pendiente sea el entorno grafico y la unidad grafica.

| Orden | Historia | Requisito | Criterio de hecho |
| --- | --- | --- | --- |
| 1 | Restricciones de diseno y variantes que las cumplan | RF-CIC-033 | Una descripcion declara limites (ciclos, palabras de codigo, uso del bus); `lorenz restricciones` los comprueba contra programas y `lorenz variantes` propone cambios de temporizacion que los cumplen. Con pruebas y documentado. |
| 2 | Varios nucleos sobre memoria compartida | RF-CIC-023 | `nucleos` en la descripcion de ciclos; cada nucleo ejecuta su programa, comparten la memoria y turnan el bus; el resultado es repetible. Metricas por nucleo. |
| 3 | Coherencia de cache entre nucleos | RF-CIC-024 | Una cache por nucleo con protocolo MSI; las escrituras invalidan las copias ajenas; se cuentan invalidaciones y transferencias. Un programa que comparte un dato da el mismo resultado con y sin cache. |
| 4 | Genericos sencillos sobre enteros | RF-SGT-021 | Una funcion puede declarar un parametro de tipo entero y se instancia por cada tipo con que se llama; el compilador lo rechaza con un mensaje claro cuando no hay tipo que cuadre. |
| 5 | Cierre | todos | Pruebas del workspace y clippy sin avisos, informe y respaldo al dia, version 0.3.0 en `main` con su etiqueta. |

**Fuera del sprint:** editor visual, depurador grafico, IDE y GPU (fases de interfaz).

**Reglas de trabajo:** las de `docs/GIT-WORKFLOW.md`. Una rama por historia desde `develop`,
fusion `--no-ff`, commits pequenos que compilan solos, cada historia deja su fila de
`docs/TRAZABILIDAD.md`, su entrada de bitacora y su seccion de documentacion.

**Riesgos:**

- La simulacion de varios nucleos obliga a que las memorias de los nucleos coincidan; se resuelve
  difundiendo cada escritura a los demas en el orden en que ocurre, lo que fija un modelo de memoria
  sencillo (consistencia secuencial).
- Los genericos tocan el analizador, el generador y el interprete de referencia; se limitan a
  funciones y a parametros de tipo entero.

## Registro de cierre

**Resultado:** las cinco historias se hicieron completas.

| Historia | Requisito | Estado | Decisiones |
| --- | --- | --- | --- |
| Restricciones y variantes | RF-CIC-033 | Hecha | D-089 a D-091 |
| Varios nucleos | RF-CIC-023 | Hecha | D-092 a D-094 |
| Coherencia de cache | RF-CIC-024 | Hecha | D-095 a D-097 |
| Genericos de SGT | RF-SGT-021 | Hecha | D-098 a D-100 |
| Cierre | todos | Hecha: version 0.3.0 | |

**Lo que sale del sprint:**

- Pruebas del workspace: 1907, todas verdes; clippy sin avisos.
- Cuatro requisitos de la ERS pasan a Hecho. Quedan solo los del IDE, los de la GPU y el control
  grafico de DEP-022.
- Un fallo propio encontrado y corregido: las pruebas de las ordenes nuevas escribian el mismo
  archivo temporal a la vez y a veces leian a medias; ahora se escribe una sola vez.

**Para el siguiente sprint:** el entorno grafico y la unidad grafica, o ampliar lo hecho (inferencia
de tipos en los genericos, MESI, propagar el DMA entre nucleos).

## Sprint 2 — Unidad gráfica

**Fecha:** 2026-10-16. **Versión que lo cierra:** 0.4.0.

**Objetivo:** que la unidad gráfica exista y se pueda usar sin entorno gráfico: dibujar, recibir
comandos de la CPU, ejecutar hilos en warps y compararse con la CPU.

| Orden | Historia | Requisitos | Criterio de hecho |
| --- | --- | --- | --- |
| 1 | Superficie y primitivas | RF-GPU-002, 005 a 009 | Framebuffer configurable; relleno, líneas, triángulos planos e interpolados, profundidad. |
| 2 | Volcado e inspección | RF-GPU-004, 022 | PNG y PPM ampliados; descripción de un pixel. |
| 3 | Comandos, cola y ciclos | RF-GPU-010 a 012, 017 | Comandos con codificación y guion; cola con profundidad; ciclos propios. |
| 4 | Acople a la CPU | RF-GPU-001, 010, 012, 021 | Puertos, estado e interrupción; sin `[gpu]` no hay costo. |
| 5 | Hilos en warps | RF-GPU-013 a 016 | Kernels, divergencia visible, memoria compartida, barreras. |
| 6 | Herramientas | RF-GPU-018 a 020 | Depurador por hilo, comparación CPU↔GPU, tres ejemplos. |
| 7 | Cierre | todos | Pruebas y clippy, documentación, versión 0.4.0 en `main`. |

**Fuera del sprint:** visor en tiempo real (RF-GPU-003) y todo el IDE.

**Riesgos:** unir el modo gráfico y el de hilos en una sola memoria (se aplaza, D-108); modelar la
reconvergencia con precisión de hardware (se simplifica, D-102).

### Registro de cierre del sprint 2

**Resultado:** las siete historias se hicieron; solo queda fuera el visor en tiempo real, que estaba
fuera del sprint.

| Historia | Requisitos | Estado | Decisiones |
| --- | --- | --- | --- |
| Superficie y primitivas | RF-GPU-002, 005 a 009 | Hecha | D-105 |
| Volcado e inspeccion | RF-GPU-004, 022 | Hecha | D-106 |
| Comandos, cola y ciclos | RF-GPU-010 a 012, 017 | Hecha | D-101 |
| Acople a la CPU | RF-GPU-001, 010, 012, 021 | Hecha | D-103, D-104 |
| Hilos en warps | RF-GPU-013 a 016 | Hecha | D-102, D-108 |
| Herramientas | RF-GPU-018 a 020 | Hecha | D-107 |
| Cierre | todos | Hecha: version 0.4.0 | |

**Lo que sale del sprint:**

- Pruebas del workspace: 1979, todas verdes; clippy sin avisos.
- 21 de los 22 requisitos RF-GPU pasan a Hecho; queda RF-GPU-003 (visor en tiempo real).
- Un fallo propio encontrado y corregido: la carrera de archivos temporales de las pruebas de la CLI
  volvio a aparecer en `nucleos`; ahora se escriben de forma atomica.

**Para el siguiente sprint:** las fases del ecosistema que no necesitan interfaz (paquetes,
exportaciones, importar maquinas existentes, ejercicios guiados) o el IDE.

## Sprint 3 — Paquetes y exportaciones

**Fecha:** 2026-10-16. **Versión que lo cierra:** 0.5.0.

**Objetivo:** que una arquitectura se pueda empaquetar, verificar, instalar con sus dependencias y
documentar en HTML, todo sin interfaz.

| Orden | Historia | Requisitos | Criterio de hecho |
| --- | --- | --- | --- |
| 1 | Versiones y suma | RF-ECO-006, 008 | SHA-256 y requisitos de versión con pruebas. |
| 2 | Formato de paquete | RF-ECO-001, 002, 007, 008 | Manifiesto, `.lpkg` determinista, rutas seguras, alteración detectada. |
| 3 | Almacén y dependencias | RF-ECO-003, 005 | Instalación por versión, resolución, ciclos y faltantes. |
| 4 | Orden `paquete` | RF-ECO-001 a 003, 005 | `crear`, `verificar`, `info`, `instalar`, `listar`. |
| 5 | Exportar a HTML | RF-ECO-025 | `documentar -o x.html`. |
| 6 | Cierre | todos | Pruebas y clippy, documentación, versión 0.5.0 en `main`. |

**Fuera del sprint:** instalación desde la web (RF-ECO-004), PDF, ejercicios, desafíos, importadores
y exportaciones a otros formatos; todo el IDE.

**Riesgos:** el formato de paquete queda fijado desde aquí (cabecera con versión `1` para poder
evolucionarlo).

### Registro de cierre del sprint 3

**Resultado:** las seis historias se hicieron.

| Historia | Requisitos | Estado | Decisiones |
| --- | --- | --- | --- |
| Versiones y suma | RF-ECO-006, 008 | Hecha | D-111 |
| Formato de paquete | RF-ECO-001, 002, 007, 008 | Hecha | D-109 |
| Almacen y dependencias | RF-ECO-003, 005 | Hecha | D-110 |
| Orden `paquete` | RF-ECO-001 a 003, 005 | Hecha | |
| Exportar a HTML | RF-ECO-025 | Hecha | D-112 |
| Cierre | todos | Hecha: version 0.5.0 | |

**Lo que sale del sprint:**

- Pruebas del workspace: 2004, todas verdes; clippy sin avisos.
- Un fallo propio encontrado y corregido: las pruebas del almacen compartian carpetas temporales y
  fallaban de vez en cuando; ahora cada carpeta lleva un contador unico.

**Para el siguiente sprint:** exportar a PDF (RF-ECO-024), ejercicios guiados, desafios, importar
JDH-8 y Astro-8, o el IDE.


## Sprint 4 — De 306 a 378 requisitos (en curso)

**Fecha prevista:** 2026-10-17 en adelante. **Versiones que lo cierran:** 0.6.0, 0.7.0, 0.8.0 y 0.9.0,
una por hito. **Objetivo:** llevar el proyecto de 306 a **378 de 380 requisitos terminados (99,5 %)**,
incluidos el IDE, el depurador gráfico, el editor visual, el visor de la GPU y la versión web.
Estimación: unos 480 commits (de 611 a unos 1090), con lo que se supera el mínimo de 1000.

**Decisión de base (D-113): la interfaz es una aplicación web local.** `lorenz ide` levanta un
servidor HTTP (solo biblioteca estándar) que ofrece una API sobre el núcleo y una página única
(HTML, CSS y JS sin dependencias) embebida en el binario; se abre en el navegador. Motivos: se puede
probar de verdad aquí con el Chromium instalado (capturas y pruebas de interfaz), reutiliza el
núcleo nativo sin compilar a WebAssembly, y el mismo servidor sirve la versión «web», los enlaces
compartidos y la galería. Se descartan las bibliotecas de ventanas nativas porque aquí no se podrían
ver ni probar.

**Estado de partida:** 306 terminados, 73 faltantes, 1 por verificar (RF-DEP-022).

### Hito A — Núcleo sin interfaz → v0.6.0 (306 → 329, 86,6 %) — **terminado**

| # | Historia | Requisitos | Criterio de hecho | Commits |
| --- | --- | --- | --- | --- |
| A1 | Proyecto con configuración y tareas | RF-IDE-001 a 006 | `proyecto.toml` con arquitecturas (una activa), fuentes en orden, dispositivos y tareas con nombre; `lorenz proyecto`, `lorenz tarea`. | 18 |
| A2 | Plantillas y recientes | RF-IDE-007 a 009 | Plantillas de programa simple, SO mínimo y SGT; lista de recientes. | 8 |
| A3 | Ejercicios guiados y desafíos | RF-ECO-016, 014 | `ejercicio.toml` con enunciado y verificación; 12 ejercicios; desafíos con métricas y ranking. | 30 |
| A4 | Importadores | RF-ECO-015 | JDH-8 y Astro-8 a TOML con prueba de ejecución. | 18 |
| A5 | Planificador en Lorenz OS | RF-ECO-041 | Tareas por interrupción de reloj. | 10 |
| A6 | Exportaciones | RF-ECO-024, 034 a 036 | PDF con escritor propio; Verilog y banco de pruebas; `.circ` básico. | 42 |
| A7 | Periféricos externos y modo seguro | RF-ECO-037, 038, 009 | Dispositivo `externo` por proceso o TCP, protocolo en `docs/EXTERNOS.md`, modo que prohíbe tocar el disco. | 22 |
| A8 | Explicar trazas y marcar lo generado | RF-ECO-044, 046 | `lorenz explicar` determinista; marca en todo texto generado. | 14 |
| A9 | Instalar desde dirección web | RF-ECO-004 | Cliente HTTP(S) propio probado contra servidor local. | 10 |

### Hito B — Servidor y API → parte de v0.7.0 (sin requisitos propios)

| # | Historia | Criterio de hecho | Commits |
| --- | --- | --- | --- |
| B1 | Servidor HTTP y JSON | Servidor con rutas, límites y cierre ordenado; JSON mínimo con pruebas. | 10 |
| B2 | API del núcleo | Proyecto, archivos, ensamblar, compilar, ejecutar, paso a paso, registros, memoria, trazas, puntos de parada, GPU; sesiones independientes. | 24 |
| B3 | Banco de pruebas de interfaz | Chromium sin cabeza dentro de las pruebas: abrir, pulsar, leer el DOM y capturar. | 6 |

### Hito C — IDE → v0.7.0 (329 → 358, 94,2 %)

| # | Historia | Requisitos | Commits |
| --- | --- | --- | --- |
| C1 | Estructura: paneles acoplables, disposición recordada, atajos reasignables | RF-IDE-017 a 020 | 16 |
| C2 | Editor: resaltado de ensamblador y SGT, autocompletado, codificación bajo el cursor, errores en línea, ir a definición, buscar y reemplazar | RF-IDE-010 a 016 | 36 |
| C3 | Instrumentos: registros como lámparas, consola de teletipo, pantalla con efecto de monitor y sus interruptores | RF-IDE-021 a 024 | 20 |
| C4 | Apariencia: tema sobrio, marcas de diseño, temas propios | RF-IDE-025 a 027 | 14 |
| C5 | Sonido: relé, teclado y campanilla, silencio por familia, todo funciona sin sonido | RF-IDE-028 a 030 | 8 |
| C6 | Accesibilidad: tamaño de texto, alto contraste, depurar solo con teclado, no depender del color | RF-IDE-031 a 034 | 16 |
| C7 | Robustez: progreso en operaciones largas, autoguardado antes de compilar, recuperación tras cierre, documentación de la arquitectura dentro del programa | RF-IDE-035 a 038 | 18 |

### Hito D — Depuración gráfica y visor de la GPU → v0.8.0 (358 → 366)

| # | Historia | Requisitos | Commits |
| --- | --- | --- | --- |
| D1 | Control deslizante sobre la traza (cierra RF-DEP-022) y ejecución lenta con animación | RF-DEP-022, 034 | 10 |
| D2 | Animación del dato entre memoria, registros y unidad aritmética | RF-DEP-033 | 12 |
| D3 | Mapa de memoria gráfico, editor hexadecimal en vivo, línea de tiempo de interrupciones y E/S | RF-DEP-036 a 038 | 20 |
| D4 | Paridad con el depurador de línea de comandos (cada orden con su equivalente gráfico, comprobado por una prueba que las compara) | RF-DEP-044 | 8 |
| D5 | Visor del framebuffer en tiempo real | RF-GPU-003 | 8 |

### Hito E — Editor visual de arquitectura → v0.8.0 (366 → 370)

| # | Historia | Requisitos | Commits |
| --- | --- | --- | --- |
| E1 | Lectura y escritura del TOML que conserva comentarios y formato (decisión D-114: dependencia `toml_edit` o analizador propio) | base de RF-ARQ-040 | 16 |
| E2 | Diagrama de bloques que se compone arrastrando | RF-ARQ-038 | 14 |
| E3 | Sincronía texto ↔ diagrama en ambos sentidos | RF-ARQ-039, 040 | 12 |
| E4 | Errores del cargador señalados en el diagrama | RF-ARQ-041 | 6 |

### Hito F — Versión web, compartir y galería → v0.9.0 (370 → 378, 99,5 %)

| # | Historia | Requisitos | Commits |
| --- | --- | --- | --- |
| F1 | Modo `lorenz servir`: IDE alojado, sin instalar nada; editar, ensamblar y ejecutar | RF-ECO-030 | 10 |
| F2 | Trabajo conservado entre sesiones y exportación del proyecto a `.lpkg` para el escritorio | RF-ECO-031, 032 | 10 |
| F3 | Aviso claro de las funciones no disponibles frente al escritorio | RF-ECO-033 | 4 |
| F4 | Enlaces compartidos con identificador por contenido; ejecución en el navegador | RF-ECO-010, 011 | 14 |
| F5 | Derivar una copia propia | RF-ECO-012 | 6 |
| F6 | Galería de máquinas publicadas con búsqueda | RF-ECO-013 | 12 |
| F7 | Cierre final: pruebas, clippy, informe, respaldo, tag | todos | 10 |

### Qué queda fuera y por qué (2 de 380)

- **RF-ECO-045** (borrador de arquitectura desde lenguaje natural): exige un modelo de IA y
  RF-ECO-043 obliga a que todo funcione sin ella. Se deja pendiente.
- **RF-ECO-029** (núcleo como módulo de navegador): este entorno no puede instalar el objetivo
  `wasm32`. Se prepara el núcleo y un guion de compilación, y el requisito queda «por verificar»
  hasta compilarlo en tu equipo. Los demás requisitos web se cumplen con el servidor.

### Riesgos

- **Tamaño del IDE:** C2 y E3 son las historias más inciertas (editor con resaltado propio y
  sincronía bidireccional). Si una crece más del doble, se parte y se entrega la versión mínima que
  cumpla el requisito.
- **Servidor público:** `servir` guarda programas de terceros. Se ejecutan con los límites de
  ciclos y memoria del emulador, sin acceso a disco ni red (A7) y con cuotas por sesión.
- **Pruebas de interfaz:** dependen del Chromium del entorno; si no está, esas pruebas se omiten
  con aviso en vez de fallar.
- **Importadores:** se basan en la documentación pública de JDH-8 y Astro-8; si es ambigua, la
  descripción importada lleva avisos.
- **HTTPS en A9:** dependencia TLS o solo HTTP con SHA-256 publicado (D-115, pendiente).

### Línea de corte si el tiempo aprieta

Se recorta de abajo hacia arriba: F6, F5, E, D2, A6 (Logisim), A9. Los hitos A, B y C no se tocan:
sin ellos no hay IDE.

### Definición de terminado

Cada historia entra por una rama `feature/*`, con commits pequeños que compilan, pruebas propias,
fila de trazabilidad y entrada en la bitácora. Cada hito termina con todas las pruebas en verde,
clippy sin avisos, informe y respaldo renovados y su tag.
