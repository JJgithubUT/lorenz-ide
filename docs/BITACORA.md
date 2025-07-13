# Bitacora de desarrollo

Registro de lo que se va haciendo en Lorenz. Se escribe en orden inverso: lo mas reciente arriba.
Cada entrada corresponde a una sesion de trabajo y cita los requisitos de la ERS atendidos.

Formato de una entrada:

```
## AAAA-MM-DD — titulo corto

**Hecho:** que quedo funcionando.
**Decidido:** decisiones tomadas, con su identificador D-NNN.
**Pendiente:** lo que queda abierto y por que.
**Requisitos:** los RF y RNF que este trabajo atiende.
**Commits:** rango o rama.
```

---

## 2026-10-15 — Arranque del repositorio

**Hecho:**

- Repositorio inicializado con las tres ramas permanentes: `main`, `develop` y `deploy`.
- Documentado el flujo de trabajo en `docs/GIT-WORKFLOW.md`, incluida la politica de granularidad
  que fija un minimo de 1000 commits para el proyecto.
- Creada esta bitacora.

**Decidido:**

- **D-001** — El nucleo se escribe en Rust. Motivo: un solo binario para linea de comandos y
  WebAssembly, y buen ecosistema para analizadores.
- **D-002** — La descripcion de arquitectura se escribe en TOML, con un lenguaje de campos de bits
  propio solo para la codificacion de instrucciones.
- **D-003** — La licencia del proyecto queda pendiente hasta antes de la primera publicacion.
- **D-004** — El historial no se aplasta al fusionar. El detalle del historial es un objetivo del
  proyecto, no ruido.

**Pendiente:**

- Elegir la licencia (D-003).
- Decidir si el editor visual de arquitectura escribe el TOML directamente o pasa por una
  representacion intermedia.

**Requisitos:** ninguno todavia; trabajo de infraestructura.

**Commits:** rama `main`, commits iniciales.
