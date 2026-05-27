# Explicar una ejecucion

`lorenz explicar <maquina.toml> <programa> [--limite N] [--direccion A]` ejecuta el programa y
cuenta, instruccion a instruccion, lo que hace: la instruccion, la descripcion que la maquina da
de ella, los registros y banderas que cambian (con valor anterior y nuevo), las lecturas y
escrituras de memoria y los saltos. Por omision explica las primeras 40 instrucciones.

No usa ningun modelo de lenguaje: el texto sale de la descripcion de la maquina y de lo que la
ejecucion cambia de verdad, asi que el mismo programa da siempre la misma explicacion.

## Marca de texto generado

Todo lo que Lorenz escribe por si mismo (explicaciones, documentacion, modelos Verilog, circuitos
de Logisim, PDF, maquinas importadas) lleva la linea
`Generado por Lorenz <version> con <orden>. Se regenera: no lo edites a mano.`
