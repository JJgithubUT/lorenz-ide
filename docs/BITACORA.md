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
- Pruebas: 23 de interrupciones, 5 de `ie` en el interprete, 2 de arranque del estado, 2 de
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
