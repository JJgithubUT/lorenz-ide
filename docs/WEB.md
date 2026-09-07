# Version web de Lorenz

`lorenz servir` ofrece el mismo IDE que `lorenz ide`, pero alojado: se abre en el navegador sin
instalar nada (RF-ECO-030). Cada visitante tiene su propio espacio de trabajo.

```
lorenz servir [--direccion HOST:PUERTO] [--datos CARPETA] [--segundos N]
```

## Que ofrece

- **Espacio propio y persistente (RF-ECO-031).** El servidor da una galleta `lorenz=<32 hex>`
  (`HttpOnly`, `SameSite=Lax`) y guarda el trabajo en `<datos>/espacios/<id>`. Sobrevive al cierre
  del navegador y al reinicio del servidor. Cada espacio nace con los ejemplos.
- **Exportar (RF-ECO-032).** `GET /api/exportar` devuelve un `.lpkg` para abrirlo con el escritorio.
- **Aviso de diferencias (RF-ECO-033).** Una banda lista lo que no hay frente al escritorio
  (`GET /api/modo`).
- **Compartir (RF-ECO-010, 011).** `POST /api/compartir` guarda una maquina y su programa con un
  identificador derivado del contenido (el mismo contenido da el mismo enlace). `/c/<id>` lo abre
  en `compartido/`, donde se ejecuta sin instalar nada.
- **Derivar (RF-ECO-012).** `POST /api/derivar` copia lo compartido al espacio propio, editable.
- **Galeria (RF-ECO-013).** `POST/GET /api/galeria`: titulo, descripcion y etiquetas; la busqueda
  ignora mayusculas y acentos.

## Modelo de seguridad

El servidor ejecuta programas de terceros, asi que todo es restrictivo por defecto:

| Riesgo | Medida |
| --- | --- |
| Salir del espacio | rutas validadas por `Ide::resolver` (tambien por enlaces) y filtro `ruta_fuera_del_espacio` sobre herencias, inclusiones e importaciones (`..`, `/`, `~`, `X:`) |
| Llenar el disco | cuotas: 300 archivos, 4 MiB en total, 512 KiB por archivo |
| Lanzar procesos o conexiones | las maquinas con dispositivos `externo` se rechazan |
| Alterar lo compartido | `compartido/` es de solo lectura |
| Peticiones desde otras paginas | se comprueba `Origin` en las peticiones que no son GET |
| Abuso | 900 peticiones por minuto y visitante (429) |
| Ejecuciones infinitas | las ejecuciones usan los limites de pasos del IDE |

`solo_locales` se activa solo si la direccion es de loopback.

## Despliegue

El servidor habla HTTP simple (D-115: sin TLS propio). Para publicarlo:

1. Pon delante un proxy inverso con HTTPS (Caddy, nginx) y reenvia a `127.0.0.1:PUERTO`.
2. Ejecuta `lorenz servir` en un contenedor **sin acceso a la red** y con un volumen para `--datos`.
3. Limita CPU y memoria del contenedor: la cuota de disco es de Lorenz, la de CPU no.

## Limitaciones

Dispositivos externos, terminal y orden `lorenz`, instalar paquetes de la red, y editar lo
compartido (se deriva una copia). El trabajo vive en el servidor: se exporta con `.lpkg`.

## Nucleo en WebAssembly (RF-ECO-029)

`crates/lorenz-wasm` empaqueta arquitectura, ensamblador y emulador como un modulo `.wasm` sin
dependencias de JavaScript (interfaz C minima: `lorenz_reservar`, `lorenz_ejecutar`,
`lorenz_liberar`). `scripts/compilar-wasm.sh` lo compila y lo deja en `crates/lorenz-wasm/web/`,
junto a `lorenz.js` (cargador), `index.html` (pagina de prueba) y `probar.mjs` (comprobacion con
Node).

Verificado en un equipo con Windows: el modulo pesa ~1 MB, ensambla y ejecuta `suma.s` del ejemplo
y explica los errores de ensamblado, ejecutado por el motor WebAssembly de Node 24. Limites: no
hay disco, asi que `.incluir` y la herencia de arquitecturas no funcionan dentro del modulo, y los
dispositivos externos no existen.
