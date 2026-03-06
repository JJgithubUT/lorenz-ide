# Lorenz OS

Sistema operativo minimo escrito en SGT para una maquina RISC de 16 bits con pantalla de
64x32 pixeles (0xE000) y teclado con interrupcion 1.

| Archivo | Papel |
|---|---|
| `maquina.toml` | La maquina: copia de `sgt-16` con pantalla y teclado |
| `pantalla.sgt` | Controlador de pantalla: pixeles, texto, desplazamiento |
| `teclado.sgt` | Controlador de teclado por interrupcion |
| `reloj.sgt` | Reloj por ticks |
| `arranque.sgt` | Vectores y arranque |
| `so.sgt` | Nucleo y bucle principal; importa los demas modulos |

Compilar: `lorenz compilar maquina.toml so.sgt -o so.hex`

Crear un proyecto propio a partir de este: `lorenz nuevo ejemplos/so-ejemplo mi-so --nombre mi-so`
