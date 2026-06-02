# Ejercicios guiados

Doce ejercicios sobre la maquina `risc-16` (`maquina.toml`), del 1 (el mas facil) al 5. Cada uno es
una carpeta:

| Archivo | Para que |
| --- | --- |
| `ejercicio.toml` | titulo, nivel, enunciado y pistas |
| `solucion.s` | **tu solucion**: viene con un esqueleto que solo detiene la maquina |
| `pruebas.toml` | las pruebas que deciden si esta resuelto |
| `referencia.s` | una solucion de ejemplo; mirala despues de intentarlo |

```
lorenz ejercicio listar ejercicios
lorenz ejercicio ver ejercicios/1-suma
lorenz ejercicio ver ejercicios/3-maximo --pista 1
lorenz ejercicio verificar ejercicios/1-suma
```

`verificar` ejecuta las pruebas sobre `solucion.s` y dice cuales fallan y por que. Con
`--solucion ARCHIVO` se prueba otro archivo. Un ejercicio resuelto muestra tambien sus ciclos,
instrucciones y el tamano del programa, que sirven para compararte con `lorenz desafio`.
