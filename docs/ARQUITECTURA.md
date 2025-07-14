# Arquitectura del codigo de Lorenz

No confundir con la arquitectura de las maquinas que el usuario describe: este documento trata de
como se organiza el codigo del propio producto.

## Principio

El nucleo no sabe que existe una interfaz grafica. Todo lo que hace Lorenz debe poder hacerse
desde la linea de comandos sobre la misma biblioteca (RNF-POR-004, RNF-MAN-001).

## Crates del workspace

| Crate | Responsabilidad | Depende de |
| --- | --- | --- |
| `lorenz-arch` | Leer, validar y exponer la descripcion de arquitectura | — |
| `lorenz-emu` | Ejecutar programas sobre una maquina descrita | `lorenz-arch` |
| `lorenz-asm` | Ensamblar y desensamblar segun la descripcion | `lorenz-arch` |
| `lorenz-dbg` | Trazas, puntos de ruptura, linea de tiempo | `lorenz-emu` |
| `lorenz-cli` | Interfaz de linea de comandos | todos los anteriores |
| `sgt` | Lenguaje Saggitarius: analisis, IR y generacion de codigo | `lorenz-arch` |

Ninguna herramienta guarda su propia copia de la definicion de la ISA: todas la consultan a
`lorenz-arch` (RNF-MAN-002).

## Reglas

1. `lorenz-arch` no depende de nada del resto del workspace. Es la raiz del grafo.
2. Un crate no accede al sistema de archivos salvo que su responsabilidad lo exija; quien lee
   archivos es `lorenz-cli` o una funcion explicita de carga.
3. Los errores de cada crate son un tipo propio con variantes nombradas, nunca cadenas de texto.
4. Todo lo publico lleva documentacion con ejemplo compilable.
5. Nada en el nucleo usa aleatoriedad ni la hora del sistema sin que se pueda fijar desde fuera
   (RNF-FIA-001, RF-TST-035).
