# Integracion continua con Lorenz

Satisface RF-TST-034 (ejemplo documentado), RF-TST-031 (codigos de salida), RF-TST-032 (JSON),
RF-TST-033 (sin entorno grafico), RF-TST-035 (semilla) y RF-TST-036 (historial).
El flujo completo esta en `ejemplos/ci/lorenz.yml`.

## Pasos recomendados

1. `lorenz probar pruebas.toml --historial --json --salida resultados.json`
   Ejecuta las pruebas, compara con `.lorenz/ultimo.json` y lo reescribe. Conserva la carpeta
   `.lorenz/` entre ejecuciones (cache de CI) para detectar regresiones.
2. `lorenz conformidad maquina.toml --json` comprueba que el emulador cumple la arquitectura.
3. `lorenz aleatorio maquina.toml --semilla 42 --programas 200` compara el emulador con la
   referencia independiente. Una divergencia se reduce sola; el informe indica la semilla
   (`semilla + k`) para reproducirla con `--semilla S+k --programas 1`.
4. `lorenz compatibilidad anterior.toml nueva.toml programa.s` avisa si un cambio de ISA
   deja inservibles binarios ya construidos.

## Codigos de salida

| Codigo | Significado |
|---|---|
| 0 | todo correcto |
| 1 | pruebas o casos fallan, divergencia o binario incompatible; tambien una regresion que rompe |
| 2 | uso incorrecto de la orden |
| 3 | limite de instrucciones superado |
| 4 | `comparar` encuentra diferencias |
| 5 | una prueba no se pudo preparar (no ensambla, falta un archivo) |
| 6 | una prueba es mas lenta que el umbral (`--umbral %`) respecto a la ejecucion anterior |

## Consejos

- Fija siempre la semilla en CI: el mismo commit debe dar el mismo informe.
- Sube los JSON como artefactos aunque el paso falle (`if: always()`).
- Usa `--hilos N` si el servidor comparte procesadores con otros trabajos.
