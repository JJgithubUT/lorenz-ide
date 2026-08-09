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
| `POST .../instante` `{n}` · `orden` `{linea}` | salta a un instante de la traza; ejecuta una orden del depurador de linea de comandos |
| `GET .../mapa?bloque=` | regiones de memoria con su ocupacion por bloques, PC y SP |
| `GET .../eventos` · `.../explicacion` | interrupciones atendidas; que hara la proxima instruccion |
| `POST /api/arquitectura/analizar` `{texto, ruta}` | bloques del diagrama y errores del cargador (con linea y bloque) |
| `POST /api/arquitectura/editar` `{texto, ruta, cambio}` | aplica `fijar`, `quitar` o `anadir` al texto sin perder comentarios; devuelve el texto nuevo |
| `POST /api/gpu` `{maquina, programa}` | abre una sesion grafica (la maquina necesita `[gpu]`) |
| `POST /api/gpu/:id/avanzar` `{n}` · `GET .../estado` | ejecuta n instrucciones de la CPU junto a la GPU; metricas |
| `GET /api/gpu/:id/imagen?ampliar=` · `.../pixel?x=&y=` | framebuffer como PNG; describe un pixel |

Las respuestas de `paso`, `continuar` y `hasta` traen `parada` (`tipo`: `paso`, `detenida`,
`ruptura`, `observacion`, `llegada`, `retorno`, `limite`, `fallo`, `condicion`) y el `estado`.

### Version alojada (`lorenz servir`)

| Metodo y ruta | Hace |
|---|---|
| `GET /api/modo` | si es alojado, cuotas y limitaciones |
| `GET /api/exportar` | el espacio como `.lpkg` |
| `POST /api/compartir` `{maquina, programa}` | enlace `/c/<id>` por contenido |
| `GET /c/:id` · `GET /api/compartido/:id` | abre lo compartido en `compartido/` |
| `POST /api/derivar` `{id, nombre}` | copia propia editable |
| `GET/POST /api/galeria`, `DELETE /api/galeria/:id`, `GET /api/galeria/etiquetas` | galeria con busqueda |

Ver `docs/WEB.md`.

## Pruebas

`crates/lorenz-cli/tests/ide_api.rs` prueba la API contra un servidor real. `pruebas-ui/banco.py`
abre el IDE en Chromium (playwright) y la prueba `ide_ui.rs` lo lanza; se omite si no hay
playwright o Chromium.

## La interfaz

`lorenz ide` sirve una pagina con paneles (todos se pueden mover entre las cuatro zonas, separar
como ventanas flotantes, plegar y reordenar; la disposicion se recuerda):

| Panel | Que hace |
|---|---|
| Archivos | arbol del proyecto y archivo nuevo |
| Editor | pestanas, resaltado segun la arquitectura activa, autocompletado, errores sobre la linea, codificacion bajo el cursor (pie), F12 o Ctrl+clic para ir a la definicion |
| Buscar y reemplazar | en todo el proyecto, con o sin distinguir mayusculas |
| Problemas | errores del ultimo compilado; Intro lleva a la linea |
| Registros | lamparas por bit (clic invierte un bit) o numeros; banderas; contadores |
| Desensamblado | instrucciones con puntos de ruptura (clic en el circulo o Espacio) y la actual marcada con ▶ |
| Memoria · Puntos de ruptura | volcado desde una direccion; lista de rupturas |
| Consola | salida de la UART como teletipo y entrada de texto |
| Pantalla | el framebuffer con efecto de monitor; curvatura, grano, parpadeo y texturas se apagan por separado |
| Documentacion | la de la arquitectura activa |
| Apariencia, sonido y accesibilidad | temas, tema propio por marcas, tamano de texto, alto contraste, sonido por familia y atajos |

### Atajos (reasignables)

| Accion | Atajo |
|---|---|
| Compilar | Ctrl+B |
| Ejecutar / continuar · Pausar | F5 · F6 |
| Paso · Paso atras | F10 · Mayus+F10 |
| Reiniciar | Ctrl+Mayus+F5 |
| Alternar ruptura en el PC | F9 |
| Ir a la definicion · Buscar | F12 · Ctrl+Mayus+F |
| Guardar | Ctrl+S |
| Panel siguiente / anterior | F8 · Mayus+F8 |

Toda la depuracion se puede hacer solo con teclado. Ningun estado depende solo del color: las
lamparas llevan ●/○, la linea actual ▶, las banderas ✓/✗, los registros cambiados Δ y los errores ✖.

### Trabajo que no se pierde

Antes de compilar o abrir una sesion se guardan los archivos abiertos. Mientras se escribe se
guarda un borrador en el navegador; si la pagina se cierra de golpe, al volver se ofrece
conservarlo o descartarlo. Las operaciones que tardan mas de un segundo muestran un indicador con
los segundos transcurridos.

### Por dentro

Los archivos de `crates/lorenz-cli/src/ide/web/` se incrustan en el binario; cada modulo registra
sus paneles en `L.paneles`. No hay dependencias ni paso de construccion. Las pruebas de
`pruebas-ui/banco.py` abren cada pantalla en Chromium (varias a la vez); `LORENZ_UI_SERIE=1` las
fuerza en serie.

## Depuracion avanzada

Paneles del depurador grafico: **Instante y velocidad** (control deslizante sobre la traza y
ejecucion lenta), **Camino del dato** (que mueve la proxima instruccion entre memoria, registros,
unidad aritmetica, PC y puertos; el rotulo lo dice con palabras), **Mapa de memoria**,
**Memoria** (hexadecimal, con celdas editables), **Linea de tiempo**, **Ordenes del depurador** y
**Unidad grafica**. `ide::paridad` lista cada orden de `lorenz depurar` con su equivalente grafico y
una prueba la compara con la ayuda del depurador.

## Editor visual de arquitectura

El panel *Editor visual de arquitectura* dibuja la descripcion abierta en el editor (cualquier
`.toml`) como bloques: registros, banderas, regiones de memoria y dispositivos. Se puede:

- **Anadir** un bloque pulsando o arrastrando desde la paleta hasta el diagrama.
- **Cambiar** un campo en el formulario del bloque elegido; **quitar** el bloque con su boton o con
  Supr; **mover** los bloques con el raton (la posicion se recuerda en el navegador).
- **Deshacer** el ultimo cambio hecho desde el diagrama.

El diagrama y el texto van a la par: escribir en el texto redibuja el diagrama y un cambio del
diagrama se escribe en el texto con `toml_edit`, de modo que los comentarios, el orden y la base de
los numeros (`0xFF`) se conservan. Los fallos del cargador se marcan en el bloque con ⚠ y su
descripcion; pulsar el fallo lleva a su linea. Las instrucciones se cuentan, pero se editan en el
texto, igual que lo que la descripcion hereda de otra.
