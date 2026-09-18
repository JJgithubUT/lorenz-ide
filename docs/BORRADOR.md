# Borrador de arquitectura desde texto

```
lorenz borrador <descripcion en palabras> [-o maquina.toml]
```

En el IDE: panel «Editor visual de arquitectura» → «Borrador desde una descripcion» (RF-ECO-045).

**No usa IA.** Lee la descripcion con reglas fijas, asi que el mismo texto da siempre el mismo
borrador y RF-ECO-043 (funcionar sin IA) se mantiene. Es un punto de partida: el resultado es una
descripcion normal que se edita a mano o con el diagrama.

## Que entiende

| Dice | Efecto |
| --- | --- |
| `16 bits`, `palabras de 8 bits` | bits de palabra (8, 16, 24 o 32) |
| `direcciones de 12 bits`, `memoria de 64 KiB` | bits de direccion (hasta 16) |
| `8 registros`, `registros A, B, C y D` | cantidad (potencia de dos que quepa) o nombres |
| `big endian`, `little endian` | orden de bytes |
| `llamada mini`, `nombre mini` | nombre de la maquina |
| `suma`, `resta`, `logicas`, `xor` | ADD, SUB, AND y OR, XOR |
| `saltos`, `bucle`, `condicional` | agrega JC |
| `pantalla` | pantalla de 64x32 (solo con palabras de 8 bits) |

Siempre incluye NOP, MOV, LDI, LD, ST, JMP, JZ, JNZ y HLT, con formato regular: 4 bits de
operacion y campos de registro del ancho justo.

## Supuestos

Lo que el texto no dijo se decide y se lista (en la salida de error de la orden, y en el panel):
bits por defecto, reparto de memoria (un octavo de ROM, el resto RAM), banderas Z y C. El borrador
solo se entrega si pasa la validacion del cargador.
