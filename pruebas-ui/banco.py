"""Banco de pruebas de la interfaz del IDE con Chromium (playwright).

Uso: python3 pruebas-ui/banco.py <ruta-al-binario-lorenz> [prueba...]
Sale con 0 si todo pasa, 1 si algo falla y 77 si falta playwright o Chromium (prueba omitida).
"""
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request

OMITIDA = 77


def puerto_libre():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


class Proyecto:
    """Una carpeta temporal con una maquina y un programa, y un `lorenz ide` sirviéndola."""

    def __init__(self, binario, raiz_repo):
        self.dir = tempfile.mkdtemp(prefix="lorenz-ui-")
        shutil.copy(os.path.join(raiz_repo, "ejercicios", "maquina.toml"), self.dir)
        with open(os.path.join(self.dir, "p.s"), "w") as f:
            f.write("        LI    R1, 42\n        ADDI  R2, R1, 1\n        HLT\n")
        self.puerto = puerto_libre()
        self.proc = subprocess.Popen(
            [binario, "ide", self.dir, "--puerto", str(self.puerto), "--segundos", "120"],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )
        self.url = f"http://127.0.0.1:{self.puerto}"
        for _ in range(100):
            try:
                urllib.request.urlopen(self.url + "/api/proyecto", timeout=1).read()
                return
            except Exception:
                time.sleep(0.05)
        self.cerrar()
        raise RuntimeError("el servidor no arranco")

    def cerrar(self):
        self.proc.kill()
        self.proc.wait()
        shutil.rmtree(self.dir, ignore_errors=True)


def prueba_pagina_carga(pw, p):
    b = pw.chromium.launch()
    try:
        pag = b.new_page()
        errores = []
        pag.on("pageerror", lambda e: errores.append(str(e)))
        pag.goto(p.url + "/")
        assert "Lorenz" in pag.title(), pag.title()
        assert errores == [], errores
    finally:
        b.close()


def prueba_api_desde_el_navegador(pw, p):
    b = pw.chromium.launch()
    try:
        pag = b.new_page()
        pag.goto(p.url + "/")
        r = pag.evaluate("fetch('/api/proyecto').then(r => r.json())")
        rutas = [a["ruta"] for a in r["archivos"]]
        assert "p.s" in rutas, rutas
        r = pag.evaluate(
            "fetch('/api/sesiones',{method:'POST',body:JSON.stringify({maquina:'maquina.toml',programa:'p.s'})}).then(r => r.json())"
        )
        assert isinstance(r["id"], int), r
    finally:
        b.close()


PRUEBAS = {f.__name__: f for f in [prueba_pagina_carga, prueba_api_desde_el_navegador]}


def principal(argv):
    if len(argv) < 2:
        print(__doc__)
        return 2
    binario = os.path.abspath(argv[1])
    raiz_repo = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    try:
        from playwright.sync_api import sync_playwright
    except ImportError:
        print("omitida: falta playwright para Python")
        return OMITIDA
    elegidas = argv[2:] or list(PRUEBAS)
    fallos = 0
    with sync_playwright() as pw:
        try:
            pw.chromium.launch().close()
        except Exception as e:
            print(f"omitida: no se puede lanzar Chromium ({e})")
            return OMITIDA
        for nombre in elegidas:
            p = Proyecto(binario, raiz_repo)
            try:
                PRUEBAS[nombre](pw, p)
                print(f"ok    {nombre}")
            except Exception as e:  # noqa: BLE001
                fallos += 1
                print(f"FALLA {nombre}: {e}")
            finally:
                p.cerrar()
    return 1 if fallos else 0


if __name__ == "__main__":
    sys.exit(principal(sys.argv))
