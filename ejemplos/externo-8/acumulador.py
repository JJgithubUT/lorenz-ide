#!/usr/bin/env python3
"""Periferico externo de ejemplo: suma lo que se le escribe y devuelve el total al leer.

Protocolo (docs/EXTERNOS.md): una orden por linea por la entrada estandar, una respuesta por linea
por la salida estandar.
"""
import sys

total = 0
for linea in sys.stdin:
    partes = linea.split()
    if not partes or partes[0] == "FIN":
        break
    if partes[0] == "ESCRIBIR" and len(partes) == 3:
        total += int(partes[2])
        print("OK", flush=True)
    elif partes[0] == "LEER" and len(partes) == 2:
        print(total, flush=True)
    else:
        print("ERR orden desconocida: " + linea.strip(), flush=True)
