# IDE de Lorenz (API y arquitectura)

`lorenz ide [carpeta] [--puerto N] [--segundos N]` sirve la carpeta (por defecto `.`) en
`http://127.0.0.1:N` (por defecto 8080). Es una aplicacion web local (D-113): no hay WASM ni
dependencias externas. El servidor (`lorenz-web`) usa solo la biblioteca estandar.

## Seguridad

- Solo escucha en `127.0.0.1` y rechaza peticiones cuyo `Host` no sea local (403), para que una
  pagina ajena no pueda atacarlo desde el navegador.
- Ninguna ruta puede salir de la carpeta del proyecto: se rechazan `..`, rutas absolutas, `\` y
  enlaces simbolicos que apunten fuera.
- Limites: cuerpo 4 MiB, cabeceras 16 KiB, 64 conexiones a la vez (503 si hay mas), 10 s por
  conexion, 16 sesiones de ejecucion. Un manejador que entra en panico devuelve 500 y el servidor
  sigue vivo.
- No funciona en modo seguro (`--seguro`).

## API

Todo es JSON; los errores salen como `{"error": "..."}`.

| Metodo y ruta | Hace |
|---|---|
| `GET /api/proyecto` | lista de archivos y carpetas |
| `GET /api/archivo?ruta=` | lee un archivo de texto |
| `PUT /api/archivo` `{ruta, texto}` | escribe (crea carpetas) |
| `DELETE /api/archivo?ruta=` | borra |
| `POST /api/ensamblar` `{maquina, fuente}` | ensambla; 422 con `errores` si falla |
| `POST /api/sesiones` `{maquina, programa}` | abre una sesion; devuelve `{id}` |
| `GET /api/sesiones` | ids de sesiones abiertas |
| `DELETE /api/sesiones/:id` | la cierra |
| `GET /api/sesiones/:id/estado` | pc, sp, registros, banderas, contadores, ubicacion |
| `POST /api/sesiones/:id/paso` `{n}` | ejecuta n instrucciones |
| `POST .../continuar` `{limite}` | sigue hasta una parada |
| `POST .../hasta` `{direccion}` | sigue hasta una direccion |
| `POST .../reiniciar` | vuelve al arranque |
| `POST .../grabar` `{activa}` · `atras` `{n}` · `adelante` `{n}` | depuracion hacia atras |
| `POST .../registro` `{nombre, valor}` · `memoria` `{direccion, valor}` · `puerto` | edita el estado |
| `GET .../memoria?desde=&n=` | celdas (`null` si no hay memoria) |
| `GET .../desensamblado?desde=&n=` | lineas con etiqueta, linea de fuente y `actual` |
| `GET/POST .../rupturas`, `DELETE .../rupturas/:id` | puntos de ruptura (con condicion) |
| `GET .../pantalla/:nombre` · `.../serie/:nombre` | pixeles de una pantalla; texto de una UART |

Las respuestas de `paso`, `continuar` y `hasta` traen `parada` (`tipo`: `paso`, `detenida`,
`ruptura`, `observacion`, `llegada`, `retorno`, `limite`, `fallo`, `condicion`) y el `estado`.

## Pruebas

`crates/lorenz-cli/tests/ide_api.rs` prueba la API contra un servidor real. `pruebas-ui/banco.py`
abre el IDE en Chromium (playwright) y la prueba `ide_ui.rs` lo lanza; se omite si no hay
playwright o Chromium.
