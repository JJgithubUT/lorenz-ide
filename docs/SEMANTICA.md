# El lenguaje de semantica

Cada instruccion de una arquitectura declara que le hace a la maquina con el campo `semantica`.
El texto se analiza **una sola vez al cargar** la descripcion y el emulador interpreta el arbol
resultante (decision D-009). Cambiar la arquitectura no exige recompilar nada.

## Gramatica

```text
programa  := [ sentencia { ';' sentencia } [ ';' ] ]
sentencia := 'detener'
           | 'si' expr ':' cuerpo
           | destino '=' expr
cuerpo    := '{' programa '}' | sentencia
destino   := ident | 'mem' '[' expr ']' | 'out' '[' expr ']'
unario    := ( '!' | '~' | '-' ) unario | primario
primario  := numero | ident | 'mem' '[' expr ']' | 'in' '[' expr ']' | '(' expr ')'
numero    := decimal | '0x' hex | '0b' binario     (se admite '_' como separador)
```

Un texto vacio es valido y significa que la instruccion no hace nada.

## Operadores

De menor a mayor precedencia, todos asociativos a la izquierda. La tabla sigue el orden de Rust:
`a == b | c` se lee `a == (b | c)`.

| Nivel | Operadores | Significado |
| --- | --- | --- |
| 1 | `\|\|` | o logico |
| 2 | `&&` | y logico |
| 3 | `==` `!=` `<` `<=` `>` `>=` | comparacion; el resultado es 1 o 0 |
| 4 | `\|` | o de bits |
| 5 | `^` | xor de bits |
| 6 | `&` | y de bits |
| 7 | `<<` `>>` | desplazamiento |
| 8 | `+` `-` | suma y resta |
| 9 | `*` `/` `%` | producto, division, modulo |
| — | `!` `~` `-` (unarios) | negacion logica, complemento de bits, cambio de signo |

## Nombres

Un nombre dentro de la semantica puede ser:

1. Un **operando** de la instruccion (`rd`, `imm`, `dir`).
2. Un **registro** por su nombre declarado (`A`, `SP`).
3. Una **bandera** por su nombre declarado (`Z`, `C`); vale 1 si esta encendida.
4. Los alias **`pc`** y **`sp`**, que apuntan al registro con ese rol, se llame como se llame.

`mem`, `in` y `out` son palabras con significado propio solo cuando van seguidas de `[`.
`si` y `detener` son palabras reservadas.

Solo se puede asignar a un operando de tipo registro, a un registro que no sea de rol `fijo`, a una
bandera, a `pc`, a `sp`, a `mem[...]` y a `out[...]`. El validador lo comprueba al cargar (`E-SEMANT`).

## Aritmetica

Las expresiones se evaluan con enteros de 128 bits con signo y se **truncan al ancho del destino**
solo al asignar. Eso permite detectar el acarreo: se compara el resultado sin truncar con el
rango del destino.

## Banderas (decision D-013)

La lista `afecta` de una instruccion nombra las banderas que modifica. Las tres convenciones de
nombre se calculan solas a partir del **ultimo valor asignado** por la instruccion:

| Bandera | Se enciende cuando |
| --- | --- |
| `Z` | el resultado truncado es cero |
| `N` | el bit mas alto del resultado truncado es 1 |
| `C` | la ultima operacion fue `+`, `-`, `*` o `<<` y el resultado sin truncar queda fuera del rango del destino (acarreo en la suma, prestamo en la resta) |

Cualquier otra bandera declarada en `afecta` no se toca por convencion: se asigna de forma
explicita en la semantica (`V = ...`). Una asignacion explicita a una bandera siempre manda sobre la
convencion.

Las banderas que una instruccion **no** lista en `afecta` quedan intactas (RF-EMU-007).

## Ejemplos

```text
rd = rd + rs                      # suma; con afecta = ["Z", "C"] actualiza cero y acarreo
mem[dir] = rs                     # escritura en memoria
si Z: pc = dir                    # salto condicional
sp = sp - 1; mem[sp] = rs         # apilar
out[puerto] = rs                  # salida a un puerto
detener                           # parar la maquina
```
