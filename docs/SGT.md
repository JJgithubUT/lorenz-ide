# SGT (Saggitarius)

SGT es el lenguaje de alto nivel de Lorenz. Un programa SGT se compila para la maquina que el
usuario describio en TOML: el compilador no conoce ninguna arquitectura, lee de la descripcion lo
que la maquina sabe hacer (RF-SGT-024). Si la maquina no puede ejecutar algo, el error lo dice en
terminos de la maquina, no del lenguaje (RF-SGT-035).

Todos los ejemplos de este documento se compilan y se ejecutan en las pruebas
(`crates/lorenz-sgt/tests/documentacion.rs`). Un ejemplo marcado con `sgt` funciona en `risc-16`
y en `sgt-16`; uno marcado con `sgt:sgt-16` solo en esa maquina. La linea `// espera: x = 5`
dice que valor debe tener la variable global `x` al terminar; `// error: texto` marca un ejemplo
que el compilador debe rechazar con ese texto.

## Usar el compilador

```
lorenz compilar maquina.toml programa.sgt -o programa.hex
lorenz ejecutar maquina.toml programa.hex
```

Opciones de `compilar`: `-O0` a `-O3` (nivel de optimizacion), `--asm ARCHIVO` (el ensamblador
generado, con la linea de SGT antes de cada grupo de instrucciones), `--lineas ARCHIVO` (que linea
de SGT origino cada tramo de codigo), `--costes ARCHIVO` (ciclos de cada funcion).
`lorenz convencion maquina.toml` escribe la convencion de llamada de esa maquina.

## Programa minimo

Todo programa tiene una funcion `principal` sin parametros. Al terminar, la maquina se detiene.

```sgt
var suma: u16;

funcion principal() {
    var i: u16 = 1;
    mientras i <= 10 {
        suma += i;
        i += 1;
    }
}
// espera: suma = 55
```

## Tipos (RF-SGT-001 a 006)

Los enteros se llaman `u` (sin signo) o `i` (con signo) seguidos de los bits. Los tipos que
existen salen del ancho de palabra de la maquina: en una de 16 bits son `u16`, `i16`, `u32`,
`i32`, `u64` e `i64`. Los de mas de una palabra se operan palabra a palabra (suma y resta con
acarreo, comparacion desde la palabra alta, desplazamientos, y multiplicar y dividir con rutinas de
la biblioteca); quedan fuera de ellos `segun`, `salida` y los punteros de varias palabras.

- `bool`: `verdadero` o `falso`.
- `*T`: puntero a `T`. `&x` da la direccion de `x` y `*p` lee o escribe lo apuntado. Sumar `n` a un
  puntero avanza `n` palabras (decision D-061), y `p[i]` equivale a `*(p + i)`.
- `[T; N]`: arreglo de `N` elementos; `N` se conoce al compilar.
- `estructura`: campos con nombre.

```sgt
estructura Punto { x: u16, y: u16 }

var puntos: [Punto; 3];
var total: u16;

funcion principal() {
    puntos[0].x = 4;
    puntos[2].y = 6;
    var p: *u16 = &puntos[0].x;
    *p += 1;
    total = puntos[0].x + puntos[2].y;
}
// espera: total = 11
```

Un campo puede fijar su posicion con `@ n` (palabras desde el inicio de la estructura).

## Constantes y variables

`const` se evalua al compilar. Una variable global puede ubicarse en una direccion concreta con
`@` (RF-SGT-017); el compilador comprueba que cae en una region y que no es de solo lectura si se
escribe (RF-SGT-028).

```sgt
const TAM: u16 = 4;
const DOBLE: u16 = TAM * 2;

var datos: [u16; TAM];
var r: u16;

funcion principal() {
    datos[TAM - 1] = DOBLE;
    r = datos[3];
}
// espera: r = 8
```

Una variable local sin valor inicial vale 0 (decision D-060). Una global sin valor inicial tambien.

## Funciones (RF-SGT-009)

Los parametros y el valor devuelto son enteros, booleanos o punteros. No hay recursion
(decision D-059): las variables locales viven en celdas fijas y el compilador la rechaza. La
convencion de llamada de cada maquina esta en `docs/sgt/`.

```sgt
var r: u16;

funcion cuadrado(x: u16) -> u16 {
    devolver x * x;
}

funcion principal() {
    r = cuadrado(7) + cuadrado(1);
}
// espera: r = 50
```

```sgt-error
funcion f(n: u16) -> u16 {
    devolver f(n - 1);
}
funcion principal() { f(3); }
// error: recursion
```

## Genericos sobre enteros (RF-SGT-021)

Una funcion puede declarar parametros de tipo entre `<` y `>`, y se llama diciendo con que tipos:

```sgt
var a: u16;
var b: i16;

funcion maximo<T>(x: T, y: T) -> T {
    si x > y { devolver x; }
    devolver y;
}

funcion principal() {
    a = maximo<u16>(7, 300);
    b = maximo<i16>(-5, 3);
}
// espera: a = 300
// espera: b = 3
```

El compilador **no infiere** los tipos: hay que escribirlos en cada llamada, `maximo<u16>(...)`.
Por cada combinacion de tipos que se usa genera una copia de la funcion con los parametros
sustituidos, llamada `maximo__u16`, `maximo__i16`... (se ven asi en el ensamblador y en los
informes de coste). Dos llamadas con los mismos tipos comparten la copia, y una funcion generica que
nadie llama no deja codigo. El parametro de tipo vale en los parametros, el retorno, las variables
locales (`var y: T`), las conversiones (`x como T`) y los punteros (`*T`). Puede haber varios
(`funcion f<A, B>(x: A) -> B`) y una generica puede llamar a otra (`sumar<T>(a, b)`).

Limites, a proposito para que sean sencillos:

- Solo se instancian con tipos enteros (`u16`, `i32`, `usize`...), y el ancho debe existir en la
  maquina: `f<u8>` falla en una maquina de 16 bits como lo haria `var x: u8`.
- Una funcion generica se llama desde el archivo donde esta definida; no se importa de otro modulo.
- No lleva atributos (`@interrupcion`).
- Como las demas funciones, no puede ser recursiva (decision D-059), y el error nombra la copia
  (`suma__u16 se llama a si misma`).
- `a < b > (c)` se lee como una llamada generica a `a` con el tipo `b`; para comparar, usa
  parentesis: `(a < b) > (c)`.
- Una generica solo se revisa al instanciarla: un error en su cuerpo sale la primera vez que se usa,
  en la linea del cuerpo.

```sgt-error
funcion f<T>(x: T) -> T { devolver x; }
funcion principal() { var a: u16 = f(1); }
// error: necesita que se diga con que tipos
```

## Control de flujo (RF-SGT-010, 011)

`si`/`sino`, `mientras`, `bucle` (infinito, se sale con `romper`), `continuar` y `segun`. Las
condiciones deben ser `bool`: compara con `!= 0` si quieres usar un entero.

```sgt
var pares: u16;
var impares: u16;
var segun_tres: u16;

funcion principal() {
    var i: u16 = 0;
    bucle {
        i += 1;
        si i > 10 { romper; }
        si i % 2 == 0 { pares += 1; continuar; }
        impares += 1;
    }
    segun i {
        11 => { segun_tres = 1; }
        otro => { segun_tres = 2; }
    }
}
// espera: pares = 5
// espera: impares = 5
// espera: segun_tres = 1
```

## Operadores y desbordamiento (RF-SGT-019, 020)

- Aritmeticos: `+ - * / %`. Dan la vuelta al desbordar: `65535 + 1` en `u16` es 0.
- Saturantes: `+|` y `-|` se quedan en el extremo del rango.
- Bits: `& | ^ ~ << >>`. Un desplazamiento de tantos bits como tiene el tipo (o mas) da 0, o -1 si es
  `>>` de un negativo con signo.
- Comparaciones y logicos: `== != < <= > >= && || !`. `&&` y `||` no evaluan el segundo operando si
  el primero ya decide.
- `como` cambia de tipo entero: `x como i16` reinterpreta o recorta el valor al ancho del tipo destino.
- Dividir entre cero detiene la maquina (decision D-062).

```sgt
var vuelta: u16;
var satura: u16;
var resta: u16;
var desp: u16;

funcion principal() {
    var m: u16 = 65535;
    vuelta = m + 1;
    satura = m +| 5;
    resta = 3 -| 10;
    desp = 1 << 20;
}
// espera: vuelta = 0
// espera: satura = 65535
// espera: resta = 0
// espera: desp = 0
```

Si la maquina no tiene instruccion para un operador, el compilador llama a una rutina escrita en
SGT (`__mul`, `__divu`...). La lista, por maquina, esta en `docs/sgt/`.

## Modulos (RF-SGT-018)

Un programa puede repartirse en archivos. `importar` nombra otro archivo (`nombre.sgt`, en la
carpeta del que importa) y los simbolos que se quieren usar de el:

```
importar geom { Punto, norma1, ESCALA };

var p: Punto;
var r: u16;

funcion principal() {
    p.x = 4;
    p.y = 5;
    r = norma1(&p) + ESCALA;
}
```

Se pueden importar funciones, constantes, variables globales y estructuras. Lo que no se importa
es privado del modulo: el programa no lo ve aunque exista, y puede tener otro simbolo con el mismo
nombre. Un modulo puede importar de otros; los ciclos son un error. Los nombres internos del
programa compilado llevan el nombre del modulo como prefijo (`geom__norma1`), que es lo que se ve
en la tabla de costes. Un modulo tambien puede llamar a la biblioteca de SGT (`mem_*`, `cad_*`).
Los errores citan el archivo y la linea del modulo donde estan.

## Textos y biblioteca (RF-SGT-041)

Un texto inicia un arreglo de enteros global: un caracter por elemento y un cero final. Debe sobrar
un elemento para ese cero. Hay seis funciones de biblioteca; solo entran en el programa las que se
usan.

| Funcion | Que hace |
| --- | --- |
| `mem_llenar(destino, valor, n)` | escribe `valor` en `n` palabras |
| `mem_copiar(destino, origen, n)` | copia `n` palabras (las zonas no deben solaparse) |
| `mem_comparar(a, b, n)` | `0` si son iguales, `-1` si el primero es menor, `1` si es mayor |
| `cad_longitud(t)` | caracteres antes del cero final |
| `cad_copiar(destino, origen)` | copia el texto con su cero y devuelve su longitud |
| `cad_comparar(a, b)` | como `mem_comparar`, hasta el cero final |

```sgt
var saludo: [u16; 8] = "hola";
var copia: [u16; 8];
var largo: u16;
var iguales: i16;

funcion principal() {
    largo = cad_copiar(&copia[0], &saludo[0]);
    iguales = cad_comparar(&copia[0], &saludo[0]);
}
// espera: largo = 4
// espera: iguales = 0
```

```sgt-error
var t: [u16; 3] = "hola";
funcion principal() { }
// error: solo tiene 3 elementos
```

## Puertos, registros y ensamblador (RF-SGT-012 a 014, 029)

`entrada(puerto)` y `salida(puerto, valor)` usan las instrucciones de E/S de la maquina; el puerto
debe estar declarado en su descripcion. `reg.NOMBRE` lee o escribe un registro de la maquina que el
compilador no use para sus propios valores. `asm("...")` inserta ensamblador; `{variable}` se
sustituye por la direccion de esa variable.

```sgt:sgt-16
var leido: u16;

funcion principal() {
    salida(0x10, 72);
    salida(0x10, 105);
    reg.SP = 0x5000;
    leido = reg.SP;
}
// espera: leido = 20480
```

```sgt-error
funcion principal() { salida(0x77, 1); }
// error: 0x77
```

## Interrupciones (RF-SGT-016)

`@interrupcion(n)` convierte una funcion sin parametros en el manejador de la linea `n`; la maquina
debe declarar un vector para esa linea. El compilador guarda y repone los registros, y vuelve con
la instruccion de retorno de interrupcion.

```sgt:sgt-16
var ticks: u16;

@interrupcion(0)
funcion reloj() { ticks += 1; }

funcion principal() {
    mientras ticks < 3 { }
}
// espera: ticks = 3
```

## Coste y presupuesto (RF-SGT-030, 031)

`--costes` lista los ciclos de peor caso de cada funcion, sumando lo que llama. Un bucle cuenta una
sola iteracion y se avisa. `@presupuesto(n)` hace fallar la compilacion si el peor caso pasa de `n`
ciclos.

```sgt
var r: u16;

@presupuesto(10000)
funcion principal() { r = 1; }
// espera: r = 1
```

```sgt-error
var r: u16;

@presupuesto(3)
funcion principal() { r = 1; r += 2; }
// error: presupuesto es de 3
```

## Optimizacion (RF-SGT-033, 034)

| Nivel | Que hace |
| --- | --- |
| `-O0` | nada |
| `-O1` | plegado de constantes, ramas con condicion conocida, codigo muerto (por omision) |
| `-O2` | ademas, identidades (`x + 0`, `x * 1`) y sentencias sin efecto |
| `-O3` | ademas, `x * 2^k` como desplazamiento y division y resto sin signo entre potencias de dos (ejecuta menos instrucciones; en maquinas sin desplazamientos puede ocupar algo mas) |

Ninguno cambia lo que el programa hace: nunca se elimina una division que pueda detener la maquina,
ni una llamada, ni una lectura de puerto.

## Banderas de la maquina (RF-SGT-015)

Una maquina con banderas (por ejemplo `sgt-8`, con `Z` y `C`) las expone como `bandera.NOMBRE`,
un `bool` que vale lo que la bandera tiene en ese momento. Sirve en cualquier condicion:

```
var acarreo: u8;
funcion principal() {
    var a: u8 = 200;
    var s: u8 = a + 100;
    si bandera.C { acarreo = 1; }
}
```

La bandera es la que dejo la ultima instruccion que la actualizo, asi que hay que leerla justo
despues de la operacion que interesa: entre una sentencia y otra el compilador solo mueve datos
(las cargas y los guardados no tocan las banderas), pero con `-O1` o mas una operacion constante
puede desaparecer y con ella el efecto sobre la bandera. Pedir una bandera que la maquina no tiene
es un error que lista las que si tiene. El interprete de referencia no las modela.

## Errores (RF-SGT-035, 036)

Cada diagnostico lleva archivo, linea y columna. Cuando la maquina no puede hacer algo, lo dice
con su nombre:

```sgt-error
var x: u32;
funcion principal() { segun x { 1 => { } otro => { } } }
// error: varias palabras
```

## Lo que falta

Lo que sigue pendiente de la ERS esta en `docs/TRAZABILIDAD.md`.

## Proyecto nuevo desde una plantilla (RF-ECO-042)

`lorenz nuevo ejemplos/so-ejemplo mi-so --nombre mi-so` copia la carpeta, cambia el nombre de la
maquina en `maquina.toml` y deja un proyecto listo para `lorenz compilar`.
