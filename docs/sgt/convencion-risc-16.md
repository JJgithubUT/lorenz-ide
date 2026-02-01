# Convencion de llamada de SGT en `risc-16`

Palabra de 16 bits, direcciones de 16 bits.

## Registros

| Registro | Uso |
| --- | --- |
| `R0` | valor de retorno y resultado de cada expresion |
| `R1` | auxiliar: direccion de retorno, operandos intermedios |
| `R2` | temporal de expresiones (se guarda en una celda si hay una llamada de por medio) |
| `R3` | temporal de expresiones (se guarda en una celda si hay una llamada de por medio) |
| `R4` | temporal de expresiones (se guarda en una celda si hay una llamada de por medio) |
| `R5` | temporal de expresiones (se guarda en una celda si hay una llamada de por medio) |
| `R6` | temporal de expresiones (se guarda en una celda si hay una llamada de por medio) |
| `R7` | apunta al inicio de los datos; permite leer y escribir con registro mas desplazamiento |

El compilador no usa `PC`: queda libre para ensamblador en linea y para `registro(...)`.

## Llamada

1. Quien llama evalua los argumentos, de izquierda a derecha, en registros temporales.
2. Guarda en celdas fijas los temporales que sigan vivos (el compilador no usa pila de datos: decision D-059).
3. Escribe cada argumento en la celda del parametro correspondiente de la funcion llamada.
4. Escribe en la celda de retorno de la funcion llamada la direccion a la que debe volver, y salta a ella con `J`.
5. La funcion llamada, al terminar, lee su celda de retorno en `R1` y vuelve con `JR`. El valor que devuelve queda en `R0`.

Las variables locales y los parametros viven en celdas fijas de la region de datos, una por variable y funcion. Por eso no hay recursion; el compilador la rechaza.

## Interrupciones

Un manejador (`@interrupcion(n)`) guarda en un area propia todos los registros que usa el compilador, ejecuta su cuerpo y los repone antes de volver con la instruccion de retorno de interrupcion. Cada manejador recibe su propia copia de las funciones que llama (decision D-063), porque la rutina interrumpida podria estar ejecutando la misma.

## Operadores resueltos con rutinas

| Operador | Rutina de la biblioteca |
| --- | --- |
| `*` sobre u16 | `__mul` |
| `/` sobre u16 | `__divu` |
| `%` sobre u16 | `__modu` |
| `/` sobre i16 | `__divs` |
| `%` sobre i16 | `__mods` |
| `<<` sobre u16 | `__shl` |
| `>>` sobre u16 | `__shr` |
| `>>` sobre i16 | `__sar` |
| `+|` sobre u16 | `__sumasat_u` |
| `-|` sobre u16 | `__restasat_u` |
| `+|` sobre i16 | `__sumasat_s` |
| `-|` sobre i16 | `__restasat_s` |
