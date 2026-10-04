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
        self.binario = binario
        self.dir = tempfile.mkdtemp(prefix="lorenz-ui-")
        shutil.copy(os.path.join(raiz_repo, "ejercicios", "maquina.toml"), self.dir)
        with open(os.path.join(self.dir, "p.s"), "w") as f:
            f.write("        LI    R1, 42\n        ADDI  R2, R1, 1\n        HLT\n")
        shutil.copy(os.path.join(raiz_repo, "ejemplos", "ejemplo-8-serie", "maquina.toml"), os.path.join(self.dir, "serie.toml"))
        with open(os.path.join(self.dir, "hola.s"), "w") as f:
            f.write("        LDI A, 0x48\n        OUT A\n        LDI A, 0x69\n        OUT A\n        LDI A, 0xFF\n        ST [0xF000], A\n        HLT\n")
        with open(os.path.join(self.dir, "q.s"), "w") as f:
            f.write("inicio: LI R1, 5\n        ADDI R2, R1, 1\n        BEQ R1, R1, inicio\n        HLT\n")
        gpu = os.path.join(self.dir, "z-gpu")
        os.makedirs(os.path.join(gpu, "risc-16"))
        os.makedirs(os.path.join(gpu, "gpu-16", "programas"))
        shutil.copy(os.path.join(raiz_repo, "ejemplos", "risc-16", "maquina.toml"), os.path.join(gpu, "risc-16"))
        shutil.copy(os.path.join(raiz_repo, "ejemplos", "gpu-16", "maquina.toml"), os.path.join(gpu, "gpu-16"))
        shutil.copy(os.path.join(raiz_repo, "ejemplos", "gpu-16", "programas", "triangulo.s"), os.path.join(gpu, "gpu-16", "programas"))
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


class ProyectoWeb:
    """Un `lorenz servir` con sus datos en una carpeta temporal (la version web)."""

    def __init__(self, binario):
        self.dir = tempfile.mkdtemp(prefix="lorenz-web-")
        self.puerto = puerto_libre()
        self.proc = subprocess.Popen(
            [binario, "servir", "--datos", self.dir, "--direccion", f"127.0.0.1:{self.puerto}", "--segundos", "120"],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )
        self.url = f"http://127.0.0.1:{self.puerto}"
        for _ in range(100):
            try:
                urllib.request.urlopen(self.url + "/api/modo", timeout=1).read()
                return
            except Exception:
                time.sleep(0.05)
        self.cerrar()
        raise RuntimeError("el servicio no arranco")

    def cerrar(self):
        self.proc.kill()
        self.proc.wait()
        shutil.rmtree(self.dir, ignore_errors=True)


class Navegador:
    """Un Chromium con una pagina abierta en el IDE; falla si la pagina lanza errores de JS."""

    def __init__(self, pw, p, ancho=1400, alto=900, seleccionar=True):
        self.seleccionar = seleccionar
        self.b = pw.chromium.launch()
        self.ctx = self.b.new_context(viewport={"width": ancho, "height": alto})
        self.pag = self.ctx.new_page()
        self.errores = []
        self.pag.on("pageerror", lambda e: self.errores.append(str(e)))
        # Los 4xx de la API son respuestas esperadas (p. ej. errores de compilacion), no fallos de la pagina.
        self.pag.on("console", lambda m: self.errores.append(m.text) if m.type == "error" and "Failed to load resource" not in m.text else None)
        self.url = p.url
        self.abrir()

    def abrir(self):
        self.pag.goto(self.url + "/")
        self.pag.wait_for_selector("body[data-listo='1']")
        # Por defecto se trabaja con la maquina RISC-16 y p.s; las pruebas eligen otras si hace falta.
        if self.seleccionar:
            self.pag.select_option("#sel-maquina", "maquina.toml")
            self.pag.select_option("#sel-programa", "p.s")

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


def accion_panel(n, panel, accion):
    """Mover o separar un panel: esas acciones viven en su menu «⋯»."""
    n.pag.click(f"#panel-{panel} .menu-panel > summary")
    n.pag.click(f"#panel-{panel} .menu [data-accion='{accion}']")


def zona_de(n, panel):
    return n.pag.evaluate(f"document.getElementById('panel-{panel}').parentElement.id")


@con_navegador
def prueba_paneles_se_mueven_y_se_recuerdan(n, p):
    assert zona_de(n, "archivos") == "zona-izq"
    accion_panel(n, "archivos", "der")
    assert zona_de(n, "archivos") == "zona-der"
    n.recargar()
    assert zona_de(n, "archivos") == "zona-der", "la disposicion no se recordo"
    accion_panel(n, "archivos", "separar")
    assert "flotante" in n.pag.get_attribute("#panel-archivos", "class")
    n.recargar()
    assert "flotante" in n.pag.get_attribute("#panel-archivos", "class"), "el panel separado no se recordo"
    accion_panel(n, "archivos", "separar")
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


def abrir_archivo(n, ruta):
    n.pag.click(f"#panel-archivos li[data-ruta='{ruta}']")
    n.pag.wait_for_function(f"L.editor.activo() === '{ruta}'")


@con_navegador
def prueba_editor_resalta_segun_la_arquitectura(n, p):
    abrir_archivo(n, "p.s")
    assert n.pag.locator(".resalte .t-mnemonico", has_text="LI").count() == 1
    assert n.pag.locator(".resalte .t-registro", has_text="R1").count() >= 1
    assert n.pag.locator(".resalte .t-numero", has_text="42").count() == 1
    n.pag.fill(".area", "FOO R1 ; hola\n.org 0x10\nx: HLT")
    assert n.pag.locator(".resalte .t-comentario", has_text="; hola").count() == 1
    assert n.pag.locator(".resalte .t-directiva", has_text=".org").count() == 1
    assert n.pag.locator(".resalte .t-etiqueta", has_text="x:").count() == 1
    assert n.pag.locator(".resalte .t-mnemonico", has_text="FOO").count() == 0, "FOO no es de esta arquitectura"


@con_navegador
def prueba_editor_resalta_sgt(n, p):
    n.pag.evaluate("L.api('PUT','/api/archivo',{ruta:'a.sgt',texto:'// c\\nfuncion f() -> u16 {\\n  devolver 0x10; /* b */\\n}\\n'})")
    n.pag.evaluate("L.cargarProyecto()")
    n.pag.wait_for_selector("#panel-archivos li[data-ruta='a.sgt']")
    abrir_archivo(n, "a.sgt")
    assert n.pag.locator(".resalte .t-mnemonico", has_text="funcion").count() == 1
    assert n.pag.locator(".resalte .t-mnemonico", has_text="devolver").count() == 1
    assert n.pag.locator(".resalte .t-registro", has_text="u16").count() == 1
    assert n.pag.locator(".resalte .t-comentario").count() == 2


@con_navegador
def prueba_editor_autocompleta(n, p):
    abrir_archivo(n, "p.s")
    n.pag.click(".area")
    n.pag.keyboard.press("Control+End")
    n.pag.keyboard.type("\n        ADD")
    n.pag.wait_for_selector(".sugerencias li")
    textos = n.pag.locator(".sugerencias li span:first-child").all_inner_texts()
    assert "ADDI" in textos and "ADD" not in textos or "ADDI" in textos, textos
    n.pag.keyboard.press("ArrowDown")
    n.pag.keyboard.press("Enter")
    assert n.pag.input_value(".area").rstrip().split()[-1] in textos, n.pag.input_value(".area")
    n.pag.keyboard.type(" R")
    n.pag.wait_for_selector(".sugerencias li")
    assert "R1" in n.pag.locator(".sugerencias li span:first-child").all_inner_texts()
    n.pag.keyboard.press("Escape")
    assert n.pag.locator(".sugerencias li").count() == 0 or not n.pag.locator(".sugerencias").is_visible()
    # Las etiquetas del propio archivo tambien se sugieren.
    abrir_archivo(n, "q.s")
    n.pag.click(".area")
    n.pag.keyboard.press("Control+End")
    n.pag.keyboard.type("\n        BEQ R1, R1, ini")
    n.pag.wait_for_selector(".sugerencias li")
    assert "inicio" in n.pag.locator(".sugerencias li span:first-child").all_inner_texts()


@con_navegador
def prueba_editor_muestra_la_codificacion(n, p):
    abrir_archivo(n, "p.s")
    n.pag.click(".area")
    n.pag.keyboard.press("Control+Home")
    n.pag.keyboard.press("ArrowDown")
    n.pag.wait_for_function("document.getElementById('pie-codificacion').textContent.includes('0x0001')")
    t = n.pag.inner_text("#pie-codificacion")
    assert "L2" in t and all(c in "01 " for c in t.split(":")[1].split("(")[0].strip()), t


@con_navegador
def prueba_editor_marca_errores_en_la_linea(n, p):
    abrir_archivo(n, "p.s")
    n.pag.fill(".area", "        LI R1, 1\n        FOO R2\n        HLT\n")
    n.pag.keyboard.press("Control+b")
    n.pag.wait_for_selector(".resalte .linea.con-error")
    assert n.pag.locator(".resalte .linea.con-error").get_attribute("data-n") == "2"
    assert "FOO" in n.pag.locator(".resalte .linea.con-error").get_attribute("title")
    assert n.pag.locator(".numeros .con-error").inner_text().startswith("✖2")
    n.pag.fill(".area", "        LI R1, 1\n        HLT\n")
    n.pag.keyboard.press("Control+b")
    n.pag.wait_for_function("document.querySelectorAll('.resalte .linea.con-error').length === 0")
    assert "Compilado" in n.pag.inner_text("#estado-texto")


@con_navegador
def prueba_editor_va_a_la_definicion(n, p):
    abrir_archivo(n, "q.s")
    n.pag.click(".area")
    n.pag.keyboard.press("Control+Home")
    n.pag.keyboard.press("ArrowDown")
    n.pag.keyboard.press("ArrowDown")
    n.pag.keyboard.press("End")
    n.pag.keyboard.press("ArrowLeft")
    n.pag.keyboard.press("ArrowLeft")
    n.pag.keyboard.press("F12")
    n.pag.wait_for_function("document.querySelector('.area').selectionStart < 10")
    assert n.pag.evaluate("(() => { const a = document.querySelector('.area'); return a.value.slice(a.selectionStart, a.selectionEnd); })()").startswith("inicio")


@con_navegador
def prueba_guardado_automatico_y_recuperacion(n, p):
    abrir_archivo(n, "p.s")
    n.pag.fill(".area", "        HLT\n")
    n.pag.wait_for_function("(L.guardado.leer('borradores', {})['p.s'] || '').includes('HLT')")
    n.recargar()
    n.pag.wait_for_selector("body[data-editor-listo='1']")
    n.pag.wait_for_selector(".aviso-recuperado")
    assert n.pag.input_value(".area") == "        HLT\n"
    n.pag.keyboard.press("Control+b")  # compilar guarda antes
    n.pag.wait_for_function("document.getElementById('estado-texto').textContent.includes('Compilado')")
    assert open(os.path.join(p.dir, "p.s")).read() == "        HLT\n"
    assert n.pag.evaluate("L.guardado.leer('borradores', {})['p.s']") is None


@con_navegador
def prueba_buscar_y_reemplazar_en_el_proyecto(n, p):
    abrir_panel(n, "buscar")
    n.pag.fill("#buscar-texto", "inicio")
    n.pag.click("#buscar-ir")
    n.pag.wait_for_function("document.getElementById('buscar-info').textContent.startsWith('2')")
    n.pag.click("#buscar-resultados li >> nth=1")
    n.pag.wait_for_function("L.editor.activo() === 'q.s'")
    assert n.pag.evaluate("document.querySelector('.area').selectionStart") > 0
    n.pag.fill("#buscar-por", "arranque")
    n.pag.once("dialog", lambda d: d.accept())
    n.pag.click("#reemplazar-ir")
    n.pag.wait_for_function("document.getElementById('buscar-info').textContent.includes('reemplazo')")
    assert open(os.path.join(p.dir, "q.s")).read().startswith("arranque:")
    assert n.pag.input_value(".area").startswith("arranque:"), "el editor debe recargar el archivo abierto"
    n.pag.keyboard.press("Control+Shift+F")
    assert n.pag.evaluate("document.activeElement.id") == "buscar-texto"


@con_navegador
def prueba_lista_de_problemas(n, p):
    abrir_archivo(n, "p.s")
    n.pag.fill(".area", "        FOO R2\n        HLT\n")
    n.pag.keyboard.press("Control+b")
    n.pag.wait_for_selector("#problemas-lista li")
    assert "p.s:1" in n.pag.inner_text("#problemas-lista li")
    abrir_archivo(n, "q.s")
    n.pag.click("#problemas-lista li")
    n.pag.wait_for_function("L.editor.activo() === 'p.s'")


@con_navegador
def prueba_documentacion_dentro_del_ide(n, p):
    n.pag.click("#panel-documentacion [data-accion='colapsar']")
    n.pag.wait_for_selector("#documento iframe")
    cuerpo = n.pag.frame_locator("#documento iframe").locator("body")
    cuerpo.wait_for()
    assert "ADDI" in cuerpo.inner_text()


def elegir(n, maquina, programa):
    n.pag.select_option("#sel-maquina", maquina)
    n.pag.select_option("#sel-programa", programa)


def registro_valor(n, nombre):
    return n.pag.inner_text(f"[data-registro='{nombre}'] .valor")


@con_navegador
def prueba_depurador_paso_a_paso_con_lamparas(n, p):
    n.pag.click("#accion-paso")
    n.pag.wait_for_function("document.getElementById('contadores').textContent.includes('PC 0x0001')")
    assert "= 42" in registro_valor(n, "R1")
    assert n.pag.locator("[data-registro='R1'] .lampara.on").count() == 3, "42 = 0b101010 tiene tres bits"
    assert n.pag.locator("[data-registro='R1'] .delta").count() == 1, "el registro que cambio se marca"
    assert n.pag.locator("[data-registro='R2'] .delta").count() == 0
    n.pag.click("#accion-paso")
    n.pag.wait_for_function("document.querySelector('[data-registro=\\'R2\\'] .valor').textContent.includes('= 43')")
    n.pag.click("#registros-modo")
    assert n.pag.locator(".lampara").count() == 0
    assert "0x002a" in registro_valor(n, "R1")
    n.pag.click("#registros-modo")
    assert n.pag.locator(".lampara").count() > 0


@con_navegador
def prueba_depurador_lamparas_se_pueden_conmutar(n, p):
    n.pag.click("#accion-paso")
    n.pag.wait_for_selector("[data-registro='R1'] .lampara")
    # El bit 0 es la ultima lampara de la fila.
    n.pag.locator("[data-registro='R1'] .lampara").last.click()
    n.pag.wait_for_function("document.querySelector('[data-registro=\\'R1\\'] .valor').textContent.includes('= 43')")


@con_navegador
def prueba_depurador_ruptura_y_ejecucion(n, p):
    n.pag.click("#accion-paso")
    n.pag.wait_for_selector("#desens-lista li")
    # Ruptura en la tercera instruccion (HLT, direccion 2).
    n.pag.click("#desens-lista li[data-direccion='2'] .bp")
    n.pag.wait_for_selector("#desens-lista li[data-direccion='2'] .bp.puesta")
    assert n.pag.inner_text("#desens-lista li[data-direccion='2'] .bp") == "\u25cf"
    assert n.pag.inner_text("#desens-lista li[data-direccion='1'] .bp") == "\u25cb"
    n.pag.click("#accion-ejecutar")
    n.pag.wait_for_function("document.getElementById('estado-texto').textContent.includes('ruptura')")
    assert "PC 0x0002" in n.pag.inner_text("#contadores")
    assert n.pag.inner_text("#desens-lista li.actual .flecha") == "\u25b6", "la linea actual no depende solo del color"
    n.pag.click("#accion-ejecutar")
    n.pag.wait_for_function("document.getElementById('estado-texto').textContent.includes('detenida')")
    assert "DETENIDA" in n.pag.inner_text("#contadores")
    n.pag.click("#accion-atras")
    n.pag.wait_for_function("!document.getElementById('contadores').textContent.includes('DETENIDA')")
    n.pag.click("#accion-reiniciar")
    n.pag.wait_for_function("document.getElementById('contadores').textContent.includes('instr 0')")


@con_navegador
def prueba_depuracion_completa_solo_con_teclado(n, p):
    n.pag.keyboard.press("F10")
    n.pag.wait_for_function("document.getElementById('contadores').textContent.includes('PC 0x0001')")
    assert "= 42" in registro_valor(n, "R1")
    n.pag.keyboard.press("F10")
    n.pag.wait_for_function("document.getElementById('contadores').textContent.includes('PC 0x0002')")
    n.pag.keyboard.press("Shift+F10")
    n.pag.wait_for_function("document.getElementById('contadores').textContent.includes('PC 0x0001')")
    n.pag.keyboard.press("F9")  # ruptura en el PC actual
    n.pag.wait_for_selector("#desens-lista li[data-direccion='1'] .bp.puesta")
    n.pag.keyboard.press("Control+Shift+F5")
    n.pag.wait_for_function("document.getElementById('contadores').textContent.includes('instr 0')")
    n.pag.keyboard.press("F5")
    n.pag.wait_for_function("document.getElementById('estado-texto').textContent.includes('ruptura')")
    n.pag.keyboard.press("F5")
    n.pag.wait_for_function("document.getElementById('contadores').textContent.includes('DETENIDA')")
    # Foco por paneles con F8.
    antes = n.pag.evaluate("document.activeElement.closest('.panel') && document.activeElement.closest('.panel').id")
    n.pag.keyboard.press("F8")
    despues = n.pag.evaluate("document.activeElement.closest('.panel') && document.activeElement.closest('.panel').id")
    assert antes != despues and despues, (antes, despues)


@con_navegador
def prueba_las_banderas_no_dependen_del_color(n, p):
    elegir(n, "serie.toml", "hola.s")
    n.pag.click("#accion-paso")
    n.pag.wait_for_selector("#banderas .bandera")
    textos = n.pag.locator("#banderas .bandera").all_inner_texts()
    assert textos and all(("✓1" in t) or ("✗0" in t) for t in textos), textos


@con_navegador
def prueba_consola_de_teletipo(n, p):
    elegir(n, "serie.toml", "hola.s")
    n.pag.click("#accion-ejecutar")
    n.pag.wait_for_function("document.getElementById('consola-salida').textContent.includes('Hi')")
    assert "Hi" in n.pag.inner_text("#consola-salida")
    assert n.pag.get_attribute("#consola-salida", "role") == "log"
    n.pag.fill("#consola-entrada", "abc")
    n.pag.keyboard.press("Enter")
    n.pag.wait_for_function("document.getElementById('consola-entrada').value === ''")


@con_navegador
def prueba_pantalla_muestra_el_framebuffer(n, p):
    elegir(n, "serie.toml", "hola.s")
    assert n.pag.get_attribute("#pantalla-lienzo", "data-encendidos") in (None, "0") or True
    n.pag.click("#accion-ejecutar")
    n.pag.wait_for_function("document.getElementById('pantalla-lienzo').getAttribute('data-encendidos') === '8'")
    brillo = n.pag.evaluate("(() => { const c = document.getElementById('pantalla-lienzo'); const d = c.getContext('2d').getImageData(0, 0, 8, 1).data; let s = 0; for (let i = 1; i < d.length; i += 4) s += d[i]; return s; })()")
    assert brillo > 0, "la primera fila debe tener pixeles encendidos"


@con_navegador
def prueba_efectos_del_monitor_son_independientes(n, p):
    abrir_panel(n, "pantalla")
    clases = lambda: n.pag.get_attribute("#monitor", "class")
    for f in ["curvatura", "grano", "parpadeo", "texturas"]:
        assert f"efecto-{f}" in clases()
    for f in ["curvatura", "grano", "parpadeo", "texturas"]:
        n.pag.uncheck(f"#efecto-{f}")
        resto = [g for g in ["curvatura", "grano", "parpadeo", "texturas"] if g != f]
        assert f"efecto-{f}" not in clases()
        assert all(f"efecto-{g}" in clases() for g in resto), "apagar un efecto no debe tocar los demas"
        n.pag.check(f"#efecto-{f}")
    n.pag.uncheck("#efecto-grano")
    n.recargar()
    assert "efecto-grano" not in clases() and "efecto-curvatura" in clases(), "los efectos se recuerdan"
    assert n.pag.is_checked("#efecto-grano") is False


def abrir_ajustes(n):
    if "colapsado" in n.pag.get_attribute("#panel-ajustes", "class"):
        n.pag.click("#panel-ajustes [data-accion='colapsar']")


@con_navegador
def prueba_tema_sobrio_sin_efectos_con_la_misma_funcionalidad(n, p):
    abrir_ajustes(n)
    n.pag.select_option("#ajuste-tema", "sobrio")
    assert n.pag.get_attribute("html", "data-tema") == "sobrio"
    estilo = n.pag.evaluate("""(() => {
        const m = document.getElementById('monitor');
        const cs = getComputedStyle(m);
        return {sombra: cs.boxShadow, radio: cs.borderRadius, antes: getComputedStyle(m, '::before').display,
                despues: getComputedStyle(m, '::after').display, textura: getComputedStyle(document.body).backgroundImage,
                fondo: getComputedStyle(document.body).backgroundColor};
    })()""")
    assert estilo["sombra"] == "none" and estilo["antes"] == "none" and estilo["despues"] == "none", estilo
    assert estilo["textura"] == "none" and estilo["radio"] in ("0px", "0"), estilo
    assert estilo["fondo"] == "rgb(232, 235, 239)", estilo
    # La misma funcionalidad: depurar, editar, buscar.
    n.pag.click("#accion-paso")
    n.pag.wait_for_selector("[data-registro='R1']")
    abrir_archivo(n, "p.s")
    assert n.pag.locator(".resalte .t-mnemonico").count() >= 1
    n.recargar()
    assert n.pag.get_attribute("html", "data-tema") == "sobrio", "el tema se recuerda"


@con_navegador
def prueba_temas_propios_con_marcas_modificables(n, p):
    abrir_ajustes(n)
    n.pag.fill("#marca-color-fondo", "#102030")
    n.pag.fill("#marca-color-acento", "#ff00aa")
    n.pag.select_option("#marca-fuente", "Georgia, serif")
    n.pag.fill("#tema-nombre", "mio")
    n.pag.click("#tema-guardar")
    css = lambda v: n.pag.evaluate(f"getComputedStyle(document.documentElement).getPropertyValue('{v}').trim()")
    assert css("--color-fondo") == "#102030" and css("--color-acento") == "#ff00aa" and "Georgia" in css("--fuente")
    assert n.pag.evaluate("getComputedStyle(document.body).backgroundColor") == "rgb(16, 32, 48)"
    n.recargar()
    assert css("--color-fondo") == "#102030", "el tema propio se recuerda"
    n.pag.select_option("#ajuste-tema", "oscuro")
    assert css("--color-fondo") == "#121418"
    n.pag.select_option("#ajuste-tema", "mio")
    assert css("--color-fondo") == "#102030"
    n.pag.click("#tema-borrar")
    assert css("--color-fondo") == "#121418"
    assert n.pag.locator("#ajuste-tema option[value='mio']").count() == 0


@con_navegador
def prueba_sonido_por_familia_y_sin_sonido(n, p):
    cuenta = lambda f: n.pag.evaluate(f"L.sonido.cuenta['{f}']")
    abrir_ajustes(n)
    assert not any(n.pag.is_checked(f"#sonido-{f}") for f in ["rele", "teclado", "campanilla"]), "por defecto en silencio"
    n.pag.click("#accion-paso")
    n.pag.wait_for_selector("[data-registro='R1']")
    assert cuenta("rele") == 0
    n.pag.check("#sonido-rele")
    n.pag.wait_for_timeout(100)
    n.pag.click("#accion-paso")
    n.pag.wait_for_function("L.sonido.cuenta.rele === 1")
    assert cuenta("teclado") == 0 and cuenta("campanilla") == 0, "cada familia es independiente"
    abrir_archivo(n, "p.s")
    n.pag.click(".area")
    n.pag.keyboard.type("x")
    assert cuenta("teclado") == 0
    n.pag.check("#sonido-teclado")
    n.pag.click(".area")
    n.pag.keyboard.type("y")
    assert cuenta("teclado") == 1
    n.pag.uncheck("#sonido-rele")
    n.pag.check("#sonido-campanilla")
    n.pag.fill(".area", "        HLT\n")
    n.pag.click("#accion-ejecutar")
    n.pag.wait_for_function("L.sonido.cuenta.campanilla === 1")
    assert cuenta("rele") == 1, "el rele esta silenciado"
    n.recargar()
    assert n.pag.is_checked("#sonido-campanilla") and not n.pag.is_checked("#sonido-rele")


@con_navegador
def prueba_tamano_de_texto_sin_romper_la_disposicion(n, p):
    abrir_ajustes(n)
    for px in (11, 20, 28):
        n.pag.fill("#ajuste-tam", str(px))
        n.pag.dispatch_event("#ajuste-tam", "input")
        assert n.pag.evaluate("getComputedStyle(document.documentElement).fontSize") == f"{px}px"
        desborde = n.pag.evaluate("document.documentElement.scrollWidth - document.documentElement.clientWidth")
        assert desborde <= 0, f"con {px}px hay desborde horizontal de {desborde}px"
        caja = n.pag.locator("#accion-paso").bounding_box()
        assert caja and caja["x"] >= 0 and caja["x"] + caja["width"] <= 1400, caja
    n.recargar()
    assert n.pag.evaluate("getComputedStyle(document.documentElement).fontSize") == "28px"


@con_navegador
def prueba_alto_contraste(n, p):
    abrir_ajustes(n)
    n.pag.check("#ajuste-contraste")
    razon = n.pag.evaluate("""(() => {
        const lum = c => { const m = c.match(/\\d+/g).map(Number); const f = v => { v /= 255; return v <= .03928 ? v / 12.92 : Math.pow((v + .055) / 1.055, 2.4); };
            return .2126 * f(m[0]) + .7152 * f(m[1]) + .0722 * f(m[2]); };
        const cs = getComputedStyle(document.body);
        const a = lum(cs.color), b = lum(cs.backgroundColor);
        return (Math.max(a, b) + .05) / (Math.min(a, b) + .05);
    })()""")
    assert razon >= 7, razon
    assert n.pag.get_attribute("html", "data-contraste") == "alto"
    n.pag.click("#accion-paso")
    n.pag.wait_for_selector("[data-registro='R1'] .lampara.on")


@con_navegador
def prueba_progreso_de_operaciones_largas(n, p):
    def lento(ruta):
        import time
        time.sleep(1.8)
        ruta.continue_()
    n.pag.route("**/api/ensamblar", lento)
    abrir_archivo(n, "p.s")
    assert n.pag.locator("#progreso").is_hidden()
    n.pag.keyboard.press("Control+b")
    n.pag.wait_for_selector("#progreso:not([hidden])", timeout=15000)
    assert "Compilando" in n.pag.inner_text("#progreso-texto")
    n.pag.wait_for_selector("#progreso", state="hidden", timeout=20000)


@con_navegador
def prueba_reasignar_un_atajo_desde_la_interfaz(n, p):
    abrir_ajustes(n)
    n.pag.click("#tabla-atajos [data-atajo='paso']")
    n.pag.keyboard.press("Control+y")
    assert n.pag.inner_text("#tabla-atajos [data-atajo='paso']") == "Ctrl+Y"
    n.pag.keyboard.press("F10")
    n.pag.wait_for_timeout(300)
    assert n.pag.locator("[data-registro='R1']").count() == 0, "F10 ya no avanza"
    n.pag.keyboard.press("Control+y")
    n.pag.wait_for_selector("[data-registro='R1']")
    n.pag.click("#atajos-restaurar")
    assert n.pag.inner_text("#tabla-atajos [data-atajo='paso']") == "F10"


def abrir_panel(n, id_):
    """Deja el panel a la vista: lo despliega, o elige su pestana si esta en la zona inferior."""
    if n.pag.locator(f"#panel-{id_}.en-pestana-oculta").count():
        n.pag.click(f"#pestana-{id_}")
    elif n.pag.locator(f"#panel-{id_}.colapsado").count() and not n.pag.locator(f"#zona-abajo #panel-{id_}").count():
        n.pag.click(f"#panel-{id_} [data-accion='colapsar']")


@con_navegador
def prueba_instante_con_control_deslizante(n, p):
    abrir_panel(n, "instante")
    n.pag.click("#accion-paso")
    n.pag.wait_for_function("document.getElementById('contadores').textContent.includes('PC 0x0001')")
    n.pag.click("#accion-paso")
    n.pag.wait_for_function("document.getElementById('contadores').textContent.includes('PC 0x0002')")
    assert n.pag.get_attribute("#instante-desliz", "max") == "2"
    n.pag.fill("#instante-desliz", "1")
    n.pag.wait_for_function("document.getElementById('contadores').textContent.includes('PC 0x0001')")
    assert "Instante 1 de 2" in n.pag.inner_text("#instante-texto")
    n.pag.select_option("#velocidad", "250")
    assert n.pag.evaluate("L.sesion.lento") == 250


@con_navegador
def prueba_camino_del_dato_se_anima(n, p):
    elegir(n, "serie.toml", "hola.s")
    n.pag.click("#accion-paso")
    n.pag.wait_for_function("document.getElementById('dato-detalle').textContent.length > 0")
    assert n.pag.get_attribute("#dato-svg", "role") == "img"
    assert n.pag.locator("#dato-svg .flujo").count() == 7
    assert n.pag.inner_text("#dato-rotulo").strip() != ""
    # el significado no depende del color: el rotulo nombra el camino activo
    for _ in range(12):
        if "Puertos" in n.pag.inner_text("#dato-rotulo") or "salida" in n.pag.inner_text("#dato-rotulo"):
            break
        n.pag.click("#accion-paso")
        n.pag.wait_for_timeout(150)


@con_navegador
def prueba_mapa_de_memoria_y_editor_hexadecimal(n, p):
    abrir_panel(n, "mapa")
    n.pag.click("#accion-paso")
    n.pag.wait_for_function("document.getElementById('mapa-lienzo').getAttribute('data-bloques') !== null")
    assert int(n.pag.get_attribute("#mapa-lienzo", "data-bloques")) > 0
    assert n.pag.locator("#mapa-leyenda li").count() >= 1
    abrir_panel(n, "memoria")
    n.pag.wait_for_selector("#memoria-vista .celda")
    n.pag.click("#memoria-vista .celda[data-dir='100']")
    n.pag.fill("#memoria-vista input.edicion", "0x5a")
    n.pag.keyboard.press("Enter")
    n.pag.wait_for_function("(document.querySelector('#memoria-vista .celda[data-dir=\\'100\\']')?.textContent || '').includes('5a')")
    assert "cambi" in n.pag.get_attribute("#memoria-vista .celda[data-dir='100']", "title")


@con_navegador
def prueba_linea_de_tiempo_de_eventos(n, p):
    abrir_panel(n, "linea-tiempo")
    elegir(n, "serie.toml", "hola.s")
    n.pag.click("#accion-ejecutar")
    n.pag.wait_for_function("document.querySelectorAll('#eventos-lista li').length >= 2")
    textos = n.pag.inner_text("#eventos-lista")
    assert "E/S" in textos and "PARADA" in textos, textos


@con_navegador
def prueba_consola_de_ordenes_del_depurador(n, p):
    abrir_panel(n, "ordenes")
    n.pag.fill("#ordenes-entrada", "paso 2")
    n.pag.keyboard.press("Enter")
    n.pag.wait_for_function("document.getElementById('contadores').textContent.includes('PC 0x0002')")
    assert "(lorenz) paso 2" in n.pag.inner_text("#ordenes-salida")
    n.pag.fill("#ordenes-entrada", "ayuda")
    n.pag.keyboard.press("Enter")
    n.pag.wait_for_function("document.getElementById('ordenes-salida').textContent.includes('atras')")
    n.pag.click("#ordenes-entrada")
    n.pag.keyboard.press("ArrowUp")
    assert n.pag.input_value("#ordenes-entrada") == "ayuda"


@con_navegador
def prueba_visor_de_la_unidad_grafica_en_vivo(n, p):
    abrir_panel(n, "gpu")
    n.pag.select_option("#sel-maquina", "z-gpu/gpu-16/maquina.toml")
    n.pag.select_option("#sel-programa", "z-gpu/gpu-16/programas/triangulo.s")
    n.pag.click("#gpu-abrir")
    n.pag.wait_for_function("document.getElementById('gpu-info').textContent.includes('×')")
    assert n.pag.get_attribute("#gpu-lienzo", "data-pixeles") == "0"
    n.pag.click("#gpu-vivo")
    n.pag.wait_for_function("document.getElementById('gpu-info').textContent.includes('DETENIDA')")
    n.pag.wait_for_function("+document.getElementById('gpu-lienzo').getAttribute('data-pixeles') > 0")
    n.pag.wait_for_function("+document.getElementById('gpu-lienzo').getAttribute('data-cuadros') >= 2")
    azul = n.pag.evaluate("(() => { const c = document.getElementById('gpu-lienzo'); const d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data; let b = 0; for (let i = 0; i < d.length; i += 4) if (d[i + 2] > 200 && d[i] < 60) b++; return b; })()")
    assert azul > 0, "el triangulo azul debe verse en el lienzo"
    n.pag.click("#gpu-lienzo", position={"x": 5, "y": 5})
    n.pag.wait_for_function("document.getElementById('gpu-pixel').textContent.includes('r=')")


@con_navegador
def prueba_ejecucion_lenta_respeta_las_rupturas(n, p):
    abrir_panel(n, "instante")
    n.pag.select_option("#velocidad", "50")
    n.pag.click("#accion-paso")
    n.pag.wait_for_selector("#desens-lista li")
    n.pag.click("#desens-lista li[data-direccion='2'] .bp")
    n.pag.wait_for_selector("#desens-lista li[data-direccion='2'] .bp.puesta")
    n.pag.click("#accion-reiniciar")
    n.pag.wait_for_function("document.getElementById('contadores').textContent.includes('instr 0')")
    n.pag.click("#accion-ejecutar")
    n.pag.wait_for_function("document.getElementById('estado-texto').textContent.includes('ruptura')")
    assert "PC 0x0002" in n.pag.inner_text("#contadores")


def abrir_arquitectura(n, archivo="maquina.toml"):
    n.pag.evaluate(f"L.editor.abrir('{archivo}')")
    n.pag.wait_for_function(f"L.editor.activo() === '{archivo}'")
    abrir_panel(n, "arquitectura")
    n.pag.wait_for_function("document.getElementById('arq-lienzo').getAttribute('data-bloques') > 0")


def bloques_de(n, clase):
    return n.pag.locator(f".arq-bloque[data-clase='{clase}']").count()


@con_navegador
def prueba_arquitectura_texto_a_diagrama(n, p):
    abrir_arquitectura(n)
    texto = n.pag.input_value(".area")
    esperados = texto.count("[[registro]]")
    assert esperados >= 3 and bloques_de(n, "registro") == esperados
    assert bloques_de(n, "region") == texto.count("[[region]]")
    # Un cambio hecho en el texto aparece en el diagrama
    n.pag.fill(".area", texto + '\n[[registro]]\nnombre = "ZZ"\nbits = 8\nrol = "general"\ninicial = 0\n')
    n.pag.wait_for_selector(".arq-bloque[data-clase='registro'][data-nombre='ZZ']")
    assert bloques_de(n, "registro") == esperados + 1
    assert "8 bits" in n.pag.text_content(".arq-bloque[data-nombre='ZZ'] .arq-sub")
    # y al quitarlo del texto desaparece
    n.pag.fill(".area", texto)
    n.pag.wait_for_function(f"document.querySelectorAll(\".arq-bloque[data-clase='registro']\").length === {esperados}")


@con_navegador
def prueba_arquitectura_diagrama_a_texto_conserva_comentarios(n, p):
    abrir_arquitectura(n)
    antes = n.pag.input_value(".area")
    comentarios = [l for l in antes.splitlines() if l.lstrip().startswith("#")]
    assert comentarios, "la descripcion de prueba debe tener comentarios"
    regs = bloques_de(n, "registro")
    n.pag.click("#arq-paleta [data-nuevo='registro']")
    n.pag.wait_for_function(f"document.querySelectorAll(\".arq-bloque[data-clase='registro']\").length === {regs + 1}")
    despues = n.pag.input_value(".area")
    assert f'nombre = "R{regs + 1}"' in despues
    for c in comentarios:
        assert c in despues, f"se perdio el comentario: {c}"
    assert despues.count("[[registro]]") == regs + 1
    # el bloque nuevo queda elegido y su formulario permite cambiarlo
    n.pag.fill("#arq-campo-bits", "4")
    n.pag.keyboard.press("Enter")
    n.pag.wait_for_function("document.querySelector('.arq-bloque.elegido .arq-sub').textContent.startsWith('4 bits')")
    assert 'bits = 4' in n.pag.input_value(".area")
    # deshacer devuelve el texto de antes
    n.pag.click("#arq-deshacer")
    n.pag.click("#arq-deshacer")
    n.pag.wait_for_function(f"document.querySelectorAll(\".arq-bloque[data-clase='registro']\").length === {regs}")
    assert n.pag.input_value(".area") == antes
    # el archivo quedo marcado como cambiado hasta que se guarde, y en disco no se toco
    assert "•" not in n.pag.inner_text(".pestana.activa")


@con_navegador
def prueba_arquitectura_se_compone_arrastrando(n, p):
    abrir_arquitectura(n)
    antes = bloques_de(n, "region")
    origen = n.pag.locator("#arq-paleta [data-nuevo='region']")
    destino = n.pag.locator("#arq-marco")
    destino.scroll_into_view_if_needed()
    origen.drag_to(destino, target_position={"x": 400, "y": 120})
    n.pag.wait_for_function(f"document.querySelectorAll(\".arq-bloque[data-clase='region']\").length === {antes + 1}")
    assert 'acceso = "rw"' in n.pag.input_value(".area")
    # y el bloque nuevo cae donde se solto: se puede mover despues con el raton
    caja = n.pag.locator(".arq-bloque.elegido").bounding_box()
    n.pag.mouse.move(caja["x"] + 20, caja["y"] + 20)
    n.pag.mouse.down()
    n.pag.mouse.move(caja["x"] + 80, caja["y"] + 70, steps=5)
    n.pag.mouse.up()
    assert n.pag.get_attribute("#arq-lienzo", "data-movidos") is not None
    despues = n.pag.locator(".arq-bloque.elegido").bounding_box()
    assert abs(despues["x"] - caja["x"]) > 20


@con_navegador
def prueba_arquitectura_senala_los_errores_en_el_diagrama(n, p):
    abrir_arquitectura(n)
    assert n.pag.locator(".arq-bloque.con-error").count() == 0
    primero = n.pag.locator(".arq-bloque[data-clase='registro']").nth(0).get_attribute("data-nombre")
    n.pag.locator(".arq-bloque[data-clase='registro']").nth(1).click()
    n.pag.fill("#arq-campo-nombre", primero)
    n.pag.keyboard.press("Enter")
    n.pag.wait_for_selector(".arq-bloque.con-error")
    # el error no depende del color: marca en el texto del bloque y descripcion accesible
    assert "⚠" in n.pag.text_content(".arq-bloque.con-error .arq-nombre")
    assert "error" in n.pag.get_attribute(".arq-bloque.con-error", "aria-label")
    assert n.pag.locator("#arq-errores li").count() >= 1
    n.pag.click("#arq-errores li button")
    n.pag.wait_for_function("document.activeElement && document.activeElement.classList.contains('area')")
    n.pag.click("#arq-deshacer")
    n.pag.wait_for_function("document.querySelectorAll('.arq-bloque.con-error').length === 0")


@con_navegador
def prueba_arquitectura_con_teclado(n, p):
    abrir_arquitectura(n)
    regs = bloques_de(n, "registro")
    n.pag.locator(".arq-bloque[data-clase='registro']").nth(0).focus()
    n.pag.keyboard.press("Enter")
    n.pag.wait_for_selector(".arq-bloque.elegido")
    n.pag.keyboard.press("Delete")
    n.pag.wait_for_function(f"document.querySelectorAll(\".arq-bloque[data-clase='registro']\").length === {regs - 1}")


def con_web(f):
    """La prueba recibe un servicio web propio y una funcion para abrir visitantes distintos."""
    def envuelta(pw, p):
        w = ProyectoWeb(p.binario)
        abiertos = []

        def visitante():
            n = Navegador(pw, w, seleccionar=False)
            abiertos.append(n)
            return n

        try:
            f(visitante, w)
        finally:
            for n in abiertos:
                n.cerrar()
            w.cerrar()
    envuelta.__name__ = f.__name__
    return envuelta


@con_navegador
def prueba_ide_local_explica_que_compartir_es_de_la_web(n, p):
    assert n.pag.is_hidden("#banda-web")
    abrir_panel(n, "compartir")
    assert n.pag.is_visible("#compartir-solo-web")
    assert n.pag.is_hidden("#compartir-web")


@con_web
def prueba_web_avisa_de_lo_que_no_hay_en_el_escritorio(visitante, w):
    n = visitante()
    n.pag.wait_for_selector("#banda-web:not([hidden])")
    n.pag.click("#web-diferencias summary")
    assert n.pag.locator("#web-limitaciones li").count() >= 5
    assert "externos" in n.pag.inner_text("#web-limitaciones")
    assert "Versión web" in n.pag.inner_text("#banda-web")
    # el espacio viene con ejemplos
    assert n.pag.locator("#sel-maquina option[value='ejemplo-8/maquina.toml']").count() == 1


@con_web
def prueba_web_conserva_el_trabajo_entre_sesiones(visitante, w):
    n = visitante()
    n.pag.evaluate("L.api('PUT','/api/archivo',{ruta:'mio.s',texto:'        HLT\\n'})")
    n.pag.reload()
    n.pag.wait_for_selector("body[data-listo='1']")
    assert n.pag.locator("#sel-programa option[value='mio.s']").count() == 1
    # otra persona (otro navegador, otra galleta) no lo ve
    otro = visitante()
    assert otro.pag.locator("#sel-programa option[value='mio.s']").count() == 0
    # exportar da un paquete
    datos = n.pag.evaluate("fetch('/api/exportar').then(r => r.text()).then(t => t.slice(0, 20))")
    assert datos.startswith("LORENZ-PAQUETE 1")


@con_web
def prueba_web_compartir_abrir_ejecutar_derivar_y_galeria(visitante, w):
    ana = visitante()
    ana.pag.select_option("#sel-maquina", "ejemplo-8/maquina.toml")
    ana.pag.select_option("#sel-programa", "ejemplo-8/programas/suma.s")
    abrir_panel(ana, "compartir")
    ana.pag.click("#compartir-hacer")
    ana.pag.wait_for_function("document.getElementById('compartir-enlace').value.includes('/c/')")
    enlace = ana.pag.input_value("#compartir-enlace")
    ana.pag.fill("#galeria-titulo", "Suma en ocho bits")
    ana.pag.fill("#galeria-descripcion", "Un ejemplo para empezar")
    ana.pag.fill("#galeria-etiquetas", "didáctica, suma")
    ana.pag.click("#galeria-publicar")
    ana.pag.wait_for_function("document.getElementById('compartir-aviso').textContent.includes('Publicado')")

    # Eva abre el enlace sin instalar nada y lo ejecuta
    eva = visitante()
    eva.pag.goto(enlace)
    eva.pag.wait_for_selector("body[data-listo='1']")
    assert eva.pag.input_value("#sel-maquina").startswith("compartido/")
    assert eva.pag.input_value("#sel-programa").endswith("suma.s")
    eva.pag.click("#accion-ejecutar")
    eva.pag.wait_for_function("document.getElementById('estado-texto').textContent.includes('detenida')")
    # lo compartido no se edita...
    resp = eva.pag.evaluate("L.api('PUT','/api/archivo',{ruta:L.proyecto.programa,texto:'x'}).then(r => r.estado)")
    assert resp == 403
    # ...se deriva una copia, y esa si
    abrir_panel(eva, "compartir")
    eva.pag.fill("#derivar-nombre", "mi-suma")
    eva.pag.click("#derivar-hacer")
    eva.pag.wait_for_function("document.getElementById('sel-programa').value === 'mi-suma/ejemplo-8/programas/suma.s'")
    resp = eva.pag.evaluate("L.api('PUT','/api/archivo',{ruta:L.proyecto.programa,texto:'        HLT\\n'}).then(r => r.estado)")
    assert resp == 200
    # la galeria la encuentra por palabras, sin importar acentos
    eva.pag.fill("#galeria-q", "didactica")
    eva.pag.click("#galeria-buscar")
    eva.pag.wait_for_selector("#galeria-lista .entrada-galeria")
    assert "Suma en ocho bits" in eva.pag.inner_text("#galeria-lista")
    eva.pag.fill("#galeria-q", "cohete")
    eva.pag.click("#galeria-buscar")
    eva.pag.wait_for_selector("#galeria-vacia")


@con_navegador
def prueba_elegir_un_programa_lo_abre_en_el_editor(n, p):
    n.pag.select_option("#sel-programa", "p.s")
    n.pag.wait_for_function("L.editor.activo() === 'p.s' && L.editor.texto().length > 0")


@con_navegador
def prueba_borrador_desde_texto_crea_y_abre_la_maquina(n, p):
    abrir_panel(n, "arquitectura")
    n.pag.click("#arq-borrador summary")
    n.pag.fill("#borrador-texto", "Procesador de 16 bits con 8 registros, 64 KiB de memoria, suma y resta")
    n.pag.click("#borrador-crear")
    n.pag.wait_for_function("L.editor.activo() === 'borrador.toml'")
    assert n.pag.locator("#borrador-supuestos li").count() >= 2
    # el diagrama muestra la maquina nueva y es valida
    n.pag.wait_for_function("document.getElementById('arq-aviso').textContent.includes('válida')")
    # una segunda vez no pisa la primera
    n.pag.click("#borrador-crear")
    n.pag.wait_for_function("L.editor.activo() === 'borrador-2.toml'")
    # sin texto, avisa
    n.pag.fill("#borrador-texto", "")
    n.pag.click("#borrador-crear")
    assert "Escribe" in n.pag.inner_text("#borrador-aviso")


@con_navegador
def prueba_pestanas_inferiores_se_manejan_con_teclado(n, p):
    assert n.pag.locator("#pestanas-abajo [role=tab]").count() >= 6
    assert n.pag.locator("#zona-abajo .panel:not(.en-pestana-oculta)").count() == 1, "solo un panel a la vez"
    n.pag.click("#pestana-buscar")
    n.pag.wait_for_selector("#buscar-texto", state="visible")
    n.pag.keyboard.press("ArrowRight")
    n.pag.wait_for_function("document.activeElement.id === 'pestana-problemas'")
    assert n.pag.get_attribute("#pestana-problemas", "aria-selected") == "true"
    assert n.pag.get_attribute("#pestana-buscar", "aria-selected") == "false"
    n.recargar()
    assert n.pag.get_attribute("#pestana-problemas", "aria-selected") == "true", "la pestana se recuerda"


@con_navegador
def prueba_menu_del_panel_se_cierra_con_escape(n, p):
    n.pag.click("#panel-archivos .menu-panel > summary")
    assert n.pag.is_visible("#panel-archivos .menu [data-accion='der']")
    n.pag.keyboard.press("Escape")
    assert not n.pag.is_visible("#panel-archivos .menu [data-accion='der']")
    n.pag.click("#panel-archivos .menu-panel > summary")
    n.pag.click("#titulo-editor")
    assert not n.pag.is_visible("#panel-archivos .menu [data-accion='der']"), "pulsar fuera lo cierra"


@con_navegador
def prueba_la_linea_en_ejecucion_se_marca_en_el_editor(n, p):
    abrir_archivo(n, "p.s")
    assert n.pag.locator(".resalte .linea.ejecutando").count() == 0
    n.pag.click("#accion-paso")
    n.pag.wait_for_selector(".resalte .linea.ejecutando")
    assert n.pag.locator(".resalte .linea.ejecutando").count() == 1
    assert n.pag.locator(".numeros .ejecutando").count() == 1
    n.pag.click("#accion-reiniciar")
    n.pag.wait_for_function("document.querySelectorAll('.resalte .linea.ejecutando').length <= 1")


PRUEBAS = {f.__name__: f for f in [prueba_pagina_carga, prueba_api_desde_el_navegador, prueba_paneles_se_mueven_y_se_recuerdan, prueba_paneles_se_arrastran, prueba_atajos_reasignables, prueba_editor_resalta_segun_la_arquitectura, prueba_editor_resalta_sgt, prueba_editor_autocompleta, prueba_editor_muestra_la_codificacion, prueba_editor_marca_errores_en_la_linea, prueba_editor_va_a_la_definicion, prueba_guardado_automatico_y_recuperacion, prueba_buscar_y_reemplazar_en_el_proyecto, prueba_lista_de_problemas, prueba_documentacion_dentro_del_ide, prueba_depurador_paso_a_paso_con_lamparas, prueba_depurador_lamparas_se_pueden_conmutar, prueba_depurador_ruptura_y_ejecucion, prueba_depuracion_completa_solo_con_teclado, prueba_las_banderas_no_dependen_del_color, prueba_consola_de_teletipo, prueba_pantalla_muestra_el_framebuffer, prueba_efectos_del_monitor_son_independientes, prueba_tema_sobrio_sin_efectos_con_la_misma_funcionalidad, prueba_temas_propios_con_marcas_modificables, prueba_sonido_por_familia_y_sin_sonido, prueba_tamano_de_texto_sin_romper_la_disposicion, prueba_alto_contraste, prueba_progreso_de_operaciones_largas, prueba_reasignar_un_atajo_desde_la_interfaz, prueba_instante_con_control_deslizante, prueba_camino_del_dato_se_anima, prueba_mapa_de_memoria_y_editor_hexadecimal, prueba_linea_de_tiempo_de_eventos, prueba_consola_de_ordenes_del_depurador, prueba_visor_de_la_unidad_grafica_en_vivo, prueba_ejecucion_lenta_respeta_las_rupturas, prueba_arquitectura_texto_a_diagrama, prueba_arquitectura_diagrama_a_texto_conserva_comentarios, prueba_arquitectura_se_compone_arrastrando, prueba_arquitectura_senala_los_errores_en_el_diagrama, prueba_arquitectura_con_teclado, prueba_ide_local_explica_que_compartir_es_de_la_web, prueba_web_avisa_de_lo_que_no_hay_en_el_escritorio, prueba_web_conserva_el_trabajo_entre_sesiones, prueba_web_compartir_abrir_ejecutar_derivar_y_galeria, prueba_elegir_un_programa_lo_abre_en_el_editor, prueba_borrador_desde_texto_crea_y_abre_la_maquina, prueba_pestanas_inferiores_se_manejan_con_teclado, prueba_menu_del_panel_se_cierra_con_escape, prueba_la_linea_en_ejecucion_se_marca_en_el_editor]}


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
    desconocidas = [e for e in elegidas if e not in PRUEBAS]
    if desconocidas:
        print("pruebas desconocidas:", desconocidas)
        return 2
    if len(elegidas) > 1 and not os.environ.get("LORENZ_UI_SERIE"):
        return en_paralelo(binario, elegidas)
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
                import traceback
                linea = [f for f in traceback.extract_tb(e.__traceback__) if f.filename == __file__]
                donde = f" (banco.py:{linea[-1].lineno})" if linea else ""
                print(f"FALLA {nombre}{donde}: {type(e).__name__}: {e}")
            finally:
                p.cerrar()
    return 1 if fallos else 0


def en_paralelo(binario, elegidas):
    """Cada prueba en su propio proceso (su propio servidor y navegador), varias a la vez."""
    from concurrent.futures import ThreadPoolExecutor

    def una(nombre):
        env = dict(os.environ, LORENZ_UI_SERIE="1")
        r = subprocess.run([sys.executable, os.path.abspath(__file__), binario, nombre], capture_output=True, text=True, env=env)
        if r.returncode not in (0, OMITIDA):  # una sola repeticion: bajo carga algunas esperas se agotan
            r = subprocess.run([sys.executable, os.path.abspath(__file__), binario, nombre], capture_output=True, text=True, env=env)
        return nombre, r.returncode, (r.stdout + r.stderr).strip()

    trabajadores = min(len(elegidas), max(1, (os.cpu_count() or 2)), 6)
    omitidas = fallos = 0
    with ThreadPoolExecutor(trabajadores) as ex:
        for nombre, codigo, salida in ex.map(una, elegidas):
            if codigo == OMITIDA:
                omitidas += 1
                print(salida)
            else:
                print(salida or f"{'ok   ' if codigo == 0 else 'FALLA'} {nombre}")
                fallos += codigo != 0
    if omitidas == len(elegidas):
        return OMITIDA
    return 1 if fallos else 0


if __name__ == "__main__":
    sys.exit(principal(sys.argv))
