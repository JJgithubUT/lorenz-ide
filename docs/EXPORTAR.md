# Exportar

`lorenz exportar <formato> <maquina.toml> [programa] -o SALIDA`

| Formato | Salida | Para que |
| --- | --- | --- |
| `pdf` | `ARCHIVO.pdf` | La documentacion de la arquitectura (la de `documentar`) en papel. Escritor de PDF propio: A4, Helvetica y Courier, paginas numeradas. |
| `verilog` | carpeta con `<maquina>_cpu.v`, `<maquina>_tb.v` y `programa.mem` | Un modelo de simulacion de la maquina y un banco de pruebas que corre el programa y escribe los registros. |
| `logisim` | `ARCHIVO.circ` | Bloques de Logisim (registros, ROM con el programa, RAM, reloj) y la tabla de instrucciones como notas. |

## Verilog

El modelo ejecuta una instruccion por ciclo de reloj con la misma semantica que el emulador: el
`pc` que ve la semantica ya apunta a la instruccion siguiente, y las banderas de `afecta` siguen
las convenciones Z, N y C. Se comprueba contra el emulador con `iverilog` (los registros finales
tienen que coincidir; la prueba se omite si `iverilog` no esta instalado).

```
lorenz exportar verilog maquina.toml programa.s -o salida/
cd salida && iverilog -o sim *_tb.v *_cpu.v && vvp sim
```

No es un diseno sintetizable (asignaciones bloqueantes, memoria como arreglo). No modela
interrupciones, dispositivos, `in[]`/`out[]` ni la proteccion de las regiones de solo lectura, y
lo dice con un aviso. Las maquinas con pares o bancos de registros se rechazan.

## Logisim

Los bloques **no vienen cableados**: es un punto de partida con anchuras y contenido de memoria
ya puestos, para construir la ruta de datos a mano.
