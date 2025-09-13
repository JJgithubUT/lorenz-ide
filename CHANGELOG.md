# Registro de cambios

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
