# La unidad gráfica

Lorenz modela una unidad gráfica (GPU) como un componente más de la máquina. Es un modelo
**educativo, no realista**: sirve para ver cómo una CPU le manda trabajo a un coprocesador, cómo se
dibuja un triángulo y por qué miles de hilos pequeños pueden ganarle a un bucle (RF-GPU-001 a 022).

Tiene dos modos que no comparten estado:

- **Modo gráfico:** un framebuffer, una cola de comandos y unas pocas primitivas de dibujo.
- **Modo de hilos:** hilos agrupados en warps que corren un *kernel*, con memoria compartida,
  divergencia y barreras.

Todo vive en el crate `lorenz-gpu` y se configura en la sección `[gpu]` de la descripción de la
máquina. Una descripción **sin `[gpu]` no tiene unidad gráfica**: el emulador funcional ni la nota
(RF-GPU-021), igual que ignora `[pipeline]` o `[nucleos]`.

## Configuración

```toml
[gpu]
dispositivo  = "gpu0"   # nombre del [[dispositivo]] que la CPU maneja (solo en modo gráfico)
ancho        = 32       # resolución del framebuffer, de 1x1 a 4096x4096 (por omisión 64x48)
alto         = 24
color        = 16       # bits por pixel: 8 (RGB332), 16 (RGB565) o 24 (por omisión)
profundidad  = true     # buffer de profundidad
cola         = 8        # comandos que caben esperando (1 a 65536, por omisión 16)
interrupcion = 3        # línea que se levanta al terminar un lote
razon_reloj  = 1        # ciclos de la GPU por cada ciclo de la CPU (1 a 1024)
ciclos_comando   = 4    # coste base de cada comando (al menos 1)
ciclos_pixel     = 1    # por pixel escrito
ciclos_triangulo = 8    # preparar un triángulo

[gpu.hilos]
tamano_warp         = 8     # hilos que avanzan juntos (1 a 64)
unidades            = 2     # unidades de ejecución (1 a 256)
memoria_compartida  = 256   # palabras por grupo
memoria_global      = 4096  # palabras
latencia_global     = 20    # ciclos que espera un warp por un acceso a memoria global
latencia_compartida = 2
ciclos_multiplicar  = 4     # mul, div y rem
```

Cualquier clave desconocida es un error, con el nombre de la clave.

## Modo gráfico

### Comandos

Un comando es una palabra de operación seguida de un número fijo de argumentos. Los argumentos son
enteros con signo; el color va como `0xRRGGBB`.

| Operación | Código | Argumentos | Qué hace |
| --- | --- | --- | --- |
| `nada` | 0 | | No hace nada |
| `color` | 1 | `rgb` | Fija el color de lo que se dibuje después |
| `limpiar` | 2 | | Borra toda la superficie (y la profundidad) con el color actual |
| `rellenar` | 3 | `x y ancho alto` | Rellena un rectángulo; lo que cae fuera se recorta |
| `linea` | 4 | `x0 y0 x1 y1` | Línea de Bresenham con ambos extremos |
| `triangulo` | 5 | `x0 y0 x1 y1 x2 y2` | Triángulo de color plano |
| `triangulo_color` | 6 | `x0 y0 c0 x1 y1 c1 x2 y2 c2` | Triángulo con el color interpolado entre vértices |
| `profundidad` | 7 | `z` | Fija la profundidad de lo que se dibuje después |
| `prueba_z` | 8 | `0` o `1` | Activa la prueba de profundidad (si hay buffer) |
| `fin_lote` | 9 | | Marca el final de un lote: puede interrumpir a la CPU |

Un pixel pertenece a un triángulo si su centro está dentro o sobre una arista; un triángulo de área
cero no dibuja nada y el orden de los vértices no importa. Con la prueba de profundidad, un pixel solo
se escribe si su `z` es **menor** que el guardado (más cerca). La profundidad es una por primitiva,
no por vértice (D-105).

### Guiones `.gpu`

Para probar sin escribir un programa de CPU, un guion de texto lleva una orden por línea, con los
mismos nombres; `;` empieza un comentario y los números pueden ir en decimal, `0x...` o `#RRGGBB`:

```text
prueba_z 1
color #101830
limpiar
profundidad 5
color #E0E0E0
linea 0 23 31 0
fin_lote
```

```
lorenz gpu maquina.toml figuras.gpu --imagen salida.png --ampliar 8
```

### Manejarla desde la CPU

La descripción declara un dispositivo `generico` **sin script** con tres puertos, y `[gpu]` lo nombra
(ver `ejemplos/gpu-16/maquina.toml`):

```toml
[[dispositivo]]
nombre       = "gpu0"
tipo         = "generico"
puertos      = { comando = 0x20, estado = 0x21, control = 0x22 }
interrupcion = 3
```

- **`comando`** (se escribe): la CPU deja aquí las palabras de cada comando, una tras otra
  (`OUT 0x20, R1`). La unidad las junta hasta tener un comando completo y lo mete en la cola.
- **`estado`** (se lee): bit 0, la unidad está ocupada; bit 1, la cola está llena; bit 2, hubo un
  error (operación desconocida o cola desbordada); bit 3, terminó un lote. Desde el bit 8, los
  huecos libres de la cola.
- **`control`** (se escribe): un 1 borra los bits 2 y 3 de `estado`.

Las palabras se leen con el ancho de la máquina (D-104): los argumentos con signo se extienden, y los
colores van en el formato nativo de la palabra, RGB332 en 8 bits, RGB565 en 16 y `0xRRGGBB` desde 24.
Si la CPU escribe con la cola llena, el comando se pierde y se enciende el bit de error; lo correcto
es mirar `estado` antes. Si la línea `interrupcion` está declarada, se levanta al terminar cada lote
(`fin_lote`).

### Tiempo

La unidad tiene su **propio reloj** (RF-GPU-017). Toma un comando de la cola cuando está libre y queda
ocupada su coste: `ciclos_comando` + `ciclos_triangulo` (si es un triángulo) + `ciclos_pixel` por
cada pixel escrito. Mientras tanto la CPU sigue. Por cada ciclo de la CPU la unidad avanza
`razon_reloj` ciclos. `lorenz gpu` da los dos totales por separado y el uso de la unidad (ciclos
ocupada entre ciclos totales).

Simplificación (D-101): el efecto de un comando sobre la superficie se aplica **al tomarlo**, y el
coste se paga después como tiempo ocupado. El resultado final y el tiempo total son los mismos; solo
cambia el instante en que un pixel se ve.

### Ver el resultado

```
lorenz gpu maquina.toml programa.s --imagen salida.png --ampliar 8 --pixel 20,12 --texto
```

- `--imagen` vuelca el framebuffer a **PNG** o **PPM** (por la extensión), ampliado un factor entero
  (RF-GPU-004). El PNG va sin comprimir, sin dependencias (D-106).
- `--pixel X,Y` describe un pixel: `(20, 12): #732C63 (r=115, g=44, b=99), z=3` (RF-GPU-022).
- `--texto` dibuja la superficie con caracteres según el brillo.

El visor gráfico en tiempo real (RF-GPU-003) llega con el IDE; usará estas mismas operaciones.

## Modo de hilos

### El modelo

Un *kernel* es un programa que corren **todos los hilos a la vez**, cada uno con sus registros. Los
hilos se reparten en *grupos*; los de un grupo comparten memoria compartida y pueden esperarse en una
barrera. Dentro de un grupo se agrupan en *warps* de `tamano_warp` hilos que avanzan en lockstep: en
cada emisión el warp ejecuta **una** instrucción para todos sus hilos activos.

Cada **unidad de ejecución** emite como mucho una instrucción de warp por ciclo y corre un grupo a la
vez; los grupos se reparten entre las unidades según van quedando libres. Un acceso a memoria deja
al warp esperando su latencia, tiempo que otros warps del mismo grupo aprovechan: con más warps se
esconde mejor la latencia, que es justo lo que hace rápida a una GPU.

### Divergencia

Cada hilo lleva su propio contador de programa. El warp emite la instrucción del **menor contador**
entre sus hilos vivos y activa solo los que están en ese punto. Cuando un salto separa a los hilos
(`bz`, `bnz`), el warp recorre un camino y después el otro, con la mitad de los carriles parados; en
cuanto los contadores coinciden otra vez, los hilos se reagrupan solos (D-102). Es el modelo más
sencillo que reproduce el comportamiento visible de la divergencia.

`--traza` marca, en cada instrucción de warp, qué hilos trabajaron y cuándo el warp estaba dividido
(RF-GPU-016):

```
ciclo     2  u0  g0.w0  pc=2   [########] bz    r1, par
ciclo     3  u0  g0.w0  pc=3   [.#.#.#.#] divergente mul   r2, r0, 100
ciclo     8  u0  g0.w0  pc=5   [#.#.#.#.] divergente li    r2, 7
ciclo     9  u0  g0.w0  pc=6   [########] st    [r0+0], r2
```

### El lenguaje de los kernels

Es el de la GPU, no el de la CPU de la máquina (D-107): registros `r0` a `r15` de 64 bits con signo, y
el segundo operando de las operaciones aritméticas es un registro o una constante. `;` empieza un
comentario y una etiqueta termina en `:`.

| Instrucción | Qué hace |
| --- | --- |
| `li rd, n` · `mov rd, rs` | Cargar una constante; copiar un registro |
| `add sub mul div rem and or xor shl shr slt seq rd, ra, rb` | Aritmética y lógica; `slt` y `seq` dan 0 o 1 |
| `tid rd` · `gid rd` · `bid rd` · `bdim rd` | Índice del hilo en su grupo; índice global; índice del grupo; hilos por grupo |
| `ld rd, [ra+off]` · `st [ra+off], rs` | Memoria global |
| `lds rd, [ra+off]` · `sts [ra+off], rs` | Memoria compartida del grupo |
| `bra et` · `bz rs, et` · `bnz rs, et` | Saltos (los condicionales por hilo: de ahí la divergencia) |
| `bar` | Barrera: espera a todos los hilos vivos del grupo |
| `exit` | Termina el hilo |

Directivas: `.global N` (palabras de memoria global), `.compartida N`, `.dato DIR v1 v2 ...` (valores
iniciales) y `.lanzar GRUPOS HILOS`. Una división por cero o un acceso fuera de la memoria es un error
con el grupo, el warp, el hilo y la línea.

### Memoria compartida y barreras

Cada grupo tiene su propia memoria compartida, que empieza en ceros. `bar` hace esperar a cada hilo
hasta que todos los hilos vivos de su grupo han llegado a una barrera; los que ya terminaron con
`exit` no cuentan. Es lo que permite, por ejemplo, la reducción de `kernels/reduccion.kgpu`: cada
paso suma la mitad de los elementos sobre la otra mitad, con una barrera entre pasos.

### Órdenes

```
lorenz kernel maquina.toml kernel.kgpu [--grupos N] [--hilos N] [--traza] [--volcar DESDE:CANTIDAD]
lorenz depurar-kernel maquina.toml kernel.kgpu [--guion ARCHIVO]
lorenz comparar-gpu maquina.toml programa kernel.kgpu [--resultado DIR_CPU:DIR_GPU:CANTIDAD]
```

`kernel` da los ciclos, las instrucciones de warp (y cuántas con el warp dividido), el uso de
carriles, los ciclos ociosos y los accesos a cada memoria. Un kernel que no termina se corta tras
`--limite` instrucciones de warp (código de salida 3).

`depurar-kernel` (RF-GPU-019) avanza de a una instrucción de warp:

| Orden | Qué hace |
| --- | --- |
| `paso [N]` | Ejecuta N instrucciones de warp y las muestra |
| `continuar` | Corre hasta el final |
| `hilo G.W.L` | Registros, contador y estado de un hilo (grupo.warp.carril) |
| `grupos` | Grupos que están corriendo en cada unidad |
| `memoria D [N]` | Memoria global |
| `compartida G [D N]` | Memoria compartida de un grupo en curso |
| `info` | Métricas hasta ahora |

`comparar-gpu` (RF-GPU-018) resuelve **el mismo problema** en la CPU y en la GPU. Ejecuta el programa
en la máquina y el kernel en la unidad, da los ciclos de cada una y compara los resultados:
`--resultado 0x4020:32:16` compara 16 palabras desde la dirección 0x4020 de la memoria de la CPU con
las que están desde la 32 de la memoria global de la GPU (código de salida 4 si difieren). Con la
suma de vectores de `ejemplos/gpu-16`:

```
CPU: 220 ciclos, 157 instrucciones
GPU: 63 ciclos, 16 hilos en 2 grupos (uso de carriles 100.0%)
la GPU tarda 3.49 veces menos
resultados: iguales en 16 palabras
```

La memoria de la GPU es aparte de la de la CPU (D-107); los datos iniciales de un kernel van en
`.dato`.

## Ejemplos

`ejemplos/gpu-16/` es `risc-16` con una unidad gráfica, más `OUT` e `IN` para los puertos:

| Archivo | Qué muestra |
| --- | --- |
| `programas/triangulo.s` | La CPU dibuja un triángulo mandando palabras por el puerto |
| `programas/figuras.gpu` | Guion con fondo, rectángulo, línea y triángulo interpolado, con profundidad |
| `kernels/suma-vectores.kgpu` | Un hilo por elemento; gana a un bucle de la CPU |
| `kernels/reduccion.kgpu` | Memoria compartida y barreras |
| `kernels/divergencia.kgpu` | Pares e impares siguen caminos distintos: mirar con `--traza` |
| `programas/suma-vectores.s` | La misma suma en la CPU, para `comparar-gpu` |

## Límites

- No hay visor gráfico en tiempo real todavía (RF-GPU-003): lo trae el IDE.
- Los dos modos no comparten estado: los kernels no dibujan en el framebuffer.
- La memoria de la GPU y la de la CPU son independientes; no hay copias entre ellas durante la
  ejecución.
- Un acceso a memoria cuesta lo mismo sea cual sea la dirección: no se modelan agrupaciones de
  accesos ni conflictos de bancos.
- La profundidad es una por primitiva; no hay interpolación de `z` entre vértices.
