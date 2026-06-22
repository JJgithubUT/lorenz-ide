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


class Navegador:
    """Un Chromium con una pagina abierta en el IDE; falla si la pagina lanza errores de JS."""

    def __init__(self, pw, p, ancho=1400, alto=900):
        self.b = pw.chromium.launch()
        self.ctx = self.b.new_context(viewport={"width": ancho, "height": alto})
        self.pag = self.ctx.new_page()
        self.errores = []
        self.pag.on("pageerror", lambda e: self.errores.append(str(e)))
        self.pag.on("console", lambda m: self.errores.append(m.text) if m.type == "error" else None)
        self.url = p.url
        self.abrir()

    def abrir(self):
        self.pag.goto(self.url + "/")
        self.pag.wait_for_selector("body[data-listo='1']")

    def recargar(self):
        self.pag.reload()
        self.pag.wait_for_selector("body[data-listo='1']")

    def cerrar(self):
        try:
            assert self.errores == [], f"errores de la pagina: {self.errores}"
        finally:
            self.b.close()


def con_navegador(f):
    def envuelta(pw, p):
        n = Navegador(pw, p)
        try:
            f(n, p)
        finally:
            n.cerrar()
    envuelta.__name__ = f.__name__
    return envuelta


@con_navegador
def prueba_pagina_carga(n, p):
    assert "Lorenz" in n.pag.title(), n.pag.title()
    assert n.pag.locator("#panel-archivos li", has_text="p.s").count() == 1


@con_navegador
def prueba_api_desde_el_navegador(n, p):
    r = n.pag.evaluate("L.api('GET','/api/proyecto').then(r => r.datos)")
    assert "p.s" in [a["ruta"] for a in r["archivos"]], r
    r = n.pag.evaluate("L.api('POST','/api/sesiones',{maquina:'maquina.toml',programa:'p.s'}).then(r => r.datos)")
    assert isinstance(r["id"], int), r


def zona_de(n, panel):
    return n.pag.evaluate(f"document.getElementById('panel-{panel}').parentElement.id")


@con_navegador
def prueba_paneles_se_mueven_y_se_recuerdan(n, p):
    assert zona_de(n, "archivos") == "zona-izq"
    n.pag.click("#panel-archivos [data-accion='der']")
    assert zona_de(n, "archivos") == "zona-der"
    n.recargar()
    assert zona_de(n, "archivos") == "zona-der", "la disposicion no se recordo"
    n.pag.click("#panel-archivos [data-accion='separar']")
    assert "flotante" in n.pag.get_attribute("#panel-archivos", "class")
    n.recargar()
    assert "flotante" in n.pag.get_attribute("#panel-archivos", "class"), "el panel separado no se recordo"
    n.pag.click("#panel-archivos [data-accion='separar']")
    n.pag.click("#panel-archivos [data-accion='colapsar']")
    assert "colapsado" in n.pag.get_attribute("#panel-archivos", "class")
    n.pag.evaluate("L.paneles.restaurar()")
    assert zona_de(n, "archivos") == "zona-izq"


@con_navegador
def prueba_paneles_se_arrastran(n, p):
    caja = n.pag.locator("#panel-archivos > header h2").bounding_box()
    x, y = caja["x"] + 5, caja["y"] + 5
    n.pag.mouse.move(x, y)
    n.pag.mouse.down()
    n.pag.mouse.move(x + 20, y + 20, steps=5)  # empieza el arrastre y aparecen las zonas vacias
    n.pag.wait_for_selector("body.arrastrando")
    destino = n.pag.locator("#zona-der").bounding_box()
    n.pag.mouse.move(destino["x"] + 20, destino["y"] + 20, steps=10)
    n.pag.mouse.up()
    assert zona_de(n, "archivos") == "zona-der"


@con_navegador
def prueba_atajos_reasignables(n, p):
    n.pag.evaluate("window.pulsado = 0; L.atajos.registrar('paso', () => window.pulsado++)")
    n.pag.keyboard.press("F10")
    assert n.pag.evaluate("window.pulsado") == 1
    n.pag.evaluate("L.atajos.fijar('paso', 'Ctrl+Y')")
    n.pag.keyboard.press("F10")
    assert n.pag.evaluate("window.pulsado") == 1, "el atajo viejo debe dejar de funcionar"
    n.pag.keyboard.press("Control+y")
    assert n.pag.evaluate("window.pulsado") == 2
    n.recargar()
    assert n.pag.evaluate("L.atajos.de('paso')") == "Ctrl+Y", "no se recordo el atajo"
    n.pag.evaluate("L.atajos.fijar('ejecutar', 'Ctrl+Y')")
    assert n.pag.evaluate("L.atajos.de('paso')") == "", "un atajo no puede servir a dos acciones"


PRUEBAS = {f.__name__: f for f in [prueba_pagina_carga, prueba_api_desde_el_navegador, prueba_paneles_se_mueven_y_se_recuerdan, prueba_paneles_se_arrastran, prueba_atajos_reasignables]}


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
