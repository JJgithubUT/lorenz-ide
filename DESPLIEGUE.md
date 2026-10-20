# Despliegue de Lorenz

Tres formas de ponerlo en marcha, de la mas simple a la mas expuesta:

| Forma | Para quien | Orden |
| --- | --- | --- |
| Escritorio | una persona, en su equipo | `lorenz ide` |
| Servicio web | un grupo, desde el navegador y sin instalar | `lorenz servir` |
| Modulo WebAssembly | incrustar el nucleo en una pagina | `scripts/compilar-wasm.sh` |

## 1. Escritorio (Windows, Linux, macOS)

Requisitos: Rust estable (`rustup`) y, en Windows, las Build Tools de Visual Studio con C++.

```
rustup toolchain install stable
cargo build --release
```

El ejecutable queda en `target/release/lorenz` (`lorenz.exe` en Windows). En Windows:
`winget install Rustlang.Rustup` instala `rustup`.

Abrir el IDE sobre una carpeta de proyecto:

```
target\release\lorenz.exe ide ejemplos --puerto 8080
```

Y abrir `http://127.0.0.1:8080/`. Solo escucha en `127.0.0.1`: nadie mas en la red lo ve. Para
dejarlo en segundo plano en PowerShell:
`Start-Process -WindowStyle Hidden .\target\release\lorenz.exe -ArgumentList 'ide','ejemplos','--puerto','8080'`.
Para detenerlo: `Stop-Process -Name lorenz`.

Comprobar la instalacion: `lorenz --version` y `cargo test --workspace` (las pruebas de la
interfaz se omiten solas si no hay Python con `playwright` y Chromium).

## 2. Servicio web (`lorenz servir`)

```
lorenz servir --direccion 127.0.0.1:8080 --datos /var/lib/lorenz
```

- `--direccion HOST:PUERTO`: donde escucha (por omision solo en loopback).
- `--datos CARPETA`: donde guarda los espacios, lo compartido y la galeria. Debe persistir.
- `--segundos N`: se detiene solo a los N segundos (pruebas).

Cada visitante recibe una galleta `lorenz` y su propio espacio en `<datos>/espacios/`. El modelo de
seguridad y las cuotas estan en `docs/WEB.md`.

### Lo que NO hace el servidor

No habla HTTPS ni autentica personas. Por eso nunca se expone directo a Internet: va detras de un
proxy inverso.

### Proxy inverso con HTTPS (Caddy)

```
lorenz.ejemplo.org {
    reverse_proxy 127.0.0.1:8080
}
```

Caddy obtiene y renueva el certificado solo. Con nginx, un `proxy_pass http://127.0.0.1:8080;`
mas el `Host` y `X-Forwarded-For` habituales; no cambies el encabezado `Origin`, que el servidor
comprueba en las peticiones que modifican datos.

### Contenedor sin red

El servidor ejecuta programas de terceros: aislalo.

```
FROM rust:1 AS build
WORKDIR /src
COPY . .
RUN cargo build --release -p lorenz-cli

FROM debian:stable-slim
COPY --from=build /src/target/release/lorenz /usr/local/bin/lorenz
RUN useradd -m lorenz && mkdir /datos && chown lorenz /datos
USER lorenz
VOLUME /datos
EXPOSE 8080
CMD ["lorenz", "servir", "--direccion", "0.0.0.0:8080", "--datos", "/datos"]
```

```
docker run -d --name lorenz --read-only --tmpfs /tmp \
  --cpus 1 --memory 512m --pids-limit 128 --cap-drop ALL \
  -v lorenz-datos:/datos -p 127.0.0.1:8080:8080 lorenz
```

Con `0.0.0.0` dentro del contenedor, el `-p 127.0.0.1:8080:8080` lo mantiene cerrado al exterior y
solo lo alcanza el proxy. Limita CPU y memoria: la cuota de disco la pone Lorenz, la de CPU no.

### Servicio de systemd (sin contenedor)

```
[Unit]
Description=Lorenz web
After=network.target

[Service]
User=lorenz
ExecStart=/usr/local/bin/lorenz servir --direccion 127.0.0.1:8080 --datos /var/lib/lorenz
Restart=on-failure
NoNewPrivileges=true
ProtectSystem=strict
ReadWritePaths=/var/lib/lorenz
PrivateTmp=true
MemoryMax=512M
CPUQuota=100%

[Install]
WantedBy=multi-user.target
```

### Lista antes de abrirlo al publico

- [ ] Delante hay HTTPS y el puerto de Lorenz no es alcanzable desde fuera.
- [ ] `--datos` esta en un volumen con copia de seguridad.
- [ ] CPU, memoria y procesos limitados; el servicio corre sin privilegios.
- [ ] Sin dispositivos externos ni red saliente para el proceso (los espacios rechazan maquinas
      con dispositivos `externo`, pero el aislamiento de red es del despliegue).
- [ ] Dices a quien lo usa que el espacio es por navegador: sin cuentas, se pierde al borrar la
      galleta; `Exportar` baja un `.lpkg`.

## 3. Modulo WebAssembly

```
rustup target add wasm32-unknown-unknown
scripts/compilar-wasm.sh
```

Deja `lorenz_wasm.wasm` (~1 MB) en `crates/lorenz-wasm/web/` junto a `lorenz.js` y `index.html`.
Se sirve con cualquier servidor estatico (`python -m http.server` dentro de esa carpeta). Prueba
sin navegador: `node crates/lorenz-wasm/web/probar.mjs lorenz_wasm.wasm maquina.toml programa.s`.

Limites: sin disco, no hay `.incluir` ni herencia de descripciones, ni dispositivos externos.

## Versiones y publicacion

El flujo de ramas esta en `docs/GIT-WORKFLOW.md`: `develop` -> `deploy` (sube la version en
`Cargo.toml`) -> `main` (tag anotado `vX.Y.Z`) -> de vuelta a `develop`.

Para desplegar una version: `git checkout vX.Y.Z`, `cargo build --release`, reemplaza el
ejecutable y reinicia el servicio. Para volver atras, el mismo procedimiento con el tag anterior:
los datos de `--datos` no cambian de formato entre versiones 0.x sin avisarlo en `CHANGELOG.md`.

## Respaldos

- Codigo: `git bundle create lorenz.bundle --all` guarda todo el historial en un archivo.
- Datos del servicio: copia `--datos` con el servicio detenido, o desde una instantanea del volumen.
- Proyectos de usuarios: cada quien puede exportar su espacio a `.lpkg` desde la interfaz.
