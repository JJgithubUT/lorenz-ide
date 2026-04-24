# Formato de descripción de arquitectura

Una máquina de Lorenz se describe en un archivo TOML. Este documento es la referencia completa:
qué tablas existen, qué claves admite cada una, qué valores toman y qué reglas comprueba el
cargador. La versión del formato que entiende esta edición es **0.1**; un archivo con otra versión
mayor se rechaza en lugar de interpretarse a medias (RF-ARQ-002).

El lenguaje de la clave `semantica` tiene su propia referencia en [SEMANTICA.md](SEMANTICA.md).
Los ejemplos completos están en `ejemplos/`.

## Gramática

```text
archivo     := maquina [flags] { registro } { par } { instruccion } { region } { dispositivo }

maquina     := '[maquina]'         clave*  [ '[maquina.interrupciones]' clave* ]
flags       := '[flags]'           ( NOMBRE '=' '{' 'bit' '=' ENTERO [ ',' 'descripcion' '=' TEXTO ] '}' )*
registro    := '[[registro]]'      clave*
par         := '[[par]]'           clave*
instruccion := '[[instruccion]]'   clave*  { '[[instruccion.operando]]' clave* }
region      := '[[region]]'        clave*
dispositivo := '[[dispositivo]]'   clave*
```

Las tablas `[[...]]` se repiten tantas veces como elementos haya. El orden en que se declaran los
registros es el orden en que se muestran; el orden de las instrucciones no importa.

Los números se escriben en decimal, `0x` hexadecimal o `0b` binario. Los textos van entre comillas.

## `[maquina]`

| Clave | Tipo | Por omisión | Significado |
| --- | --- | --- | --- |
| `nombre` | texto | obligatoria | Nombre de la máquina |
| `formato` | texto | obligatoria | Versión del formato, por ejemplo `"0.1"` |
| `bits_palabra` | entero | obligatoria | Ancho de una palabra de memoria y de instrucción |
| `bits_direccion` | entero | obligatoria | Ancho de una dirección |
| `endianness` | `"little"` o `"big"` | `"little"` | Orden de los fragmentos de un campo repartido en varias palabras |
| `arranque` | entero | `0` | Dirección donde empieza el contador de programa |
| `pila` | `"ascendente"` o `"descendente"` | `"descendente"` | Sentido en que crece la pila |
| `hereda` | texto | ninguna | Archivo de otra descripción de la que se parte; ver [Herencia](#herencia) |

### `[maquina.interrupciones]`

| Clave | Tipo | Por omisión | Significado |
| --- | --- | --- | --- |
| `vectores` | tabla `línea = dirección` | vacía | Dirección a la que salta cada línea |
| `prioridades` | tabla `línea = entero` | la propia línea | Prioridad de cada línea; menor es más urgente |
| `habilitadas_al_arrancar` | booleano | `false` | Si las interrupciones están permitidas al empezar |

## Herencia

Una descripción parte de otra con `hereda` en `[maquina]` y declara solo lo que cambia
(RF-ARQ-035). La ruta es relativa a la carpeta del archivo que hereda y la cadena admite hasta 8
niveles; un ciclo es un error.

```toml
[maquina]
hereda = "ejemplo-8/maquina.toml"
nombre = "ejemplo-8-turbo"

[[instruccion]]            # sustituye a la ADD del padre
mnemonico = "ADD"
# ...

[[instruccion]]            # quita MOV
mnemonico = "MOV"
eliminar = true
```

La mezcla se hace antes de validar, así que el resultado pasa por las mismas reglas que cualquier
descripción:

- Las claves de `[maquina]` del hijo sustituyen a las del padre.
- `[flags]` se mezcla por nombre de bandera.
- `[[registro]]`, `[[par]]`, `[[instruccion]]` (por `mnemonico`), `[[region]]` y `[[dispositivo]]` se
  mezclan por `nombre`. Un elemento del hijo con el mismo nombre sustituye al del padre en su sitio
  y entero, no clave a clave; uno nuevo se añade al final; con `eliminar = true` se quita el del
  padre.
- Solo se pueden cargar descripciones heredadas desde archivo, no desde texto suelto.
- Una recarga ve también los cambios hechos en los archivos base.

## `[flags]`

Cada clave es el nombre de una bandera y su valor una tabla con el `bit` que ocupa dentro del
registro de banderas y una `descripcion` opcional. Dos banderas no pueden ocupar el mismo bit.

## `[[registro]]`

| Clave | Tipo | Por omisión | Significado |
| --- | --- | --- | --- |
| `nombre` | texto | obligatoria | Nombre único entre registros y pares |
| `bits` | entero | obligatoria | Ancho del registro |
| `rol` | texto | `"general"` | Uno de los de la tabla siguiente |
| `inicial` | entero | `0` | Valor al arrancar o reiniciar |

| Rol | Significado |
| --- | --- |
| `general` | Registro de uso corriente |
| `pc` | Contador de programa; exactamente uno |
| `sp` | Puntero de pila; como máximo uno |
| `flags` | Registro que contiene las banderas; como máximo uno, y obligatorio si se declara alguna bandera |
| `indirecto` | Registro cuyo valor se usa como dirección de memoria; es una etiqueta para la documentación y las herramientas, no cambia la ejecución |
| `fijo` | Solo lectura: una instrucción no puede asignarlo (RF-ARQ-008) |

## `[[par]]`

Un par junta dos registros generales como uno solo más ancho (RF-ARQ-010). Su nombre vale donde
vale el de un registro: en la lista `de` de un operando y en la semántica. Leerlo concatena el
registro alto con el bajo; escribirlo reparte el valor entre ambos.

| Clave | Tipo | Significado |
| --- | --- | --- |
| `nombre` | texto | Nombre único entre registros y pares |
| `alto` | texto | Registro que aporta los bits más significativos |
| `bajo` | texto | Registro que aporta los menos significativos |

Reglas (`E-PAR`): ambos registros existen, son de rol `general`, son distintos y suman como máximo
64 bits.

```toml
[[par]]
nombre = "HL"
alto   = "H"
bajo   = "L"
```

## `[[banco]]`

Un banco de registros junta registros que existen en varias copias, de las que solo se ve una a la
vez (RF-ARQ-012): los registros sombra de las interrupciones, o los conjuntos de registros por
tarea. El valor de un registro selector elige la copia visible; las demás conservan el suyo.

| Clave | Tipo | Significado |
| --- | --- | --- |
| `selector` | texto | Registro cuyo valor elige la copia; si vale más que `copias`, se cuenta en vueltas |
| `registros` | lista | Registros de uso general que tienen una copia por banco |
| `copias` | entero | Cuántas copias hay, al menos 2 |

Cada copia empieza con el valor `inicial` del registro. `reiniciar` vuelve a la copia 0 y restablece
todas. Reglas (`E-BANCO`): el selector y los registros existen, los registros son de rol `general`,
ninguno es el propio selector ni está en dos bancos.

```toml
[[banco]]
selector  = "MODO"
registros = ["R4", "R5", "R6"]
copias    = 2
```

## `[[instruccion]]`

| Clave | Tipo | Por omisión | Significado |
| --- | --- | --- | --- |
| `mnemonico` | texto | obligatoria | Nombre único de la instrucción |
| `sintaxis` | texto | vacía | Plantilla que lee el ensamblador, con los operandos entre llaves: `"ADD {rd}, {rs}"` |
| `codigo` | texto | obligatoria | Codificación binaria, ver más abajo |
| `semantica` | texto | vacía | Qué hace; vacía significa que no hace nada |
| `afecta` | lista de textos | vacía | Banderas que la instrucción modifica |
| `ciclos` | entero o tabla | `1` | Coste en ciclos; `{ tomado = 3, no_tomado = 2 }` para un salto |
| `descripcion` | texto | vacía | Explicación para la documentación generada |

La plantilla de `sintaxis` empieza por el mnemónico y menciona cada operando exactamente una vez
(`E-SINTAXIS`).

### Codificación

Se escribe de izquierda a derecha, del bit más significativo al menos significativo. Los espacios
solo agrupan.

| Elemento | Escritura | Significado |
| --- | --- | --- |
| Bit fijo | `0`, `1` | Forma el código de operación |
| Campo | `nombre:n` | `n` bits que reciben el operando `nombre` |
| Relleno | `.` | Se ignora al decodificar y vale cero al ensamblar |
| Palabra extra | `\|` | Lo que sigue ocupa la siguiente palabra |

Cada palabra debe sumar `bits_palabra` (`E-BITS`). Un campo puede repetirse en palabras distintas
pero no dentro de la misma; sus fragmentos se concatenan, el primero como el menos significativo en
una máquina `little` y como el más significativo en una `big`. Dos instrucciones no pueden aceptar
la misma palabra (`E-AMBIG`).

### `[[instruccion.operando]]`

| Clave | Tipo | Significado |
| --- | --- | --- |
| `nombre` | texto | Debe coincidir con un campo de la codificación (`E-CAMPO`) |
| `tipo` | texto | `registro`, `inmediato`, `direccion`, `desplazamiento` o `puerto` |
| `de` | lista | Solo en `registro`: registros o pares permitidos; el campo codifica su posición en la lista |
| `signo` | booleano | Solo en `inmediato`: si admite negativos |
| `rango` | `[mín, máx]` | Solo en `inmediato`: valores admitidos |

Un `desplazamiento` es relativo al contador de programa y siempre lleva signo.

## `[[region]]`

| Clave | Tipo | Por omisión | Significado |
| --- | --- | --- | --- |
| `nombre` | texto | obligatoria | Nombre único |
| `desde`, `hasta` | enteros | obligatorias | Rango de direcciones, ambos incluidos |
| `acceso` | `"r"`, `"w"` o `"rw"` | `"rw"` | Qué puede hacer el programa; el acceso privilegiado del depurador no se limita |
| `dispositivo` | texto | ninguno | Dispositivo mapeado en la región |
| `contenido` | texto | ninguno | Archivo `.hex` que llena la región al cargar; la ruta es relativa a la carpeta de la descripción |
| `bancos` | entero | `1` | Copias de la región, de las que se ve una a la vez (RF-ARQ-033) |
| `selector_puerto` | entero | ninguno | Puerto de salida cuyo valor elige el banco visible; obligatorio si `bancos` pasa de 1 |
| `latencia` | entero | `1` | Ciclos que cuesta tocar una palabra; solo la usa `lorenz ciclos` (ver `docs/CICLOS.md`) |

Las regiones no se solapan (`E-SOLAPE`) y la dirección de arranque cae dentro de una (`E-ARRANQUE`).

Con `bancos`, escribir `n` en `out[selector_puerto]` muestra el banco `n` (en vueltas si hay menos) y
leer ese puerto devuelve el banco visible. Cada banco empieza a cero y guarda sus propios datos en
las mismas direcciones. Reglas (`E-BANCO-MEM`): con varios bancos hay selector, dos regiones no
comparten puerto y la región no es de un dispositivo.

## `[[dispositivo]]`

| Clave | Tipo | Significado |
| --- | --- | --- |
| `nombre` | texto | Nombre único |
| `tipo` | texto | `pantalla`, `teclado`, `uart`, `temporizador`, `bloques`, `dma` o `generico` |
| `puertos` | tabla `nombre = número` | Puertos de entrada y salida que atiende |
| `interrupcion` | entero | Línea que levanta, si levanta alguna |

Claves propias de cada tipo:

| Tipo | Claves | Por omisión |
| --- | --- | --- |
| `pantalla` | `ancho`, `alto`, `formato` | 64, 32, `"indice1"` |
| `temporizador` | `periodo` (en ciclos) | 1000 |
| `bloques` | `tamano_bloque`, `cantidad_bloques` | 512, 64 |
| `dma` | puertos `origen`, `destino`, `cuenta` y `control` (y `estado` si se quiere) | — |
| `generico` | `script` en el lenguaje de semántica, que se ejecuta al escribir en el dispositivo | vacío |

Una región que nombra un dispositivo exige que esté declarado (`E-DISPOSITIVO`).

### Controlador DMA

Un dispositivo `dma` copia memoria sin que la mueva el procesador (RF-CIC-022). El programa escribe
la dirección de `origen`, la de `destino` y la `cuenta` de palabras, y arranca la copia escribiendo
un valor con el bit 0 encendido en `control`. La copia se hace entera en ese momento: el
emulador funcional no distingue "en curso" de "terminada", y `estado` vale siempre 1 (bit 0, listo),
más 2 (bit 1) si la última transferencia falló. Falla, sin copiar nada, si una dirección cae fuera
de las regiones, el destino es de solo lectura o de un dispositivo. Si declara `interrupcion`, la
línea se levanta al terminar. Leer `origen`, `destino` y `cuenta` devuelve lo escrito; leer `control`
da 0. A un `dma` al que le falte un puerto obligatorio se le rechaza al cargar.

El tiempo que la copia ocupa el bus lo cuenta `lorenz ciclos` (ver `docs/CICLOS.md`).

```toml
[[dispositivo]]
nombre = "dma0"
tipo   = "dma"
puertos = { origen = 0x20, destino = 0x21, cuenta = 0x22, control = 0x23, estado = 0x24 }
interrupcion = 0
```

## `[bus]`

Solo lo lee `lorenz ciclos`. `compartido = true` hace que la búsqueda de instrucciones, los accesos a
datos y el DMA se turnen en un único bus (RF-CIC-021); exige `[pipeline]`. Ver `docs/CICLOS.md`.

## `[nucleos]`

Solo lo lee `lorenz nucleos`. `cantidad` (1 a 64) y `compartida`, la lista de nombres de `[[region]]`
que ven todos los núcleos (RF-CIC-023). Ver `docs/CICLOS.md`.

## `[coherencia]`

Solo lo lee `lorenz nucleos`. `protocolo` (`msi`) y `latencia_invalidacion`, los ciclos que cuesta
anular una línea ajena. Exige `[cache]` (RF-CIC-024). Ver `docs/CICLOS.md`.

## `[[restriccion]]`

Solo las leen `lorenz restricciones` y `lorenz variantes`. Cada una tiene `metrica` y un `maximo`, un
`minimo` o ambos, y puede llevar `nombre` (RF-CIC-033). Ver `docs/CICLOS.md`.

## `[gpu]`

Declara una unidad gráfica: resolución, color, cola de comandos, costes y, en `[gpu.hilos]`, el
modelo de warps. Solo la leen `lorenz gpu`, `lorenz kernel`, `lorenz depurar-kernel` y
`lorenz comparar-gpu`; el emulador funcional la ignora (RF-GPU-001, RF-GPU-021). Ver
`docs/GPU.md`.

## Comentarios y documentación

`lorenz documentar maquina.toml -o isa.md` genera la documentación de la arquitectura: resumen,
registros, pares y banderas, mapa de memoria, dispositivos, interrupciones, la tabla de códigos de
operación ordenada, y por cada instrucción su codificación dibujada, sus operandos y su coste.
Con `--vigilar` la vuelve a escribir sola cada vez que el archivo cambia.

Los comentarios del TOML llegan al documento (RF-ARQ-036):

- El bloque de comentarios que abre el archivo describe la máquina entera.
- Un bloque pegado a la cabecera de una tabla (`[[registro]]`, `[[instruccion]]`, `[[par]]`,
  `[[region]]`, `[[dispositivo]]`) describe ese elemento. Una línea en blanco entre el bloque y la
  cabecera lo desconecta.
- En `[flags]`, el comentario de la línea anterior describe la bandera.

## Errores de validación

El cargador acumula todos los fallos antes de rechazar la descripción y a cada uno le da su código,
el requisito que lo exige y la línea y columna del archivo (RF-ARQ-034).

| Código | Qué se comprueba | Requisito |
| --- | --- | --- |
| `E-BITS` | Cada palabra suma `bits_palabra` | RF-ARQ-016 |
| `E-AMBIG` | No hay dos instrucciones con la misma codificación | RF-ARQ-017 |
| `E-CAMPO` | Cada campo tiene operando y cada operando tiene campo | RF-ARQ-018 |
| `E-ROL` | Los roles únicos aparecen una sola vez | RF-ARQ-009 |
| `E-PAR` | Los pares juntan registros generales válidos | RF-ARQ-010 |
| `E-SOLAPE` | Las regiones no se solapan | RF-ARQ-027 |
| `E-ARRANQUE` | El arranque cae dentro de una región | RF-ARQ-028 |
| `E-DISPOSITIVO` | Todo dispositivo citado existe | RF-ARQ-031 |
| `E-BANDERA` | Toda bandera citada está declarada | RF-ARQ-024 |
| `E-DUPLICADO` | No hay nombres repetidos | RF-ARQ-007 |
| `E-RANGO` | Los valores están dentro de lo que admite el formato | RF-ARQ-003 |
| `E-SEMANT` | La semántica se analiza y solo cita nombres que existen | RF-ARQ-022 |
| `E-SINTAXIS` | La plantilla de sintaxis es válida y cuadra con los operandos | RF-ARQ-013 |
| `E-BANCO` | Los bancos de registros tienen selector, copias y registros válidos | RF-ARQ-012 |
| `E-BANCO-MEM` | Las regiones con bancos tienen puerto selector propio | RF-ARQ-033 |

`lorenz validar maquina.toml` aplica todas las reglas y muestra los fallos de una vez.
