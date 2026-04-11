# Simulación ciclo a ciclo

El emulador funcional (`lorenz ejecutar`) dice **qué** hace un programa. `lorenz ciclos` dice
**cuánto tarda** en una máquina con pipeline, predicción de saltos, caché y memorias con latencia
(RF-CIC-001). Las dos órdenes leen la **misma descripción** y el **mismo programa**: la
temporización vive en secciones propias que el emulador funcional ignora (RF-CIC-002).

Sin ninguna sección de tiempo, cada instrucción tarda lo que declara su campo `ciclos` y el total
coincide con el del modo funcional.

## `[pipeline]`

```toml
[pipeline]
adelantamiento = true      # RF-CIC-007; por omisión true
prediccion     = "2-bits"  # RF-CIC-009: no-tomado (omisión), tomado, 1-bit, 2-bits

[[pipeline.etapa]]         # RF-CIC-003, RF-CIC-004: tantas como se quiera, en orden
nombre = "IF"
hace   = ["leer la instruccion en pc", "avanzar pc"]
[[pipeline.etapa]]
nombre = "ID"
[[pipeline.etapa]]
nombre = "EX"
[[pipeline.etapa]]
nombre = "MEM"
[[pipeline.etapa]]
nombre = "WB"
```

Si no hacen falta las microoperaciones, `etapas = ["IF", "ID", "EX", "MEM", "WB"]` basta.

Cada etapa cumple un papel que el modelo necesita conocer. Por omisión se cuentan desde el final:
`escritura` es la última, `memoria` la anterior, `ejecucion` la antepenúltima y `lectura` la
anterior a esa (con cinco etapas: ID, EX, MEM, WB). Se pueden nombrar:

| Clave | Papel |
| --- | --- |
| `lectura` | Etapa donde se leen los registros fuente |
| `ejecucion` | Etapa que calcula; dura lo que declara `ciclos` si `ciclos_declarados = true` |
| `memoria` | Etapa de los accesos a datos; dura lo que cuestan según la caché y las latencias |
| `escritura` | Etapa donde se escribe el registro destino |
| `salto` | Etapa donde se sabe si un salto se toma; por omisión, `ejecucion` |
| `ciclos_objetivo` | Ciclos extra de un salto bien predicho como tomado (calcular el destino) |

### Riesgos

- **Datos** (RF-CIC-005, RF-CIC-006): una instrucción espera el registro o la bandera que produce
  otra. Con adelantamiento el valor pasa al terminar la etapa de cálculo, o la de memoria si viene de
  una carga (un ciclo de espera por el uso inmediato de una carga). Sin él, hay que esperar a que
  se escriba, y la lectura va después de la escritura en el mismo ciclo.
- **Estructurales**: una etapa lenta (una memoria con latencia) detiene a las anteriores.
- **Control** (RF-CIC-008, RF-CIC-011): un salto mal predicho retrasa la siguiente búsqueda hasta que
  sale de su etapa `salto`, y las instrucciones de la rama equivocada se descartan. Los saltos
  indirectos (a un registro o a memoria) nunca se aciertan.

Predictores (RF-CIC-009, RF-CIC-010): `no-tomado`, `tomado`, `1-bit` (repite lo último que hizo ese
salto) y `2-bits` (contador saturado; empieza en «débilmente no tomado»). Se cuentan aciertos y
fallos.

## `[cache]`

```toml
[cache]
tamano           = 64               # palabras en total
linea            = 4                # palabras por línea
vias             = 2                # líneas por conjunto
reemplazo        = "lru"            # lru, fifo, aleatorio (determinista)  RF-CIC-016
escritura        = "post-escritura" # o "escritura-directa"                RF-CIC-017
latencia_acierto = 1
instrucciones    = false            # si la búsqueda de instrucciones también pasa por ella
```

Un fallo trae la línea entera de la memoria, palabra a palabra con la latencia de su región; si la
línea desalojada estaba sucia, se vuelca primero. En escritura directa cada escritura va a memoria y
un fallo de escritura no trae la línea. Se cuentan aciertos y fallos separados por lectura y
escritura (RF-CIC-018) y `--cache` muestra cada línea con su etiqueta y sus bits de validez y
suciedad (RF-CIC-019).

## `[bus]`

```toml
[bus]
compartido = true   # RF-CIC-021; necesita un [pipeline]
```

Por omisión se supone que la busqueda de instrucciones, los accesos a datos y el DMA no se
estorban. Con un bus compartido se turnan: cada uno toma el primer hueco libre a partir del ciclo
en que lo pide, los ya reservados no se mueven (la instruccion mas antigua tiene prioridad) y lo que
espera se anota como causa `bus`. Solo cuentan como tiempo de bus los ciclos de la memoria
principal; un acierto de cache no lo usa. Las esperas salen en `lorenz ciclos` (`esperas por el bus
compartido`), en la vista (`espera bus`) y en `comparar-ciclos`.

## DMA

Un dispositivo `dma` (ver `docs/FORMATO.md`) copia memoria sin el procesador (RF-CIC-022). Cada
palabra cuesta leer el origen mas escribir el destino, con la latencia de su region, y suma al uso
del bus. Con `[bus] compartido` la copia ocupa el bus desde que la instruccion que la arranca sale de
la etapa de memoria y retrasa los accesos que lleguen despues; sin el, corre en paralelo y no frena
al procesador.

Limites: el emulador funcional copia al instante, asi que un programa que mira `estado` ve la copia
terminada aunque el modelo de ciclos la tenga en curso; y el DMA no pasa por la cache, de modo que no
la invalida.

## Latencias

La clave `latencia` de cada `[[region]]` (RF-CIC-020) es lo que cuesta tocar una palabra de esa
región; por omisión, un ciclo. Una búsqueda de instrucción cuesta la suma de sus palabras.

## La orden

```
lorenz ciclos maquina.toml programa.{hex|s|sgt} [--limite N] [--direccion A]
              [--vista CICLOS] [--diagrama INSTRUCCIONES] [--perfil] [--cache]
              [--calor] [--calor-html ARCHIVO]
              [--historial ARCHIVO [--etiqueta TEXTO]] [--vigilar]
lorenz comparar-ciclos a.toml b.toml programa
lorenz historial-ciclos historial.toml
```

| Salida | Requisito |
| --- | --- |
| Ciclos, instrucciones, ciclos por instrucción, esperas por causa, saltos y uso del bus | RF-CIC-025, RF-CIC-027 |
| Tamaño del código en bytes | RF-CIC-026 |
| `--vista N`: el contenido de cada etapa en los primeros N ciclos, con las esperas y su causa | RF-CIC-012, RF-CIC-013 |
| `--diagrama N`: una fila por instrucción y una columna por ciclo | RF-CIC-012 |
| `--perfil`: ciclos atribuidos a cada instrucción; la suma es el total | RF-CIC-031 |
| `--calor`, `--calor-html`: los ciclos de cada línea del fuente | RF-CIC-032 |
| `--vigilar`: vuelve a simular cuando cambia la descripción (con su herencia) o el programa | RF-CIC-028 |
| `comparar-ciclos`: las cifras de dos descripciones lado a lado, con la diferencia | RF-CIC-029 |
| `--historial`: añade la medición al archivo, con la versión de la arquitectura | RF-CIC-030 |

La **versión** de una arquitectura es una huella del TOML ya mezclado con su herencia: los
comentarios y el espaciado no la cambian, una latencia sí.

Desde Rust, `lorenz_cic::Simulador` ofrece lo mismo: `ejecutar`, `metricas`, `vista(ciclo)`,
`avanzar_ciclo` y `retroceder_ciclo` (RF-CIC-014), `diagrama` y `perfil`.

## Varios núcleos

```toml
[nucleos]
cantidad = 2
compartida = ["ram"]
```

Declara una máquina con `cantidad` núcleos (de 1 a 64) que comparten las regiones de `compartida`,
nombradas como en `[[region]]` (RF-CIC-023). Con más de un núcleo hace falta al menos una. Cada
núcleo tiene sus registros, su pipeline, su caché y sus dispositivos; lo único común es esa memoria.

```
lorenz nucleos maquina.toml programa... [--limite N] [--direccion A]
```

Se da un programa por núcleo, o uno solo para que lo ejecuten todos. La salida lista, por núcleo,
los ciclos, las instrucciones, el CPI y las esperas por el bus, y después el total (los ciclos del
último núcleo en terminar), las palabras compartidas copiadas y, si el bus es común, los ciclos que
estuvo ocupado. `--limite` cuenta instrucciones de todos los núcleos juntos.

Cómo se comportan:

- En cada paso ejecuta el núcleo que menos ciclos lleva (si empatan, el de menor número). Así el
  orden en que se ven las escrituras es el del tiempo simulado y el resultado se repite.
- Una escritura a una dirección compartida se copia a la memoria de los demás núcleos en cuanto
  termina su instrucción. Es una memoria secuencialmente consistente: todos ven las escrituras en
  el mismo orden y nadie ve una antes de que se haga.
- Al empezar, lo que hay en las regiones compartidas del primer núcleo se copia a los demás.
- Con `[bus] compartido = true` los accesos de *todos* los núcleos se turnan en un único bus
  (`esperas por el bus compartido`); sin él, cada núcleo tiene el suyo y no se estorban.
- Un DMA que escribe en una región compartida no se propaga a los otros núcleos todavía.

Desde Rust: `lorenz_cic::Sistema` (`nuevo`, `ejecutar`, `ejecutar_instruccion`, `metricas`,
`nucleo(i)`).

## Restricciones de diseño y variantes

La descripción puede fijar límites que el diseño debe cumplir (RF-CIC-033):

```toml
[[restriccion]]
nombre = "rapido"          # opcional; por omisión, el nombre de la métrica
metrica = "ciclos"
maximo = 400

[[restriccion]]
metrica = "palabras-codigo"
maximo = 64
```

Cada una pide `metrica` y al menos un `maximo` o un `minimo` (enteros o decimales; el mínimo no puede
superar al máximo). Las métricas son `ciclos`, `instrucciones`, `cpi`, `esperas-datos`,
`esperas-estructurales`, `esperas-control`, `esperas-bus`, `saltos-fallados`, `uso-del-bus`,
`fallos-cache` y `palabras-codigo` (el tamaño del programa). Una cifra que no se puede medir, como los
fallos de caché sin caché, no cumple.

```
lorenz restricciones maquina.toml programa [--limite N] [--direccion A]
lorenz variantes maquina.toml programa [--limite N] [--direccion A] [--bus] [--cuantas N]
```

`restricciones` ejecuta el programa y muestra, por cada límite, lo medido y si se cumple. Sale con 0
si se cumplen todas, 5 si falla alguna y 3 si el programa no termina dentro del límite (con un
programa cortado las cifras no valen).

`variantes` prueba las combinaciones de adelantamiento, predicción de saltos, tamaño de caché (la
mitad, igual, el doble y el cuádruple, siempre que sigan siendo válidos) y, con `--bus`, bus
compartido o separado. Muestra primero las que cumplen todo y, entre ellas, las más rápidas. Después
estima, para cada instrucción nueva que propondría `sugerir`, qué cifras tendría el programa y si
cumpliría; esa parte es una estimación y no simula el repertorio ampliado. Sale con 5 si ninguna
variante cumple. Desde Rust: `lorenz_cic::{variantes, evaluar, variantes_isa}`.

## Sugerencias de instrucciones nuevas

```
lorenz sugerir maquina.toml programa... [--minima 2] [--maxima 4] [--cuantas 5] [--limite N]
```

Ejecuta los programas y busca las secuencias de instrucciones contiguas que se repiten (RF-CIC-034).
Una secuencia no puede cruzar un destino de salto ni seguir a una instruccion que salta. Por cada una
dice cuantas veces aparece en el codigo, cuantas se ejecuta y cuanto se ahorraria si fuera una sola
instruccion que ocupara lo que la mas larga de ellas y tardara lo que la mas lenta: palabras de codigo
y ciclos. Es una estimacion para orientar el repertorio, no mira los operandos y no comprueba que la
instruccion nueva se pueda codificar. Desde Rust: `lorenz_cic::sugerir::sugerir`.

## Cómo se calcula

El emulador funcional ejecuta cada instrucción y el planificador calcula en qué ciclo pasa por cada
etapa, con una recurrencia: una instrucción entra a una etapa cuando terminó la anterior, la etapa
quedó libre, sus operandos están listos y no hay un salto sin resolver. Como la ejecución es en orden,
el horario de una instrucción solo depende de las anteriores. Por eso el resultado es determinista,
retroceder un ciclo es leer el horario ya calculado y el modo funcional no cambia.

Límites: las interrupciones no tienen coste de entrada; la memoria se modela con una sola cola
(no hay contención de bus entre componentes, RF-CIC-021, que es de la fase 6).
