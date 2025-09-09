# Bitacora de desarrollo

Registro de lo que se va haciendo en Lorenz. Se escribe en orden inverso: lo mas reciente arriba.
Cada entrada corresponde a una sesion de trabajo y cita los requisitos de la ERS atendidos.

Formato de una entrada:

```
## AAAA-MM-DD — titulo corto

**Hecho:** que quedo funcionando.
**Decidido:** decisiones tomadas, con su identificador D-NNN.
**Pendiente:** lo que queda abierto y por que.
**Requisitos:** los RF y RNF que este trabajo atiende.
**Commits:** rango o rama.
```

---

## 2026-10-16 — Dispositivos con script

**Hecho:**

- Un dispositivo `generico` lleva un `script` que se analiza al cargar y se valida (nombres
  permitidos, sin `mem`/`in`/`out`/`detener`) (RF-EMU-035, RF-ARQ-022).
- El emulador lo ejecuta cada vez que el programa escribe en uno de sus puertos; recuerda su
  estado entre ejecuciones y puede levantar su interrupcion con `interrumpir`.
- El calculo de operadores binarios se extrajo del interprete para compartirlo.

**Decidido:** D-027 — el script reutiliza el lenguaje de semantica en lugar de uno nuevo, con un
entorno propio que no toca memoria ni puertos de la maquina.
**Pendiente:** RF-EMU-046 y la parte no grafica de RF-EMU-028.
**Requisitos:** RF-EMU-035.
**Commits:** feat/dispositivo-script (#175-#180).

---

## 2026-10-16 — Almacenamiento por bloques

**Hecho:**

- La arquitectura declara `cantidad_bloques` y rechaza tamano o cantidad cero.
- El emulador implementa el disco con puertos `bloque`, `datos` y `estado` (ver SEMANTICA.md);
  `cargar_bloque` y `leer_bloque` permiten poner y sacar imagenes desde fuera (RF-EMU-034).
- El contenido del disco sobrevive a `reiniciar` y entra en las instantaneas.

**Decidido:** D-026 — un acceso erroneo al disco enciende el bit de error y no escribe nada.
**Pendiente:** RF-EMU-035, 046 y la parte no grafica de 028.
**Requisitos:** RF-EMU-034.
**Commits:** feat/arch-cantidad-bloques (#170-#174). Contador: 174.

---

## 2026-10-16 — Traza en formato de maquina

**Hecho:** `lorenz ejecutar --traza-maquina` escribe `pc=0x0000 op=LDI ciclos=2 salto=0` por
instruccion (RF-EMU-045). Base para comparar dos implementaciones (RF-EMU-046).
**Pendiente:** RF-EMU-034, 035, 046 y la parte no grafica de 028.
**Requisitos:** RF-EMU-045.
**Commits:** feat/cli-traza-maquina (#165-#168). Contador: 168.

---

## 2026-10-16 — Pausa y frecuencia objetivo

**Hecho:**

- `pausar`/`reanudar`: `avanzar`, `ejecutar` y `avanzar_tiempo` se niegan en pausa; `paso` no (RF-EMU-013).
- `fijar_frecuencia_hz` y `avanzar_tiempo(ns)` ejecutan los ciclos que caben en el tiempo dado,
  arrastrando el resto al siguiente intervalo (RF-EMU-011).

**Decidido:** D-025 — el emulador no mide tiempo real ni duerme; el tiempo entra como dato, asi
las pruebas son deterministas. El ritmo real lo pondra el CLI o el depurador.
**Pendiente:** RF-EMU-034, 035, 045 (formato maquina), 046 y la parte no grafica de 028.
**Requisitos:** RF-EMU-011, RF-EMU-013.
**Commits:** feat/emu-pausa (#161-#164). Contador: 164.

---

## 2026-10-16 — Guion de entrada y prioridades de interrupcion

**Hecho:**

- `--guion` reproduce eventos de entrada (teclas, texto por serie, interrupciones) en la
  instruccion indicada (RF-EMU-041).
- La arquitectura declara `prioridades` por linea y el emulador atiende primero la mejor
  (RF-EMU-036); sin declarar, gana la linea de menor numero.

**Decidido:** la prioridad es numerica y menor significa mas urgente (D-019 se mantiene).
**Pendiente:** RF-EMU-011, 013, 034, 035, 045, 046 y la parte no grafica de 028.
**Requisitos:** RF-EMU-036, RF-EMU-041.
**Commits:** feat/emu-guion (#150-#159).

---

## 2026-10-16 — Seguimiento de la memoria

**Hecho:**

- `Memoria::activar_seguimiento` anota quien escribio cada celda por ultima vez (la carga o la
  instruccion en tal direccion) y cuales celdas nadie escribio (RF-EMU-025, RF-EMU-026). Solo se
  reserva la memoria extra cuando se activa.
- El interprete lee con `leer_vigilada`, que anota las lecturas de celdas sin escribir con la
  direccion de la instruccion que las hizo (hasta 4096).
- `lorenz ejecutar --vigilar` activa el seguimiento antes de cargar el programa y avisa de las
  lecturas sin inicializar al terminar.
- Pruebas: 23 de seguimiento, 2 de argumentos y 4 de la CLI.

**Decidido:**

- **D-024** — Al activar el seguimiento con memoria ya cargada, lo que vale distinto de cero se da
  por cargado y lo que vale cero por no escrito. Motivo: no se puede saber mas; por eso la CLI lo
  activa antes de cargar el programa.

**Requisitos:** RF-EMU-025, RF-EMU-026.

**Commits:** contador en $N al cerrar el bloque.

---

## 2026-10-16 — Control de la ejecucion, instantaneas y volcado

**Hecho:**

- `Emulador::avanzar(n)` ejecuta como mucho n instrucciones y devuelve el control; llegar a n no es
  un error (RF-EMU-010).
- `Emulador::instantanea` y `restaurar` guardan y recuperan registros, banderas, contadores, memoria,
  puertos, temporizadores e interrupciones; restaurar sobre otra maquina falla sin cambiar nada
  (RF-EMU-017, RF-EMU-018).
- Registro de interrupciones atendidas con instante, linea, direccion interrumpida y manejador, con
  tope de 65536 entradas y un contador de las que no caben (RF-EMU-039).
- CLI: `--volcar DESDE:CANTIDAD[:ARCHIVO]` (el archivo se puede volver a cargar como programa) y
  `--interrumpir LINEA@INSTRUCCION`; el resumen lista las interrupciones atendidas.
- Programa de ejemplo `interrupcion.hex`.
- Pruebas: 6 de `avanzar`, 5 del registro de interrupciones, 10 de instantaneas, 6 de escritura de
  palabras, 9 + 8 de argumentos nuevos y 13 de la CLI.

**Pendiente:** ver `docs/TRAZABILIDAD.md`: frecuencia en hercios y pausa (RF-EMU-011, 013), ultima
escritura y memoria no inicializada (025, 026), dispositivo de bloques (034), dispositivos con
script (035), prioridad declarada de las lineas (036), entrada grabada (041), traza en formato de
maquina (045) y comparacion de dos implementaciones (046).

**Requisitos:** RF-EMU-010, RF-EMU-017, RF-EMU-018, RF-EMU-024, RF-EMU-039, RF-EMU-040.

**Commits:** contador en 141 al cerrar el bloque.

---

## 2026-10-16 — UART y segundo ejemplo

**Hecho:**

- `Emulador::recibir_en_uart` y `texto_de_uart`; el puerto de estado de una UART tiene el bit 0 si
  hay un byte recibido y el bit 1 siempre encendido.
- `lorenz ejecutar` imprime lo que cada UART transmitio y gana `--serie TEXTO`.
- Nuevo ejemplo `ejemplos/ejemplo-8-serie` (ejemplo-8 mas `IN`, `OUT` y una UART) con los programas
  `hola.hex` y `eco.hex`.
- Pruebas: 14 de UART, 6 de la CLI y 2 de argumentos.

**Corregido:** los identificadores de requisito que cite en los commits de dispositivos,
interrupciones y linea de comandos (desde la rama `feat/emu-estado` hasta aqui) usaban una numeracion
provisional que no coincide con la ERS: no existen `RF-CLI-*`, y las interrupciones son
RF-EMU-036 a 040, la pantalla es RF-EMU-030, el teclado 031, la consola serie 032 y el temporizador
033. No se reescribe el historial (D-004); la tabla correcta esta en `docs/TRAZABILIDAD.md` y desde
este bloque los commits citan la numeracion de la ERS.

**Decidido:**

- **D-023** — Cada commit cita requisitos que existen en la ERS. Motivo: la trazabilidad solo sirve
  si el identificador lleva a un requisito real; `docs/TRAZABILIDAD.md` lleva la cuenta de cuales
  estan hechos.

**Requisitos:** RF-EMU-032, RF-EMU-042.

**Commits:** contador en 126 al cerrar el bloque.

---

## 2026-10-16 — Temporizador

**Hecho:**

- El formato acepta `periodo` (en ciclos, 1000 por omision) en los dispositivos de tipo
  `temporizador`; el validador rechaza el periodo 0.
- Despues de cada instruccion el emulador suma sus ciclos a cada temporizador; cada periodo completo
  cuenta un tick, lo publica en el puerto `cuenta` y levanta la linea de interrupcion si la declara.
  El resto de ciclos no se pierde y una instruccion larga puede completar varios periodos.
- `Puertos::publicar` deja un valor que se lee sin consumirse; `Emulador::ticks_de(nombre)`.
- Pruebas: 14 del temporizador, 4 de puertos publicados, 8 de declaracion de dispositivos.

**Decidido:**

- **D-022** — El periodo del temporizador se mide en ciclos de la maquina, no en tiempo real. Motivo:
  la ejecucion debe ser determinista y repetible; el tiempo real depende de la velocidad del equipo.

**Pendiente:** salida de texto de la UART, dispositivos de bloques, y revisar los requisitos RF-EMU
contra lo implementado.

**Requisitos:** RF-EMU-037, RF-EMU-038, RF-ARQ-030, RF-ARQ-031.

**Commits:** contador en 120 al cerrar el bloque.

---

## 2026-10-16 — Pantalla y teclado

**Hecho:**

- `Emulador::pantalla(nombre)` convierte la region de video en una `Imagen`: formato `indice1` (un
  bit por pixel, el bit mas significativo primero) e `indice8` (una palabra por pixel). Lo que la
  region no cubre se dibuja apagado. `Imagen::a_texto` la dibuja con `#` y `.`.
- `Emulador::pulsar_tecla(nombre, codigo)` deja la tecla en el puerto de datos y levanta la linea de
  interrupcion del teclado si la declara.
- El puerto de estado de un teclado vale 1 si hay una tecla esperando y no consume nada.
- `lorenz ejecutar` gana `--pantalla` (dibuja las pantallas al terminar) y `--teclas TEXTO`.
- Programa de ejemplo `pantalla.hex`.
- Pruebas: 12 de pantalla, 10 de teclado (una con un manejador real que lee la tecla por
  interrupcion), 3 nuevas de puertos y 6 nuevas de la CLI.
- El asistente de commits ejecuta rustfmt antes de cada commit, para que no se acumulen commits de
  solo formato.

**Decidido:**

- **D-021** — El puerto `estado` de un teclado se deduce del tipo de dispositivo: no hay que
  declararlo en la semantica. Motivo: es lo que toda maquina con teclado necesita y evita que cada
  arquitectura reinvente el mismo protocolo.

**Pendiente:** temporizador, UART y dispositivos de bloques (tipos ya aceptados por el formato pero
sin comportamiento); cerrar la fase 1 con una revision de requisitos RF-EMU contra lo implementado.

**Requisitos:** RF-EMU-035, RF-EMU-036, RF-CLI-006.

**Commits:** contador en 114 al cerrar el bloque.

---

## 2026-10-16 — Interrupciones del emulador

**Hecho:**

- `Emulador::levantar_interrupcion(linea)` deja la linea pendiente; se atiende antes de la siguiente
  instruccion si las interrupciones estan habilitadas. Gana la linea de menor numero.
- Atender una interrupcion apila el pc (sentido de pila y `endianness` de la maquina), inhabilita las
  interrupciones y salta a la direccion guardada en el vector.
- Alias `ie` en la semantica: se lee como 1 o 0 y asignarlo habilita o inhabilita. Solo existe si la
  maquina declara interrupciones; el validador lo comprueba.
- El estado arranca y reinicia con `habilitadas_al_arrancar`.
- El ejemplo de 8 bits gana `RETI`, `EI` y `DI` (11 instrucciones). El codigo 0x80 pasa a ser el
  de prueba para instrucciones invalidas.
- Pruebas: 24 de interrupciones, 5 de `ie` en el interprete, 2 de arranque del estado, 2 de
  validacion.

**Decidido:**

- **D-019** — El vector de una linea es la direccion de memoria donde la maquina guarda la direccion
  del manejador (tantas palabras como pida una direccion, en su `endianness`). Motivo: en una
  maquina de 8 bits una direccion de 16 bits no cabe en una palabra y la tabla de vectores es lo que
  hacen las maquinas reales de ese tipo. Las reglas completas estan en `docs/SEMANTICA.md`.
- **D-020** — Una maquina detenida no atiende interrupciones: `HLT` es definitivo hasta un
  `reiniciar`. Motivo: mantiene la parada predecible para los programas de prueba.

**Pendiente:** dispositivos de pantalla y teclado conectados a las regiones y los puertos
(el teclado levanta su linea al recibir una tecla).

**Requisitos:** RF-EMU-032 a RF-EMU-034, RF-ARQ-022.

**Commits:** contador en 103 al cerrar el bloque.

---

## 2026-10-16 — Linea de comandos: validar y ejecutar

**Hecho:**

- Nuevo crate `lorenz-cli` con el binario `lorenz`; la logica vive en la biblioteca para probarla
  sin lanzar procesos.
- `lorenz validar <maquina.toml>` y `lorenz ejecutar <maquina.toml> <programa.hex>` con
  `--limite`, `--direccion` y `--traza`.
- Formato de programa en texto: palabras hexadecimales o binarias, comentarios con `#`.
- Codigos de salida: 0 detenida, 1 fallo de ejecucion, 2 uso o archivos invalidos, 3 limite.
- Programas de ejemplo `suma.hex` y `bucle.hex` en `ejemplos/ejemplo-8/programas`.
- Pruebas: 15 de lectura de programas, 16 de argumentos, 20 de ordenes y 4 del binario como proceso.
- Se aplico rustfmt a todo el codigo en un solo commit.

**Decidido:**

- **D-018** — La CLI usa solo la biblioteca estandar para leer argumentos. Motivo: pocas ordenes,
  cero dependencias nuevas y mensajes de error en espanol sin adaptar un tercero.

**Pendiente:** interrupciones (el formato no define aun como se vuelve de una; hace falta decidir
la semantica de `vector` y un nombre para habilitarlas desde la semantica), dispositivos de
pantalla y teclado conectados a las regiones y los puertos, y la salida de traza en formato maquina.

**Requisitos:** RF-CLI-001 a RF-CLI-006, RF-EMU-042, RF-EMU-043.

**Commits:** contador en 89 al cerrar el bloque.

---

## 2026-10-16 — Emulador: decodificador, interprete y bucle de ejecucion

**Hecho:**

- `Decodificador`: tabla armada con la mascara y el valor fijo de la primera palabra de cada
  codificacion; comprueba tambien los bits fijos de las palabras siguientes; junta los campos
  repartidos segun el endianness; extiende el signo de inmediatos con signo y desplazamientos.
- `Interprete`: evalua el arbol de semantica con enteros de 128 bits, trunca al asignar, cortocircuita
  `&&` y `||`, protege los desplazamientos enormes y devuelve error en divisiones entre cero.
- Banderas Z, N y C calculadas con el ultimo valor asignado (D-013); las explicitas mandan.
- `Emulador`: `paso` y `ejecutar(limite)`; los ciclos de un salto condicional dependen de si se
  toma; si una instruccion falla, el pc vuelve a ella.
- Pruebas nuevas: 14 de decodificador, 40 de interprete, 18 de banderas y 20 de ejecucion completa
  con programas reales del ejemplo de 8 bits.

**Decidido:**

- **D-016** — Al ejecutar, `pc` en la semantica vale la direccion de la instruccion siguiente y
  asignarlo es un salto. Motivo: asi un desplazamiento relativo se escribe `pc = pc + off` igual que
  en las arquitecturas reales.
- **D-017** — Si una instruccion falla, el emulador deja el pc en esa instruccion, pero los efectos
  ya hechos antes del fallo dentro de la misma instruccion no se deshacen. Motivo: el depurador debe
  mostrar donde fallo; deshacer exigiria transacciones que no aportan en esta fase.

**Pendiente:** interrupciones (vectores y habilitacion), salida de traza y codigos de salida
(RF-EMU-042 a 045), y el paso de la fase 1 a la interfaz de linea de comandos.

**Requisitos:** RF-EMU-006, RF-EMU-007, RF-EMU-016 a RF-EMU-026.

**Commits:** contador en 75 al cerrar el bloque.

---

## 2026-10-16 — Emulador: memoria, estado y puertos

**Hecho:**

- Nuevo crate `lorenz-emu` con el error de ejecucion `ErrorEjecucion`.
- `Memoria`: una palabra por direccion, regiones ordenadas con busqueda binaria, permisos de lectura y
  escritura, acceso privilegiado para depurador y cargador, carga de bloques todo o nada y volcado.
- `Estado`: registros por nombre y por rol, truncado al ancho de cada registro, banderas como bits del
  registro de banderas, contadores de instrucciones y ciclos, parada, interrupciones y `reiniciar`.
- `Puertos`: cola de entrada y registro de salida por cada puerto que declara un dispositivo; tocar otro
  da error.
- Pruebas: 28 de memoria, 16 de estado y 8 de puertos.

**Pendiente:** decodificador, interprete del arbol y bucle de ejecucion (bloques 3 a 5).

**Requisitos:** RF-EMU-005, RF-EMU-007, RF-EMU-014, RF-EMU-016, RF-EMU-030, RF-EMU-031.

**Commits:** contador en 63 al cerrar el bloque (objetivo del dia 1: 160; seguimos recuperando con
commits pequenos).

---

## 2026-10-16 — Analizador del lenguaje de semantica

**Hecho:**

- Arbol de sintaxis de la semantica: expresiones, destinos, sentencias y programa, con impresion que
  se vuelve a analizar igual.
- Analizador lexico: numeros decimales, hexadecimales y binarios con separador `_`, identificadores,
  palabras reservadas, operadores y puntuacion.
- Parser de descenso recursivo con nueve niveles de precedencia, sentencias, condicionales con y sin
  llaves y la secuencia con `;`.
- El cargador analiza la semantica al cargar y guarda el arbol en cada instruccion. Un error de
  sintaxis se acumula como fallo `E-SEMANT` en lugar de detener la carga.
- Validacion de nombres: lo que la semantica cita debe ser un operando, un registro, una bandera o
  los alias `pc` y `sp`; solo se asigna a lo asignable.
- Campos repartidos en varias palabras de la instruccion (`dir:8 | dir:8`).
- `docs/SEMANTICA.md` con la gramatica, la tabla de precedencia y la convencion de banderas.
- 64 pruebas del parser, 26 del lexico, 25 de validacion de semantica y 7 nuevas de campos
  repartidos.

**Decidido:**

- **D-009** — El emulador interpreta un arbol analizado una sola vez al cargar, no el texto en cada
  instruccion. La arquitectura sigue siendo editable sin recompilar.
- **D-013** — Las banderas `Z`, `N` y `C` se calculan solas desde el ultimo valor asignado por la
  instruccion; cualquier otra bandera se asigna de forma explicita. Una asignacion explicita manda
  sobre la convencion.
- **D-014** — Un campo de la codificacion puede repartirse en palabras distintas y sus fragmentos se
  concatenan en un operando (primer fragmento menos significativo en little endian). Motivo: una
  direccion de 16 bits no cabe en una instruccion de palabras de 8 bits. Esto corrige un ejemplo del
  documento de formato v0.1, que mostraba `dir:16` dentro de una palabra de 8 bits y el validador
  habria rechazado.
- **D-015** — La precedencia de operadores sigue la de Rust: la comparacion liga menos que las
  operaciones de bits.

**Pendiente:**

- Corregir el documento de formato v0.1 con la convencion de campos repartidos.
- Posiciones de linea del TOML en los fallos de validacion.

**Requisitos:** RF-ARQ-014, 015, 018, 022, 034, 037, 042.

**Commits:** rama `feat/semantica-parser`, 28 commits.

**Estado del contador:** 50 de 1000 commits.

---

## 2026-10-15 — Cadencia de 1000 commits en una semana

**Hecho:**

- `docs/GIT-WORKFLOW.md` gana la seccion 1.1: metas diarias y por hora, calendario de siete dias
  con acumulado al cierre de cada uno, las unidades que justifican un commit propio y lo que no se
  hace para subir el contador.
- `scripts/contador.sh` compara los commits de una rama contra el calendario.
- Programado el arranque del desarrollo para las 1:30 del 2026-10-16, hora de Ciudad de Mexico.

**Decidido:**

- **D-010** — La meta de 1000 commits se cumple en una semana de desarrollo, del 2026-10-16 al
  2026-10-22: 140 commits por dia como minimo, uno cada diez minutos de trabajo. (D-009 queda
  reservado para la decision sobre como se interpreta el campo `semantica`.)
- **D-011** — El calendario fija el ritmo, no el alcance. Si el trabajo de un dia queda corto, el
  alcance pasa al dia siguiente; el contador no se recupera inflandolo.
- **D-012** — Quedan prohibidos los commits vacios, los cambios de formato sin motivo y reescribir
  historial publicado para modificar el contador.

**Pendiente:**

- Revisar el reparto por fase de la seccion 1: sumaba 1050 pensando en un ritmo mas lento y no
  coincide con el calendario de la semana.

**Requisitos:** ninguno; proceso.

**Commits:** rama `docs/cadencia-mil-commits`.

**Estado del contador:** 20 de 1000 commits.

---

## 2026-10-15 — Nucleo de carga y validacion de arquitectura

**Hecho:**

- Workspace de cargo con el crate `lorenz-arch`, raiz del grafo de dependencias.
- Analizador del lenguaje de campos de bits: bits fijos, campos con nombre, relleno y separador de
  palabra, con mascara y valor fijo para comparar codificaciones entre si.
- Modelo en memoria de una maquina: registros con rol, banderas, instrucciones, operandos, costes
  en ciclos, regiones y dispositivos.
- Cargador de TOML en dos capas: estructuras crudas que reflejan el archivo y conversion al modelo
  validado.
- Validador con las diez reglas del formato. Acumula todos los fallos antes de rechazar.
- Maquina de ejemplo de 8 bits en `ejemplos/ejemplo-8/maquina.toml`, que carga y valida limpia.
- 74 pruebas en verde: 26 del analizador de bits, 21 de carga, 26 de validacion y 1 de documentacion.

**Decidido:**

- **D-005** — Los errores de validacion se acumulan y se devuelven juntos. Ver diez problemas de una
  vez ahorra diez ciclos de prueba y error.
- **D-006** — Cada regla de validacion lleva un codigo corto (`E-BITS`, `E-AMBIG`) y el requisito de
  la ERS que la exige, para que la salida sea rastreable al documento.
- **D-007** — Una codificacion invalida detiene la carga de inmediato, a diferencia de los fallos de
  validacion: sin codificacion no hay nada que validar.
- **D-008** — Una version mayor distinta del formato se rechaza; una menor distinta se acepta.

**Pendiente:**

- El analizador de la expresion de semantica: hoy se guarda como texto sin interpretar.
- Posiciones de linea en los fallos de validacion; el tipo `Posicion` ya existe pero el cargador
  todavia no la rellena.
- Herencia entre descripciones (RF-ARQ-035).

**Requisitos:** RF-ARQ-001 a 009, 011, 013 a 019, 024 a 032, 034, 037, 042.

**Commits:** rama `feat/workspace-skeleton`, 11 commits, fusionada a `develop`.

**Estado del contador:** 16 de 1000 commits.

## 2026-10-15 — Arranque del repositorio

**Hecho:**

- Repositorio inicializado con las tres ramas permanentes: `main`, `develop` y `deploy`.
- Documentado el flujo de trabajo en `docs/GIT-WORKFLOW.md`, incluida la politica de granularidad
  que fija un minimo de 1000 commits para el proyecto.
- Creada esta bitacora.

**Decidido:**

- **D-001** — El nucleo se escribe en Rust. Motivo: un solo binario para linea de comandos y
  WebAssembly, y buen ecosistema para analizadores.
- **D-002** — La descripcion de arquitectura se escribe en TOML, con un lenguaje de campos de bits
  propio solo para la codificacion de instrucciones.
- **D-003** — La licencia del proyecto queda pendiente hasta antes de la primera publicacion.
- **D-004** — El historial no se aplasta al fusionar. El detalle del historial es un objetivo del
  proyecto, no ruido.

**Pendiente:**

- Elegir la licencia (D-003).
- Decidir si el editor visual de arquitectura escribe el TOML directamente o pasa por una
  representacion intermedia.

**Requisitos:** ninguno todavia; trabajo de infraestructura.

**Commits:** rama `main`, commits iniciales.
