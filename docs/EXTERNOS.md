# Dispositivos externos y modo seguro

## Dispositivo externo

Un periferico que vive fuera del emulador, en otro proceso o en otra maquina, se declara con
`tipo = "externo"` y exactamente una de estas dos claves:

```toml
[[dispositivo]]
nombre  = "acumulador"
tipo    = "externo"
orden   = "python3 acumulador.py"      # un proceso: se le habla por entrada y salida estandar
puertos = { datos = 0x10 }

[[dispositivo]]
nombre    = "sensor"
tipo      = "externo"
direccion = "127.0.0.1:9000"           # un servidor TCP, host:puerto
puertos   = { datos = 0x20, estado = 0x21 }
```

`orden` se lanza (sin interprete de comandos: se parte por espacios) desde el directorio actual
la primera vez que el programa toca uno de sus puertos. `ejemplos/externo-8/` trae un ejemplo
completo.

### Protocolo

Texto, una orden por linea (terminada en `\n`) y una respuesta por linea. Los numeros van en
decimal; las respuestas a `LEER` tambien pueden ir en hexadecimal (`0x..`).

| El emulador envia | El dispositivo responde |
| --- | --- |
| `LEER <puerto>` | el valor (`42`, `0x2A` o `OK 42`) |
| `ESCRIBIR <puerto> <valor>` | `OK` |
| `FIN` | nada: el emulador termina y cierra el canal |

Una respuesta `ERR <texto>` hace fallar la instruccion con ese texto. Si el dispositivo no
responde en 2 segundos, cierra la conexion o no se puede lanzar, la ejecucion termina con un error
que nombra al dispositivo. Los valores se truncan al ancho de palabra de la maquina.

Un dispositivo externo no puede levantar interrupciones: solo contesta cuando se le pregunta.

## Modo seguro

`lorenz --seguro <orden> ...` (o la variable `LORENZ_SEGURO=1`) esta pensado para ejecutar
programas ajenos. En modo seguro:

- no se escribe ningun archivo: se rechazan `-o` y las demas opciones de salida a disco, y las
  ordenes que crean cosas (`nuevo`, `paquete`, `proyecto`, `tarea`, `exportar`, `importar`...);
- no se lanza ningun proceso ni se abre ninguna conexion: los dispositivos externos no arrancan;
- solo se leen los archivos que la orden nombra (y los que ellos incluyen).

Las ordenes permitidas son `ayuda`, `version`, `validar`, `comparar`, `ensamblar`, `compilar` y
`desensamblar` (a la salida estandar), `ejecutar` (sin volcados a archivo), `probar` (sin guardar
resultados), `documentar` (sin `-o` ni `--vigilar`), `ejercicio`, `desafio` y `plantillas`. El
limite de instrucciones del emulador sigue acotando la duracion.
