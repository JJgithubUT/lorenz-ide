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

## 2026-10-15 — Cadencia de 1000 commits en una semana

**Hecho:**

- `docs/GIT-WORKFLOW.md` gana la seccion 1.1: metas diarias y por hora, calendario de siete dias
  con acumulado al cierre de cada uno, las unidades que justifican un commit propio y lo que no se
  hace para subir el contador.
- `scripts/contador.sh` compara los commits de una rama contra el calendario.
- Programado el arranque del desarrollo para las 1:30 del 2026-10-16, hora de Ciudad de Mexico.

**Decidido:**

- **D-010** — La meta de 1000 commits se cumple en una semana de desarrollo, del 2026-10-16 al
  2026-10-22: 140 commits por dia como minimo, uno cada diez minutos de trabajo. (D-009 queda
  reservado para la decision sobre como se interpreta el campo `semantica`.)
- **D-011** — El calendario fija el ritmo, no el alcance. Si el trabajo de un dia queda corto, el
  alcance pasa al dia siguiente; el contador no se recupera inflandolo.
- **D-012** — Quedan prohibidos los commits vacios, los cambios de formato sin motivo y reescribir
  historial publicado para modificar el contador.

**Pendiente:**

- Revisar el reparto por fase de la seccion 1: sumaba 1050 pensando en un ritmo mas lento y no
  coincide con el calendario de la semana.

**Requisitos:** ninguno; proceso.

**Commits:** rama `docs/cadencia-mil-commits`.

**Estado del contador:** 20 de 1000 commits.

---

## 2026-10-15 — Nucleo de carga y validacion de arquitectura

**Hecho:**

- Workspace de cargo con el crate `lorenz-arch`, raiz del grafo de dependencias.
- Analizador del lenguaje de campos de bits: bits fijos, campos con nombre, relleno y separador de
  palabra, con mascara y valor fijo para comparar codificaciones entre si.
- Modelo en memoria de una maquina: registros con rol, banderas, instrucciones, operandos, costes
  en ciclos, regiones y dispositivos.
- Cargador de TOML en dos capas: estructuras crudas que reflejan el archivo y conversion al modelo
  validado.
- Validador con las diez reglas del formato. Acumula todos los fallos antes de rechazar.
- Maquina de ejemplo de 8 bits en `ejemplos/ejemplo-8/maquina.toml`, que carga y valida limpia.
- 74 pruebas en verde: 26 del analizador de bits, 21 de carga, 26 de validacion y 1 de documentacion.

**Decidido:**

- **D-005** — Los errores de validacion se acumulan y se devuelven juntos. Ver diez problemas de una
  vez ahorra diez ciclos de prueba y error.
- **D-006** — Cada regla de validacion lleva un codigo corto (`E-BITS`, `E-AMBIG`) y el requisito de
  la ERS que la exige, para que la salida sea rastreable al documento.
- **D-007** — Una codificacion invalida detiene la carga de inmediato, a diferencia de los fallos de
  validacion: sin codificacion no hay nada que validar.
- **D-008** — Una version mayor distinta del formato se rechaza; una menor distinta se acepta.

**Pendiente:**

- El analizador de la expresion de semantica: hoy se guarda como texto sin interpretar.
- Posiciones de linea en los fallos de validacion; el tipo `Posicion` ya existe pero el cargador
  todavia no la rellena.
- Herencia entre descripciones (RF-ARQ-035).

**Requisitos:** RF-ARQ-001 a 009, 011, 013 a 019, 024 a 032, 034, 037, 042.

**Commits:** rama `feat/workspace-skeleton`, 11 commits, fusionada a `develop`.

**Estado del contador:** 16 de 1000 commits.

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
