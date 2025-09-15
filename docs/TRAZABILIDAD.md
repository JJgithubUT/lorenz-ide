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
| RF-ARQ-013 | Instrucciones con mnemonico, sintaxis, codificacion, semantica y ciclos | Hecho | `Instruccion` |
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
| RF-ARQ-029 | Precargar una region desde un binario | Parcial | se declara `contenido`, pero nadie lo carga todavia |
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
