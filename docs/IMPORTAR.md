# Importar arquitecturas

`lorenz importar tabla.tabla [-o maquina.toml]` convierte una tabla de instrucciones en texto en
la descripcion TOML de una maquina y la valida antes de escribirla (si es incoherente no se crea
el archivo). Sin `-o` imprime el TOML.

Formato, una linea por elemento (`#` comenta):

| Linea | Significado |
| --- | --- |
| `maquina: nombre palabra=8 direccion=16 endian=little arranque=0` | datos generales |
| `registros: A B C D` | registros de uso general |
| `pc: PC 16` / `sp: SP 16 0xEFFF` | contador de programa y puntero de pila |
| `bandera: Z 0 descripcion` | una bandera (crea el registro `F`) |
| `region: rom 0x0000 0x7FFF r` | region de memoria |
| `instruccion: sintaxis ; codigo ; semantica ; ciclos [; afecta]` | una instruccion |

Los operandos se clasifican por nombre: `rd`, `rs`, `rt`... registros; `imm` inmediato sin signo y
`imms` con signo (el rango sale del ancho del campo en el codigo); `dir` direccion; `off`/`rel`
desplazamiento. Los ciclos son un numero o `tomado/no_tomado`.

`importadores/` trae las tablas de JDH-8 y Astro-8, reducidas a partir de su documentacion
publica. Los codigos de operacion son los de esas tablas, no necesariamente los del hardware
original: revisa la semantica antes de usarlas como referencia.
