# Trazabilidad: requisitos del emulador

Estado de cada requisito funcional del emulador (`RF-EMU`) de la ERS frente a lo que hay en el
repositorio. Se actualiza al cerrar cada bloque de trabajo. Las tablas de otras areas se iran
agregando cuando empiece su desarrollo.

Estados: **Hecho** (implementado y con pruebas), **Parcial** (existe una parte, se dice cual),
**Pendiente**.

| Requisito | Resumen | Estado | Donde |
| --- | --- | --- | --- |
| RF-EMU-001 | Ejecutar un programa instruccion por instruccion | Hecho | `Emulador::paso`, `ejecutar` |
| RF-EMU-002 | Registros con su valor inicial | Hecho | `Estado::nuevo` |
| RF-EMU-003 | Empezar en la direccion de arranque | Hecho | `Estado::nuevo`, `reiniciar` |
| RF-EMU-004 | Decodificar segun la codificacion declarada | Hecho | `Decodificador` |
| RF-EMU-005 | Aplicar la semantica declarada | Hecho | `Interprete` |
| RF-EMU-006 | Truncar al ancho de palabra | Hecho | `Interprete::asignar` |
| RF-EMU-007 | Actualizar solo las banderas listadas | Hecho | `Interprete::actualizar_banderas` |
| RF-EMU-008 | Detenerse ante una instruccion de parada | Hecho | `detener` |
| RF-EMU-009 | Error ante una palabra que no es instruccion | Hecho | `ErrorEjecucion::InstruccionInvalida` |
| RF-EMU-010 | Ejecutar N instrucciones y devolver el control | Hecho | `Emulador::avanzar`; `ejecutar(limite)` corta con error |
| RF-EMU-011 | Frecuencia objetivo en hercios | Hecho | `fijar_frecuencia_hz`, `avanzar_tiempo(ns)`; el tiempo lo aporta el llamador |
| RF-EMU-012 | Maxima velocidad sin frecuencia | Hecho | `ejecutar` |
| RF-EMU-013 | Pausar y reanudar | Hecho | `pausar`, `reanudar`; el `paso` suelto sigue permitido |
| RF-EMU-014 | Reiniciar sin recargar el programa | Hecho | `Emulador::reiniciar` |
| RF-EMU-015 | Contar instrucciones y ciclos | Hecho | `Estado::contar_instruccion` |
| RF-EMU-016 | Resultados identicos con la misma entrada | Hecho | pruebas de determinismo |
| RF-EMU-017 | Estado completo como estructura consultable | Hecho | `Emulador::instantanea` y accesores de `Estado` |
| RF-EMU-018 | Guardar y restaurar el estado | Hecho | `Emulador::restaurar` |
| RF-EMU-019 | Memoria por regiones y permisos | Hecho | `Memoria` |
| RF-EMU-020 | Error al escribir en solo lectura | Hecho | `EscrituraEnSoloLectura` |
| RF-EMU-021 | Error fuera de toda region | Hecho | `FueraDeRegion` |
| RF-EMU-022 | Leer y escribir desde fuera del programa | Hecho | `leer_privilegiado`, `escribir_privilegiado` |
| RF-EMU-023 | Cargar un binario en una direccion | Hecho | `cargar_bloque`, `cargar_programa` |
| RF-EMU-024 | Volcar un rango a un archivo | Hecho | `--volcar` escribe en pantalla o en archivo, y se vuelve a cargar |
| RF-EMU-025 | Ultima instruccion que escribio cada direccion | Hecho | `Memoria::activar_seguimiento`, `ultima_escritura` |
| RF-EMU-026 | Marcar memoria no inicializada | Hecho | `esta_inicializada`, `lecturas_sin_inicializar`, `--vigilar` |
| RF-EMU-027 | Dispositivos por puertos | Hecho | `Puertos` |
| RF-EMU-028 | Dispositivos por region mapeada | Hecho | la pantalla lee su region; un dispositivo con script corre al escribir en la suya |
| RF-EMU-029 | Error en un puerto no declarado | Hecho | `PuertoNoDeclarado` |
| RF-EMU-030 | Pantalla con framebuffer configurable | Hecho | `pantalla::leer_imagen` (formatos `indice1`, `indice8`) |
| RF-EMU-031 | Teclado | Hecho | `Emulador::pulsar_tecla` |
| RF-EMU-032 | Consola serie | Hecho | `recibir_en_uart`, `texto_de_uart` |
| RF-EMU-033 | Temporizador programable | Hecho | `Temporizador`, periodo en ciclos |
| RF-EMU-034 | Almacenamiento por bloques | Hecho | dispositivo `bloques` con puertos `bloque`, `datos`, `estado`; `cargar_bloque`/`leer_bloque` |
| RF-EMU-035 | Dispositivos propios con script | Hecho | dispositivo `generico` con `script`; se analiza al cargar y corre al escribir en sus puertos |
| RF-EMU-036 | Prioridad de las lineas de interrupcion | Hecho | `prioridades` en la arquitectura; gana la menor, y a igual prioridad la de menor linea |
| RF-EMU-037 | Saltar al vector al aceptar una interrupcion | Hecho | `atender_interrupcion` |
| RF-EMU-038 | Respetar la habilitacion global | Hecho | `ie`, `EI`, `DI`, `RETI` |
| RF-EMU-039 | Registrar cada interrupcion atendida | Hecho | `interrupciones_atendidas`; la CLI las lista |
| RF-EMU-040 | Disparar una interrupcion a mano | Hecho | `levantar_interrupcion`, `--interrumpir LINEA@INSTRUCCION` |
| RF-EMU-041 | Entrada grabada para reproducir | Hecho | `--guion ARCHIVO` (teclas, serie e interrupciones por instruccion) |
| RF-EMU-042 | Ejecutar sin interfaz grafica | Hecho | `lorenz ejecutar` |
| RF-EMU-043 | Codigo de salida segun parada, error o limite | Hecho | 0, 1, 3 (2 es uso incorrecto) |
| RF-EMU-044 | Limite de instrucciones | Hecho | `--limite` |
| RF-EMU-045 | Traza legible | Hecho | `--traza` para personas, `--traza-maquina` en `clave=valor` |
| RF-EMU-046 | Comparar dos implementaciones | Hecho | `comparar` y `Ejecutor`; orden `lorenz comparar` (salida 4 si difieren) |

## Requisitos de arquitectura (RF-ARQ)

Estado a 2026-10-16: 30 hechos, 3 parciales y 9 pendientes de 42. Cada fila se comprobo contra el codigo de `lorenz-arch`.

| Requisito | Resumen | Estado | Donde |
| --- | --- | --- | --- |
| RF-ARQ-001 | Cargar la descripcion desde un archivo TOML | Hecho | `cargar`, `cargar_archivo` |
| RF-ARQ-002 | Rechazar una version mayor de formato desconocida | Hecho | `comprobar_version` |
| RF-ARQ-003 | Ancho de palabra entre 4 y 64 bits | Hecho | validacion de rangos (E-RANGO) |
| RF-ARQ-004 | Ancho de direccion entre 8 y 64 bits | Hecho | validacion de rangos |
| RF-ARQ-005 | Orden de bytes little o big endian | Hecho | `endianness` en `Maquina` |
| RF-ARQ-006 | Direccion de arranque | Hecho | `arranque` |
| RF-ARQ-007 | Registros con nombre, ancho y valor inicial | Hecho | `Registro` |
| RF-ARQ-008 | Roles de registro | Hecho | `RolRegistro` (pc, sp, banderas, general, indirecto, fijo) |
| RF-ARQ-009 | Exactamente un contador de programa | Hecho | validacion de roles unicos |
| RF-ARQ-010 | Pares de registros como un registro ancho | Pendiente |  |
| RF-ARQ-011 | Banderas con nombre, posicion y descripcion | Hecho | seccion `[flags]` |
| RF-ARQ-012 | Bancos de registros conmutables | Pendiente | fase 5 de la ERS |
| RF-ARQ-013 | Instrucciones con mnemonico, sintaxis, codificacion, semantica y ciclos | Hecho | `Instruccion`; la sintaxis se valida (`E-SINTAXIS`) |
| RF-ARQ-014 | Lenguaje de campos de bits | Hecho | `bits.rs` |
| RF-ARQ-015 | Instrucciones de varias palabras | Hecho | codificacion con `|` |
| RF-ARQ-016 | Los bits de cada palabra suman el ancho de palabra | Hecho | E-BITS |
| RF-ARQ-017 | Rechazar dos instrucciones con la misma codificacion | Hecho | E-AMBIG |
| RF-ARQ-018 | Cada campo tiene operando y viceversa | Hecho | E-CAMPO |
| RF-ARQ-019 | Operandos registro, inmediato, direccion, desplazamiento y puerto | Hecho | `TipoOperando` |
| RF-ARQ-020 | Rango y signo de los inmediatos | Hecho | `TipoOperando::Inmediato` |
| RF-ARQ-021 | Restringir los registros de un operando | Hecho | `de = [...]` |
| RF-ARQ-022 | Expresiones de semantica con asignacion, aritmetica, memoria, condicionales y E/S | Hecho | `semantica/`, E-SEMANT |
| RF-ARQ-023 | Banderas que modifica cada instruccion | Hecho | `afecta` |
| RF-ARQ-024 | Rechazar una bandera citada no declarada | Hecho | validacion de `afecta` |
| RF-ARQ-025 | Ciclos fijos o segun el resultado | Hecho | `Ciclos::Fijo`, `Ciclos::SegunSalto` |
| RF-ARQ-026 | Regiones de memoria con rango, nombre y permisos | Hecho | `Region` |
| RF-ARQ-027 | Rechazar regiones solapadas | Hecho | validacion de regiones |
| RF-ARQ-028 | Rechazar un arranque fuera de toda region | Hecho | validacion de regiones |
| RF-ARQ-029 | Precargar una region desde un binario | Hecho | `programa::precargar_regiones`; ruta relativa a la descripcion, mismo formato que un programa |
| RF-ARQ-030 | Dispositivos con tipo, puertos y region mapeada | Hecho | `Dispositivo`, `Region::dispositivo` |
| RF-ARQ-031 | Rechazar una region con dispositivo no declarado | Hecho | `dispositivos_citados` |
| RF-ARQ-032 | Lineas de interrupcion con vector y prioridad | Hecho | `Interrupciones` |
| RF-ARQ-033 | Bancos de memoria conmutables por puerto | Pendiente | fase 5 de la ERS |
| RF-ARQ-034 | Errores con linea y explicacion en terminos de la maquina | Parcial | las explicaciones si; los fallos de validacion aun no llevan linea |
| RF-ARQ-035 | Herencia entre descripciones | Pendiente |  |
| RF-ARQ-036 | Comentarios que llegan a la documentacion generada | Pendiente | hay `descripcion`, falta el generador de documentacion |
| RF-ARQ-037 | Descripcion cargada como estructura consultable | Hecho | `Maquina` |
| RF-ARQ-038 | Editor visual de arquitectura | Pendiente | fase del IDE |
| RF-ARQ-039 | Editor visual: texto a diagrama | Pendiente | fase del IDE |
| RF-ARQ-040 | Editor visual: diagrama a texto | Pendiente | fase del IDE |
| RF-ARQ-041 | Editor visual: errores de validacion en el diagrama | Pendiente | fase del IDE |
| RF-ARQ-042 | Al menos cuatro arquitecturas de ejemplo | Parcial | hay una de 8 bits (y su variante con UART); faltan 16 bits, RISC y de pila |

## Requisitos del ensamblador (RF-ENS)

Estado a 2026-10-16: 26 hechos y 4 pendientes de 30. El ensamblador y el desensamblador viven en
`lorenz-ens`; no conocen ninguna ISA, todo lo leen de la descripcion de la maquina.

| Requisito | Resumen | Estado | Donde |
| --- | --- | --- | --- |
| RF-ENS-001 | Ensamblador generado de la descripcion, sin configuracion | Hecho | `Ensamblador::nuevo` |
| RF-ENS-002 | Mnemonicos y sintaxis declarados | Hecho | `Plantilla` en `lorenz-arch` |
| RF-ENS-003 | Etiquetas en dos pasadas | Hecho | `primera_pasada`, `segunda_pasada` |
| RF-ENS-004 | Constantes con nombre | Hecho | `nombre = expr`, `.equ` |
| RF-ENS-005 | Expresiones aritmeticas constantes | Hecho | `expr.rs` |
| RF-ENS-006 | Literales decimal, hexadecimal, binario y caracter | Hecho | `expr.rs` |
| RF-ENS-007 | Directivas de espacio, datos y origen | Hecho | `.org`, `.db`, `.dw`, `.espacio` |
| RF-ENS-008 | Inclusion de archivos | Hecho | `.incluir`, `preprocesar.rs` |
| RF-ENS-009 | Macros con parametros | Hecho | `.macro`, `.endm` |
| RF-ENS-010 | Secciones ubicadas en regiones | Hecho | `.seccion` |
| RF-ENS-011 | Rechazar un inmediato fuera de rango | Hecho | `Causa::InmediatoFueraDeRango` |
| RF-ENS-012 | Rechazar un salto relativo fuera de alcance | Hecho | `Causa::SaltoFueraDeAlcance` |
| RF-ENS-013 | Rechazar codigo en una region que no corresponde | Hecho | `comprobar_region` (D-029) |
| RF-ENS-014 | Errores con archivo, linea, columna y causa en terminos de la ISA | Hecho | `ErrorEns` |
| RF-ENS-015 | Seguir tras un error recuperable | Hecho | acumula errores por pasada |
| RF-ENS-016 | Archivo objeto con codigo, simbolos y ubicacion | Hecho | `objeto.rs` |
| RF-ENS-017 | Mapa de direccion a archivo y linea | Hecho | `salidas::mapa` |
| RF-ENS-018 | Listado con direccion, bytes y fuente | Hecho | `salidas::listado` |
| RF-ENS-019 | Enlazador | Hecho | `enlazador::enlazar`, `lorenz enlazar`; objetos ya ubicados (con `.org`), sin reubicar codigo; externos solo en operandos de instrucciones |
| RF-ENS-020 | Simbolos no resueltos y duplicados al enlazar | Hecho | `ErrorEnlace::{NoResuelto, Duplicado, Solape}` |
| RF-ENS-021 | Desensamblador generado de la misma descripcion | Hecho | `Desensamblador` |
| RF-ENS-022 | Reconstruir el texto desde la codificacion | Hecho | `Plantilla::componer` |
| RF-ENS-023 | Marcar como datos lo que no es instruccion | Hecho | `Linea::Dato` |
| RF-ENS-024 | Etiquetas en lugar de direcciones con tabla de simbolos | Hecho | `--simbolos` |
| RF-ENS-025 | Ensamblar y desensamblar da texto equivalente | Hecho | prueba de ida y vuelta |
| RF-ENS-026 | Ensamblar desde la linea de comandos | Hecho | `lorenz ensamblar`, `desensamblar` |
| RF-ENS-027 | Palabras binarias literales | Hecho | `.db` |
| RF-ENS-028 | Aviso de banderas indeterminadas antes de un salto | Pendiente | |
| RF-ENS-029 | Exportar la tabla de simbolos | Hecho | `salidas::tabla_de_simbolos` (TOML) |
| RF-ENS-030 | Recargar sin reiniciar al cambiar la arquitectura | Pendiente | hace falta el IDE o un modo vigilante |

## Requisitos del depurador (RF-DEP)

Linea de comandos (001-018), traza, retroceso y reproduccion (019-031).

| Requisito | Descripcion | Estado | Donde |
|---|---|---|---|
| RF-DEP-001 | Depurador de linea de comandos | Hecho | `lorenz depurar` |
| RF-DEP-002 | Ejecutar una instruccion | Hecho | `Depurador::paso` |
| RF-DEP-003 | Ejecutar hasta el retorno de la subrutina | Hecho | `Depurador::salir` (por el SP); probado con CALL/RET anidados en `tests/salir.rs` |
| RF-DEP-004 | Ejecutar hasta una direccion | Hecho | `Depurador::hasta` |
| RF-DEP-005 | Mostrar todos los registros por nombre | Hecho | `vistas::registros` |
| RF-DEP-006 | Mostrar cada bandera por nombre y valor | Hecho | `vistas::registros` |
| RF-DEP-007 | Volcar memoria en hexadecimal y caracteres | Hecho | `vistas::volcar` |
| RF-DEP-008 | Modificar un registro en pausa | Hecho | orden `fijar` |
| RF-DEP-009 | Modificar una posicion de memoria en pausa | Hecho | orden `escribir` |
| RF-DEP-010 | Leer y escribir puertos a mano | Hecho | ordenes `puerto`, `entrada` |
| RF-DEP-011 | Ruptura por direccion | Hecho | `Depurador::poner_ruptura` |
| RF-DEP-012 | Ruptura por etiqueta o linea de fuente | Hecho | `Depurador::resolver` |
| RF-DEP-013 | Ruptura condicional | Hecho | `Condicion` |
| RF-DEP-014 | Observacion por escritura | Hecho | `Depurador::observar` |
| RF-DEP-015 | Observacion por lectura | Hecho | `Vigilar::Lectura` |
| RF-DEP-016 | Detenerse ante un acceso invalido e indicar la instruccion | Hecho | `Parada::Fallo` |
| RF-DEP-017 | Desensamblado junto al fuente | Hecho | `vistas::listar` |
| RF-DEP-018 | Pila cuando hay puntero de pila | Hecho | `vistas::pila` |
| RF-DEP-019 | Registrar una traza completa mientras la grabacion esta activa | Hecho | `Traza`, `PasoTraza` |
| RF-DEP-020 | Activar y desactivar la grabacion | Hecho | `Depurador::grabar`, `traza on/off` |
| RF-DEP-021 | Retroceder un paso | Hecho | `Depurador::atras` |
| RF-DEP-022 | Situarse en cualquier instante con un control deslizante | Parcial | `instante N` y `Depurador::ir_a`; el control grafico espera al IDE |
| RF-DEP-023 | Mostrar registros, memoria y pantalla de ese instante | Hecho | `ir_a` restaura la maquina; `registros`, `volcar`, `pantalla` |
| RF-DEP-024 | Quien escribio por ultima vez una direccion, y saltar | Hecho | `quien_escribio`, `Memoria::registrar_escrituras` |
| RF-DEP-025 | Quien modifico por ultima vez un registro o bandera | Hecho | `quien_cambio` |
| RF-DEP-026 | Buscar el primer instante en que se cumple una condicion | Hecho | `Depurador::buscar` |
| RF-DEP-027 | Limitar la memoria de la traza con instantaneas y reejecucion | Hecho | `Traza::adelgazar` (tope de 64 instantaneas) |
| RF-DEP-028 | Avisar al llegar al limite configurado | Hecho | `Depurador::tomar_avisos` |
| RF-DEP-029 | Exportar la traza | Hecho | `exportar` (CSV y JSON) |
| RF-DEP-030 | Guardar un fallo como archivo reproducible | Hecho | `reproducir::Fallo` (`lorenz-fallo-0.1`) |
| RF-DEP-031 | Reproducirlo identico en otra instalacion | Hecho | `Fallo::reproducir`; limite: puertos y dispositivos parten vacios |

## Requisitos de pruebas (RF-TST)

Pruebas declarativas (001-015), conformidad y cobertura (016-023), generacion aleatoria (024-026), regresion y compatibilidad (027-029), linea de comandos e integracion (030-036).

| Requisito | Descripcion | Estado | Donde |
|---|---|---|---|
| RF-TST-001 | Declarar pruebas en un proyecto | Hecho | `formato::Proyecto`, `lorenz-pruebas-0.1` |
| RF-TST-002 | Afirmar registros al terminar | Hecho | `EsperaDef.registros` |
| RF-TST-003 | Afirmar un rango de memoria | Hecho | `EsperaDef.memoria` |
| RF-TST-004 | Afirmar banderas | Hecho | `EsperaDef.banderas` |
| RF-TST-005 | Afirmar la salida de un dispositivo | Hecho | `EsperaDef.salida` |
| RF-TST-006 | Afirmar ciclos con tolerancia | Hecho | `CiclosDef` |
| RF-TST-007 | Entradas simuladas como pulsaciones | Hecho | `EntradaDef` |
| RF-TST-008 | Interrupciones en instantes dados | Hecho con limitaciones | `InterrupcionDef`; sin pulso de reloj como tal |
| RF-TST-009 | Estado inicial de registros y memoria | Hecho | `InicialDef` |
| RF-TST-010 | Limite de instrucciones | Hecho | `Prueba::limite` |
| RF-TST-011 | Ejecutar todas las pruebas | Hecho | `suite::ejecutar_suite`, `lorenz probar` |
| RF-TST-012 | Seleccionar por nombre o etiqueta | Hecho | `Opciones.nombre/etiqueta` |
| RF-TST-013 | Fallo con esperado, obtenido y lugar | Hecho | `Resultado.fallos` |
| RF-TST-014 | Abrir el depurador en el estado del fallo | Hecho | `lorenz probar --depurar` |
| RF-TST-015 | Pruebas en paralelo | Hecho | `Opciones.hilos` |
| RF-TST-016 | Suite de conformidad generada | Hecho | `conformidad::ejecutar_conformidad`, `lorenz conformidad` |
| RF-TST-017 | Efecto declarado de cada instruccion | Hecho | `Referencia` independiente |
| RF-TST-018 | Solo las banderas declaradas | Hecho | verificacion `BanderasAjenas` |
| RF-TST-019 | Codificar y decodificar son inversas | Hecho | verificacion `IdaYVuelta` |
| RF-TST-020 | Valores limite de operandos | Hecho | verificacion `Limites` |
| RF-TST-021 | Cobertura de instrucciones | Hecho | `cobertura::Cobertura`, `--cobertura` |
| RF-TST-022 | Cobertura de ramas | Hecho | `Cobertura` por (programa, direccion) |
| RF-TST-023 | Instrucciones sin ejercitar | Hecho | `InformeCobertura` |
| RF-TST-024 | Programas aleatorios validos | Hecho | `aleatorio::generar_programa`, `lorenz aleatorio` |
| RF-TST-025 | Comparar dos implementaciones | Hecho | `EjecutorReferencia` + `comparar` |
| RF-TST-026 | Reducir casos de fallo | Hecho | `aleatorio::reducir` (ddmin) |
| RF-TST-027 | Detectar programas mas lentos | Hecho | `regresion::Cambio::MasLenta`, codigo 6 |
| RF-TST-028 | Detectar programas rotos | Hecho | `regresion::Cambio::Rompe`, codigo 1 |
| RF-TST-029 | Avisar de binarios inutilizados por la ISA | Hecho | `compatibilidad`, `lorenz compatibilidad` |
| RF-TST-030 | CLI que compile, ejecute y pruebe | Hecho | `lorenz-cli` |
| RF-TST-031 | Codigos de salida distinguibles | Hecho | `suite::SALIDA_*` |
| RF-TST-032 | Resultados legibles por maquina | Hecho | `--json`, `lorenz-resultados-0.1` |
| RF-TST-033 | Sin entorno grafico | Hecho | `lorenz-cli` sin dependencias graficas |
| RF-TST-034 | Ejemplo de integracion continua | Hecho | `docs/CI.md`, `ejemplos/ci/` |
| RF-TST-035 | Semilla fijable | Hecho | `semilla` del proyecto, `--semilla` |
| RF-TST-036 | Conservar la ultima ejecucion | Hecho | `.lorenz/ultimo.json`, `--historial` |

## Concordancia con los commits anteriores

Los commits entre el 59 y el 126 citaron, en dispositivos, interrupciones y linea de comandos,
identificadores provisionales. Equivalencias para leer ese historial:

| Citado entonces | Requisito real |
| --- | --- |
| `RF-EMU-032`, `033`, `034` en interrupciones | `RF-EMU-036`, `037`, `038` |
| `RF-EMU-035`, `036` en teclado y pantalla | `RF-EMU-031`, `030` |
| `RF-EMU-037`, `038` en el temporizador | `RF-EMU-033` |
| `RF-EMU-039` en la UART | `RF-EMU-032` |
| `RF-EMU-016`, `017`, `026` en el bucle de ejecucion | `RF-EMU-001`, `044` |
| `RF-CLI-*` | `RF-EMU-042` a `045` |

Los commits 243 a 251 (depurador de linea de comandos) citaron identificadores RF-DEP equivocados en varios casos: por
ejemplo el 247 cita RF-DEP-007, 010, 017 y 018 para el interprete de ordenes. Los correctos son los
de la tabla de arriba: volcar es 007, fijar 008, escribir 009, puertos 010 (commit 251),
desensamblado con fuente 017 y pila 018.
