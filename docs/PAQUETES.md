# Paquetes

Un paquete reune una arquitectura, un dispositivo, una biblioteca o un proyecto completo en **un
solo archivo** `.lpkg`, con nombre, version y dependencias. El crate es `lorenz-paq`; la orden,
`lorenz paquete`.

## Manifiesto

Todo paquete lleva `paquete.toml` en su raiz:

```toml
[paquete]
nombre = "mi-cpu"            # minusculas, digitos y guiones; empieza por letra
version = "1.2.0"            # mayor.menor.parche
autor = "Ana"
licencia = "MIT"
tipo = "arquitectura"        # arquitectura | dispositivo | biblioteca | proyecto
descripcion = "Una CPU didactica de 8 bits"   # opcional
requiere_formato = "0.1"     # opcional: version del formato de arquitectura que exige

[dependencias]
base = "^1.0"
```

Un manifiesto con campos desconocidos, un nombre invalido o una dependencia de si mismo se rechaza.

### Requisitos de version

| Escritura | Significa |
| --- | --- |
| `1.2.3` o `=1.2.3` | exactamente esa version |
| `>=1.2`, `>1.2`, `<=1.2`, `<1.2` | comparacion |
| `^1.2` | compatible: `>=1.2.0` y menor que `2.0.0` (`^0.2` es `>=0.2.0` y menor que `0.3.0`) |
| `*` | cualquiera |

Las versiones que faltan se toman como cero (`1.2` es `1.2.0`).

## Formato del archivo

```
LORENZ-PAQUETE 1
ARCHIVO <ruta> <longitud> <sha256>
<bytes>
ARCHIVO ...
FIN <sha256 de todo lo anterior>
```

`paquete.toml` va primero y el resto ordenado por ruta, de modo que **el mismo contenido da siempre
los mismos bytes**. Al leer se comprueba la suma de cada archivo y la suma total (RF-ECO-008): un
byte alterado o un archivo truncado se rechazan. Las sumas son SHA-256, implementado en el propio
crate y validado con los vectores oficiales.

### Seguridad de las rutas

Se rechazan las rutas absolutas, las que contienen `..` o `\`, las vacias y las repetidas. Al
empaquetar una carpeta se ignoran `.git`, `target`, `.lorenz` y los `.lpkg`, y se rechazan los
enlaces simbolicos. Limites: 16 MiB por archivo y 64 MiB en total.

## Instalacion y dependencias

Un paquete se instala en `<almacen>/<nombre>/<version>/`. El almacen por omision es
`$LORENZ_PAQUETES` o `~/.lorenz/paquetes`; `--en` elige otro.

Para cada dependencia que no esta instalada, `instalar` busca en las carpetas dadas con
`--repositorio` los `.lpkg` que cumplen el requisito y elige **el de version mas alta** (D-110). Las
dependencias se instalan antes que el paquete, a su vez con las suyas. Se informa de un ciclo
(`a -> b -> a`) y de una dependencia que no se encuentra. La escritura se hace en una carpeta
temporal que luego se renombra, asi un fallo no deja un paquete a medias.

Si un paquete exige un `requiere_formato` mayor que el soportado (`0.1`), se instala igual pero con
un aviso (RF-ECO-007).

## Orden

```
lorenz paquete crear <carpeta> [-o ARCHIVO.lpkg]
lorenz paquete verificar <archivo.lpkg>
lorenz paquete info <archivo.lpkg>
lorenz paquete instalar <archivo.lpkg> [--en DIR] [--repositorio DIR]...
lorenz paquete listar [--en DIR]
```

Sale con 0 si todo fue bien, 1 si falla (archivo alterado, dependencia ausente, ciclo) y 2 si la
orden esta mal escrita.

## Fuera de alcance

Instalar desde un sitio web o un registro remoto (RF-ECO-004) y la firma de autores quedan para
cuando exista una red de distribucion; hoy los repositorios son carpetas locales.

## Instalar desde una direccion web

```
lorenz paquete instalar http://servidor/paquete.lpkg --sha256 HUELLA [--en DIR]
```

Lorenz trae su propio cliente HTTP (sin dependencias) y **no trae TLS**: no hay `https://`. Como el
canal no es de fiar, la huella SHA-256 del paquete es obligatoria desde la red y el paquete se
rechaza si no coincide; la huella la publica quien ofrece el paquete por un canal distinto (en su
pagina, en el commit...). Si el sitio solo ofrece HTTPS, baja el archivo con otro programa y
instalalo desde el disco (`--sha256` tambien comprueba un archivo local).

El cliente soporta `Content-Length`, `Transfer-Encoding: chunked`, el cierre de la conexion y hasta
cinco redirecciones a direcciones `http://`; acepta paquetes de hasta 32 MiB y espera 10 segundos
como maximo por cada operacion de red. En modo seguro (`--seguro`) no esta disponible.
