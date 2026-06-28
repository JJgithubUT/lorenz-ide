// Instrumentos: consola de teletipo y pantalla con efecto de monitor (RF-IDE-022 a RF-IDE-024, RF-GPU-003).
'use strict';
(function () {
  var L = window.L;
  var EFECTOS = ['curvatura', 'grano', 'parpadeo', 'texturas'];
  var ETIQUETAS = { curvatura: 'Curvatura', grano: 'Grano', parpadeo: 'Parpadeo', texturas: 'Texturas (líneas de barrido y brillo)' };

  function dispositivo(tipo) {
    var e = L.sesion.estado;
    return e && (e.dispositivos || []).filter(function (d) { return d.tipo === tipo; })[0];
  }

  // --- consola: salida continua estilo teletipo ---------------------------------------------
  L.paneles.registrar({
    id: 'consola', titulo: 'Consola (teletipo)', zona: 'abajo',
    construir: function (cuerpo) {
      var salida = L.el('pre', { id: 'consola-salida', class: 'teletipo', role: 'log', 'aria-live': 'off', 'aria-label': 'Salida de la UART', tabindex: '0' });
      var entrada = L.el('input', { type: 'text', id: 'consola-entrada', 'aria-label': 'Enviar texto a la UART', placeholder: 'Escribe y pulsa Intro para enviar…' });
      var visto = 0, nombre = null, pidiendo = false;
      function tomar() {
        var d = dispositivo('uart');
        if (!d) { salida.textContent = 'Esta máquina no tiene UART.'; visto = 0; nombre = null; entrada.disabled = true; return; }
        entrada.disabled = false;
        if (nombre !== d.nombre) { nombre = d.nombre; visto = 0; salida.textContent = ''; }
        if (pidiendo) return;
        pidiendo = true;
        L.depurador.pedir('serie/' + encodeURIComponent(d.nombre)).then(function (r) {
          pidiendo = false;
          if (!r.ok) return;
          var t = r.datos.texto;
          if (t.length < visto) { visto = 0; salida.textContent = ''; }
          if (t.length > visto) {
            salida.append(document.createTextNode(t.slice(visto)));
            visto = t.length;
            salida.scrollTop = salida.scrollHeight;
          }
        });
      }
      entrada.addEventListener('keydown', function (ev) {
        if (ev.key !== 'Enter' || !nombre) return;
        L.depurador.editar('serie/' + encodeURIComponent(nombre), { texto: entrada.value + '\n' });
        entrada.value = '';
      });
      cuerpo.append(salida, entrada);
      L.on('estado', function (e) { if (!e) { salida.textContent = ''; visto = 0; return; } tomar(); });
    }
  });

  // --- pantalla con efecto de monitor de tubo -------------------------------------------------
  L.paneles.registrar({
    id: 'pantalla', titulo: 'Pantalla', zona: 'abajo',
    construir: function (cuerpo) {
      var lienzo = L.el('canvas', { id: 'pantalla-lienzo', width: '64', height: '32', role: 'img', 'aria-label': 'Pantalla de la máquina emulada' });
      var monitor = L.el('div', { class: 'monitor', id: 'monitor' }, [lienzo]);
      var guardados = L.guardado.leer('efectos', { curvatura: true, grano: true, parpadeo: true, texturas: true });
      var controles = L.el('fieldset', { class: 'efectos' }, [L.el('legend', { texto: 'Efectos del monitor' })]);
      function aplicar() {
        EFECTOS.forEach(function (f) { monitor.classList.toggle('efecto-' + f, !!guardados[f]); });
        L.guardado.escribir('efectos', guardados);
      }
      EFECTOS.forEach(function (f) {
        var c = L.el('input', { type: 'checkbox', id: 'efecto-' + f });
        c.checked = !!guardados[f];
        c.addEventListener('change', function () { guardados[f] = c.checked; aplicar(); });
        controles.append(L.el('label', {}, [c, ' ' + ETIQUETAS[f]]));
      });
      var info = L.el('p', { class: 'suave', id: 'pantalla-info' });
      var ctx = lienzo.getContext('2d');
      var pidiendo = false;
      function dibujar(img) {
        if (lienzo.width !== img.ancho || lienzo.height !== img.alto) { lienzo.width = img.ancho; lienzo.height = img.alto; }
        var datos = ctx.createImageData(img.ancho, img.alto), maximo = 1;
        img.pixeles.forEach(function (v) { if (v > maximo) maximo = v; });
        for (var i = 0; i < img.pixeles.length; i++) {
          var v = maximo > 1 ? img.pixeles[i] : img.pixeles[i] * 255;
          datos.data[i * 4] = Math.round(v * 0.25); datos.data[i * 4 + 1] = v; datos.data[i * 4 + 2] = Math.round(v * 0.4); datos.data[i * 4 + 3] = 255;
        }
        ctx.putImageData(datos, 0, 0);
        lienzo.setAttribute('data-encendidos', String(img.pixeles.filter(Boolean).length));
      }
      function tomar() {
        var d = dispositivo('pantalla');
        if (!d) { info.textContent = 'Esta máquina no tiene pantalla.'; lienzo.hidden = true; return; }
        lienzo.hidden = false; info.textContent = d.nombre + ' (' + d.ancho + '×' + d.alto + ')';
        if (pidiendo) return;
        pidiendo = true;
        L.depurador.pedir('pantalla/' + encodeURIComponent(d.nombre)).then(function (r) {
          pidiendo = false;
          if (r.ok) dibujar(r.datos);
        });
      }
      cuerpo.append(monitor, info, controles);
      aplicar();
      L.on('estado', function (e) { if (e) tomar(); else { ctx.clearRect(0, 0, lienzo.width, lienzo.height); lienzo.setAttribute('data-encendidos', '0'); } });
    }
  });
})();
