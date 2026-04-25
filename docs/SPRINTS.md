# Sprints

Cada sprint tiene un objetivo, unas historias con su criterio de hecho y un orden. Al cerrarlo se
anota aqui lo que quedo y lo que no, y la bitacora recoge las decisiones.

## Sprint 1 — Cierre de la fase 6 (sin IDE ni GPU)

**Fecha:** 2026-10-16. **Version que lo cierra:** 0.3.0.

**Objetivo:** que no quede ningun requisito opcional de la fase 6 sin hacer en el nucleo, el modelo
de ciclos y el lenguaje, de modo que lo unico pendiente sea el entorno grafico y la unidad grafica.

| Orden | Historia | Requisito | Criterio de hecho |
| --- | --- | --- | --- |
| 1 | Restricciones de diseno y variantes que las cumplan | RF-CIC-033 | Una descripcion declara limites (ciclos, palabras de codigo, uso del bus); `lorenz restricciones` los comprueba contra programas y `lorenz variantes` propone cambios de temporizacion que los cumplen. Con pruebas y documentado. |
| 2 | Varios nucleos sobre memoria compartida | RF-CIC-023 | `nucleos` en la descripcion de ciclos; cada nucleo ejecuta su programa, comparten la memoria y turnan el bus; el resultado es repetible. Metricas por nucleo. |
| 3 | Coherencia de cache entre nucleos | RF-CIC-024 | Una cache por nucleo con protocolo MSI; las escrituras invalidan las copias ajenas; se cuentan invalidaciones y transferencias. Un programa que comparte un dato da el mismo resultado con y sin cache. |
| 4 | Genericos sencillos sobre enteros | RF-SGT-021 | Una funcion puede declarar un parametro de tipo entero y se instancia por cada tipo con que se llama; el compilador lo rechaza con un mensaje claro cuando no hay tipo que cuadre. |
| 5 | Cierre | todos | Pruebas del workspace y clippy sin avisos, informe y respaldo al dia, version 0.3.0 en `main` con su etiqueta. |

**Fuera del sprint:** editor visual, depurador grafico, IDE y GPU (fases de interfaz).

**Reglas de trabajo:** las de `docs/GIT-WORKFLOW.md`. Una rama por historia desde `develop`,
fusion `--no-ff`, commits pequenos que compilan solos, cada historia deja su fila de
`docs/TRAZABILIDAD.md`, su entrada de bitacora y su seccion de documentacion.

**Riesgos:**

- La simulacion de varios nucleos obliga a que las memorias de los nucleos coincidan; se resuelve
  difundiendo cada escritura a los demas en el orden en que ocurre, lo que fija un modelo de memoria
  sencillo (consistencia secuencial).
- Los genericos tocan el analizador, el generador y el interprete de referencia; se limitan a
  funciones y a parametros de tipo entero.

## Registro de cierre

**Resultado:** las cinco historias se hicieron completas.

| Historia | Requisito | Estado | Decisiones |
| --- | --- | --- | --- |
| Restricciones y variantes | RF-CIC-033 | Hecha | D-089 a D-091 |
| Varios nucleos | RF-CIC-023 | Hecha | D-092 a D-094 |
| Coherencia de cache | RF-CIC-024 | Hecha | D-095 a D-097 |
| Genericos de SGT | RF-SGT-021 | Hecha | D-098 a D-100 |
| Cierre | todos | Hecha: version 0.3.0 | |

**Lo que sale del sprint:**

- Pruebas del workspace: 1907, todas verdes; clippy sin avisos.
- Cuatro requisitos de la ERS pasan a Hecho. Quedan solo los del IDE, los de la GPU y el control
  grafico de DEP-022.
- Un fallo propio encontrado y corregido: las pruebas de las ordenes nuevas escribian el mismo
  archivo temporal a la vez y a veces leian a medias; ahora se escribe una sola vez.

**Para el siguiente sprint:** el entorno grafico y la unidad grafica, o ampliar lo hecho (inferencia
de tipos en los genericos, MESI, propagar el DMA entre nucleos).

## Sprint 2 — Unidad gráfica

**Fecha:** 2026-10-16. **Versión que lo cierra:** 0.4.0.

**Objetivo:** que la unidad gráfica exista y se pueda usar sin entorno gráfico: dibujar, recibir
comandos de la CPU, ejecutar hilos en warps y compararse con la CPU.

| Orden | Historia | Requisitos | Criterio de hecho |
| --- | --- | --- | --- |
| 1 | Superficie y primitivas | RF-GPU-002, 005 a 009 | Framebuffer configurable; relleno, líneas, triángulos planos e interpolados, profundidad. |
| 2 | Volcado e inspección | RF-GPU-004, 022 | PNG y PPM ampliados; descripción de un pixel. |
| 3 | Comandos, cola y ciclos | RF-GPU-010 a 012, 017 | Comandos con codificación y guion; cola con profundidad; ciclos propios. |
| 4 | Acople a la CPU | RF-GPU-001, 010, 012, 021 | Puertos, estado e interrupción; sin `[gpu]` no hay costo. |
| 5 | Hilos en warps | RF-GPU-013 a 016 | Kernels, divergencia visible, memoria compartida, barreras. |
| 6 | Herramientas | RF-GPU-018 a 020 | Depurador por hilo, comparación CPU↔GPU, tres ejemplos. |
| 7 | Cierre | todos | Pruebas y clippy, documentación, versión 0.4.0 en `main`. |

**Fuera del sprint:** visor en tiempo real (RF-GPU-003) y todo el IDE.

**Riesgos:** unir el modo gráfico y el de hilos en una sola memoria (se aplaza, D-108); modelar la
reconvergencia con precisión de hardware (se simplifica, D-102).
