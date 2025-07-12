# Flujo de trabajo en Git

Este documento es normativo: define las ramas, la convencion de mensajes y la granularidad
exigida a los commits del proyecto Lorenz.

## 1. Politica de granularidad

**El repositorio debe alcanzar un minimo de 1000 commits a lo largo del desarrollo.**

La cifra no es un adorno: obliga a que cada cambio quede registrado por separado y a que el
historial sirva como registro real de como se construyo el sistema. Un proyecto de esta escala
(340 requisitos funcionales, seis fases) llega a esa cifra de forma natural si se respetan estas
reglas:

1. **Un commit por cambio con sentido propio.** Si el mensaje necesita la palabra "y", casi siempre
   son dos commits.
2. **El codigo y su prueba van juntos** en el mismo commit; la prueba no se deja para despues.
3. **La documentacion de un cambio va en su propio commit**, salvo que sea una linea en la bitacora.
4. **Nunca un commit de "varios arreglos"** ni un volcado de trabajo de un dia entero.
5. **Cada requisito implementado cita su identificador** (`RF-ARQ-014`) en el cuerpo del mensaje,
   para que el historial sea rastreable contra la ERS.
6. Un commit debe dejar el arbol compilando. Si un cambio grande no cabe compilando, se parte en
   commits que si lo hagan.

### Reparto esperado por fase

| Fase | Contenido | Commits estimados |
| --- | --- | --- |
| 1 | Nucleo, formato de arquitectura, emulador funcional | 180 |
| 2 | Ensamblador, desensamblador, depurador de linea de comandos | 160 |
| 3 | IDE, depuracion grafica, pruebas | 240 |
| 4 | Lenguaje SGT y su compilador | 190 |
| 5 | Ciclo a ciclo, pipeline, cache, metricas | 130 |
| 6 | GPU, ecosistema, exportacion, version web | 150 |
| — | **Total** | **1050** |

## 2. Ramas permanentes

| Rama | Proposito | Quien escribe en ella |
| --- | --- | --- |
| `main` | Codigo publicado. Cada commit corresponde a una version etiquetada. | Solo fusiones desde `deploy` |
| `develop` | Integracion del trabajo en curso. Siempre compila y pasa las pruebas. | Solo fusiones desde ramas de trabajo |
| `deploy` | Preparacion de una version: congelacion, correcciones finales, numero de version. | Fusiones desde `develop` y correcciones propias |

Nadie hace commits directos sobre `main`. En `develop` solo se permiten commits directos para
corregir el propio flujo (por ejemplo, un error en la configuracion de integracion continua).

## 3. Ramas de trabajo

Toda la talacha ocurre en ramas que nacen de `develop` y vuelven a `develop`.

| Prefijo | Para que | Ejemplo |
| --- | --- | --- |
| `feat/` | Una funcionalidad nueva | `feat/arch-bit-dsl` |
| `fix/` | Correccion de un fallo | `fix/overlapping-regions-panic` |
| `refactor/` | Cambio interno sin alterar el comportamiento | `refactor/split-validator` |
| `test/` | Pruebas que no acompanan a una funcionalidad nueva | `test/conformance-flags` |
| `docs/` | Documentacion | `docs/formato-v0-2` |
| `chore/` | Dependencias, scripts, configuracion | `chore/bump-toml` |
| `spike/` | Exploracion que puede descartarse | `spike/wasm-core` |

Reglas:

- Una rama de trabajo cubre un requisito o un grupo pequeno de requisitos relacionados.
- El nombre va en minusculas, con guiones, sin acentos.
- Una rama de trabajo no vive mas de una semana: si crece, se parte.
- Se borra al fusionarse.

### Ramas de correccion urgente

`hotfix/` nace de `main`, corrige el fallo y se fusiona a `main` y a `develop` en el mismo acto.
Es la unica rama que no nace de `develop`.

## 4. Flujo

```
feat/xxx ──► develop ──► deploy ──► main
                 ▲                    │
                 └──── hotfix/xxx ◄───┘
```

1. Nace la rama desde `develop` actualizado.
2. Se trabaja con commits pequenos, cada uno compilando.
3. Se fusiona a `develop` sin aplastar los commits: el historial detallado es el objetivo del
   proyecto, no un ruido que convenga esconder.
4. Cuando `develop` reune una fase o un conjunto util, se fusiona a `deploy`.
5. En `deploy` se fija el numero de version, se actualiza el registro de cambios y se corrige lo
   que aparezca.
6. `deploy` se fusiona a `main` y se etiqueta.

## 5. Convencion de mensajes

```
<tipo>: <que cambia, en imperativo y en minuscula>

<por que, si no es evidente>

RF-ARQ-014, RF-ARQ-016
```

Tipos: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `perf`, `style`, `build`, `ci`, `revert`.

- La primera linea no pasa de 72 caracteres y no termina en punto.
- El cuerpo explica el motivo, no repite el diff.
- La ultima linea lista los requisitos de la ERS que el commit atiende.

Ejemplo:

```
feat: validar que los bits de cada palabra sumen el ancho declarado

Un desajuste aqui produce un ensamblador que escribe codigo invalido en
silencio, asi que el cargador debe rechazar la descripcion.

RF-ARQ-016
```

## 6. Etiquetas

Versionado semantico: `v0.1.0`. Mientras el formato de descripcion de arquitectura no se
estabilice, la version mayor permanece en 0 y un cambio incompatible sube la menor.

## 7. Que no entra al repositorio

Artefactos de compilacion, binarios generados, archivos de editor y cualquier credencial.
Ver `.gitignore`.
