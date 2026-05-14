# Ejercicios y desafios

## Ejercicios

Un ejercicio es una carpeta con `ejercicio.toml` (id, titulo, nivel 1-5, enunciado y pistas),
`pruebas.toml` (las pruebas de `lorenz probar`, con `maquina` apuntando a la maquina) y
`solucion.s` (lo que escribe quien lo resuelve).

```
lorenz ejercicio listar ejercicios
lorenz ejercicio ver ejercicios/3-maximo [--pista N]
lorenz ejercicio verificar ejercicios/3-maximo [--solucion ARCHIVO]
```

`verificar` sale con 0 si todas las pruebas pasan y con 1 si no. Al resolverse muestra ciclos,
instrucciones y palabras del programa. El repositorio trae doce ejercicios para `risc-16`
(ver `ejercicios/README.md`), cada uno con una `referencia.s` que las pruebas aceptan.

## Desafios

Un desafio (`desafio.toml`) es un ejercicio abierto con una metrica: `ciclos`, `instrucciones` o
`palabras`. Menos es mejor.

```
lorenz desafio ver desafios/multiplicar-barato
lorenz desafio evaluar desafios/multiplicar-barato mi.s
lorenz desafio comparar desafios/multiplicar-barato a.s b.s c.s
```

Solo entran al ranking las soluciones que pasan todas las pruebas; las que empatan en la metrica
comparten puesto. `evaluar` sale con 1 si la solucion no resuelve el desafio.
