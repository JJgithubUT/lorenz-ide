# Lorenz OS

Sistema operativo minimo escrito en SGT para una maquina RISC de 16 bits con pantalla de
64x32 pixeles (0xE000) y teclado con interrupcion 1.

| Archivo | Papel |
|---|---|
| `maquina.toml` | La maquina: copia de `sgt-16` con pantalla y teclado |
| `pantalla.sgt` | Controlador de pantalla: pixeles, texto, desplazamiento |
| `teclado.sgt` | Controlador de teclado por interrupcion |
| `reloj.sgt` | Reloj por ticks |
| `planificador.sgt` | Planificador cooperativo de tareas periodicas, movido por el reloj |
| `arranque.sgt` | Vectores y arranque |
| `so.sgt` | Nucleo y bucle principal; importa los demas modulos |

Compilar: `lorenz compilar maquina.toml so.sgt -o so.hex`

Crear un proyecto propio a partir de este: `lorenz nuevo ejemplos/so-ejemplo mi-so --nombre mi-so`

## Tareas

`planificador.sgt` da de alta hasta cuatro tareas con `crear(cada)`; el reloj las marca cuando
toca y el bucle del nucleo las ejecuta con `siguiente()`. El nucleo trae dos: un latido que
parpadea el pixel de la esquina inferior derecha (cada 5 tics) y un contador de segundos (cada
50). La orden `TAREAS` muestra cuantas veces ha tocado cada una.
