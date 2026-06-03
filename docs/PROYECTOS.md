# Proyectos

Un proyecto agrupa lo que hace falta para trabajar con una o varias arquitecturas: su
configuracion, los fuentes, los dispositivos y las tareas. Vive en un archivo `proyecto.toml` en la
raiz de su carpeta. Las ordenes son `lorenz proyecto`, `lorenz tarea`, `lorenz nuevo`,
`lorenz plantillas` y `lorenz recientes`.

## El archivo `proyecto.toml`

```toml
[proyecto]
nombre = "suma-enlazada"       # letras, digitos, `-` y `_`
arquitectura = "ejemplo-8"     # la activa; se puede omitir si solo hay una
salida = "construccion"        # opcional; donde se escriben los programas
pruebas = "pruebas.toml"       # opcional; lo que ejecuta el paso `probar`

[[arquitectura]]               # una o mas (RF-IDE-002, RF-IDE-003)
nombre = "ejemplo-8"
ruta = "maquina.toml"

[[fuente]]                     # en orden de compilacion (RF-IDE-004)
ruta = "programas/principal.s"

[[fuente]]
ruta = "programas/datos.s"

[[dispositivo]]                # dispositivos conectados y su entrada (RF-IDE-005)
nombre = "teclado"
entrada = "hola"

[[tarea]]                      # tareas con nombre (RF-IDE-006)
nombre = "correr"
descripcion = "construye y ejecuta"
pasos = ["construir", "ejecutar"]
limite = 1000                  # instrucciones al ejecutar
hasta_el_limite = false        # true: llegar al limite no es un fallo
```

Todas las rutas son relativas a la carpeta del proyecto y no pueden salir de ella. Un campo que no
existe, un nombre repetido o una referencia a algo no declarado se rechazan con un mensaje que
dice cual es.

### Arquitecturas

Hay una **activa**. Si el proyecto declara varias, `arquitectura` en `[proyecto]` dice cual, y las
ordenes aceptan `--arquitectura NOMBRE` para probar otra sin tocar el archivo. Si solo hay una, es
la activa. Una arquitectura secundaria que no carga solo da un aviso; la activa, un error.

### Fuentes

Todos los fuentes son del mismo lenguaje.

- **`.s` (ensamblador).** Uno solo se ensambla directamente. Varios se ensamblan cada uno a un
  objeto y se enlazan en el orden dado, de modo que un fuente puede usar con `.externo` lo que
  define otro.
- **`.sgt`.** Los modulos se encuentran por `importar`; el **ultimo** fuente es el principal y es el
  que se compila.

### Dispositivos

Un dispositivo debe existir en la arquitectura activa. `entrada` solo se admite en teclados
(`--teclas`) y lineas serie (`--serie`), y viaja a `lorenz ejecutar` con la opcion que le toca.

### Tareas

Una tarea es una lista de pasos. Un paso es uno de los basicos o el nombre de otra tarea:

| Paso | Hace |
| --- | --- |
| `ensamblar` | construye el programa en `<salida>/<nombre>.hex` (ensambla, enlaza o compila segun los fuentes) |
| `ejecutar` | `lorenz ejecutar` del programa construido, con `limite` y la entrada de los dispositivos |
| `probar` | `lorenz probar` sobre el archivo de `pruebas` |

Una tarea no puede llamarse como un paso basico ni llegar a si misma por sus pasos (se avisa del
ciclo: `a -> b -> a`). Cada paso llama a la misma orden que escribiria el usuario, asi que una
tarea nunca hace algo distinto de lo que haria la linea de comandos; se detiene en el primer paso
que falla y devuelve su codigo de salida.

## Ordenes

```
lorenz proyecto <carpeta | proyecto.toml> [--arquitectura NOMBRE]
lorenz tarea <carpeta | proyecto.toml> [NOMBRE] [--arquitectura NOMBRE]
```

`proyecto` muestra el proyecto y lo comprueba (archivos, maquina, dispositivos). `tarea` sin nombre
lista las tareas; con nombre, la ejecuta. Ejemplo: `ejemplos/proyecto-8`.

## Plantillas

```
lorenz plantillas
lorenz nuevo <plantilla | carpeta-plantilla> <destino> [--nombre NOMBRE]
```

El programa trae tres plantillas (RF-IDE-007, RF-IDE-008):

| Plantilla | Contenido |
| --- | --- |
| `programa-simple` | un programa en ensamblador sobre una maquina de 8 bits, con pruebas |
| `so-minimo` | Lorenz OS: un sistema operativo minimo en SGT con pantalla, teclado y reloj |
| `arquitectura-nueva` | una arquitectura minima comentada para empezar a disenar la tuya |

El nombre del proyecto es el de la carpeta (limpiado) o el de `--nombre`, y tambien se pone a la
maquina. Una carpeta existente con el nombre de una plantilla tiene prioridad sobre la incluida.
`nuevo` tambien acepta una carpeta cualquiera con `maquina.toml` como plantilla (RF-ECO-042).

## Proyectos recientes

```
lorenz recientes [listar | abrir N | olvidar N|todos] [--en ARCHIVO]
```

`proyecto`, `tarea` y `nuevo` apuntan el proyecto en la lista (RF-IDE-009). Se guardan los ultimos
10, el mas reciente primero, en `recientes.toml` dentro de `$LORENZ_HOME` o, si no esta fijada, de
`~/.lorenz`. Un proyecto que ya no existe se marca como tal.
