# Trazabilidad: requisitos del emulador

Estado de cada requisito funcional del emulador (`RF-EMU`) de la ERS frente a lo que hay en el
repositorio. Se actualiza al cerrar cada bloque de trabajo. Los requisitos de otras areas se iran
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
| RF-EMU-011 | Frecuencia objetivo en hercios | Pendiente | |
| RF-EMU-012 | Maxima velocidad sin frecuencia | Hecho | `ejecutar` |
| RF-EMU-013 | Pausar y reanudar | Pendiente | |
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
| RF-EMU-028 | Dispositivos por region mapeada | Parcial | solo la pantalla |
| RF-EMU-029 | Error en un puerto no declarado | Hecho | `PuertoNoDeclarado` |
| RF-EMU-030 | Pantalla con framebuffer configurable | Hecho | `pantalla::leer_imagen` (formatos `indice1`, `indice8`) |
| RF-EMU-031 | Teclado | Hecho | `Emulador::pulsar_tecla` |
| RF-EMU-032 | Consola serie | Hecho | `recibir_en_uart`, `texto_de_uart` |
| RF-EMU-033 | Temporizador programable | Hecho | `Temporizador`, periodo en ciclos |
| RF-EMU-034 | Almacenamiento por bloques | Pendiente | |
| RF-EMU-035 | Dispositivos propios con script | Pendiente | |
| RF-EMU-036 | Prioridad de las lineas de interrupcion | Parcial | gana la de menor numero; falta prioridad declarada |
| RF-EMU-037 | Saltar al vector al aceptar una interrupcion | Hecho | `atender_interrupcion` |
| RF-EMU-038 | Respetar la habilitacion global | Hecho | `ie`, `EI`, `DI`, `RETI` |
| RF-EMU-039 | Registrar cada interrupcion atendida | Hecho | `interrupciones_atendidas`; la CLI las lista |
| RF-EMU-040 | Disparar una interrupcion a mano | Hecho | `levantar_interrupcion`, `--interrumpir LINEA@INSTRUCCION` |
| RF-EMU-041 | Entrada grabada para reproducir | Pendiente | |
| RF-EMU-042 | Ejecutar sin interfaz grafica | Hecho | `lorenz ejecutar` |
| RF-EMU-043 | Codigo de salida segun parada, error o limite | Hecho | 0, 1, 3 (2 es uso incorrecto) |
| RF-EMU-044 | Limite de instrucciones | Hecho | `--limite` |
| RF-EMU-045 | Traza legible | Parcial | `--traza`; falta formato maquina |
| RF-EMU-046 | Comparar dos implementaciones | Pendiente | |

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
