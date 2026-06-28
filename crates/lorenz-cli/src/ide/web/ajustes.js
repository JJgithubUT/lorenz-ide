// Apariencia, sonido y accesibilidad (RF-IDE-025 a RF-IDE-032, RF-IDE-034).
'use strict';
(function () {
  var L = window.L;
  var MARCAS = [
    ['color-fondo', 'Fondo'], ['color-superficie', 'Superficie'], ['color-borde', 'Borde'], ['color-texto', 'Texto'], ['color-suave', 'Texto suave'],
    ['color-acento', 'Acento'], ['color-ok', 'Correcto'], ['color-error', 'Error'], ['color-aviso', 'Aviso'], ['color-lampara-on', 'Lámpara encendida'], ['color-lampara-off', 'Lámpara apagada']
  ];
  var FUENTES = {
    'DejaVu Sans Mono, Menlo, Consolas, monospace': 'Monoespaciada', 'Georgia, serif': 'Serif', 'system-ui, sans-serif': 'Sistema', '"Courier New", Courier, monospace': 'Courier'
  };
  var TEXTURAS = {
    'none': 'Ninguna',
    'repeating-linear-gradient(0deg, rgba(0,0,0,.18) 0 1px, transparent 1px 3px)': 'Líneas',
    'radial-gradient(rgba(255,255,255,.05) 1px, transparent 1px) 0 0 / 6px 6px': 'Puntos'
  };
  var TEMAS_BASE = { oscuro: 'Oscuro (predeterminado)', sobrio: 'Sobrio (sin texturas ni efectos)', terminal: 'Terminal verde' };

  var def = { tema: 'oscuro', contraste: false, tam: 14, sinAnimacion: false, sonido: { rele: false, teclado: false, campanilla: false }, propios: {} };
  var a = L.ajustes = Object.assign({}, def, L.guardado.leer('ajustes', {}));
  a.sonido = Object.assign({}, def.sonido, a.sonido || {});
  a.propios = a.propios || {};

  function raiz() { return document.documentElement; }

  function aplicar() {
    var r = raiz();
    var propio = a.propios[a.tema];
    // Las marcas de un tema propio se escriben como variables; un tema base solo cambia el atributo.
    MARCAS.map(function (m) { return m[0]; }).concat(['fuente', 'textura', 'radio']).forEach(function (k) { r.style.removeProperty('--' + k); });
    if (propio) {
      r.setAttribute('data-tema', 'propio');
      Object.keys(propio).forEach(function (k) { r.style.setProperty('--' + k, propio[k]); });
    } else {
      r.setAttribute('data-tema', TEMAS_BASE[a.tema] ? a.tema : 'oscuro');
    }
    if (a.contraste) r.setAttribute('data-contraste', 'alto'); else r.removeAttribute('data-contraste');
    if (a.sinAnimacion) r.setAttribute('data-sin-animacion', ''); else r.removeAttribute('data-sin-animacion');
    r.style.setProperty('--tam', a.tam + 'px');
    L.guardado.escribir('ajustes', a);
    L.emit('ajustes', a);
  }

  // Se aplica de inmediato para que no haya parpadeo de tema al cargar.
  aplicar();

  // --- sonido (RF-IDE-028 a RF-IDE-030): todo es opcional y el IDE funciona igual en silencio ------
  var audio = null;
  L.sonido = {
    cuenta: { rele: 0, teclado: 0, campanilla: 0 },
    activo: function (familia) { return !!a.sonido[familia]; },
    tocar: function (familia) {
      if (!a.sonido[familia]) return;
      L.sonido.cuenta[familia]++;
      try {
        var Ctx = window.AudioContext || window.webkitAudioContext;
        if (!Ctx) return;
        audio = audio || new Ctx();
        if (audio.state === 'suspended') audio.resume();
        var t = audio.currentTime, o = audio.createOscillator(), g = audio.createGain();
        var f = familia === 'rele' ? { tipo: 'square', hz: 140, dur: 0.025, vol: 0.05 } : familia === 'teclado' ? { tipo: 'triangle', hz: 900, dur: 0.02, vol: 0.04 } : { tipo: 'sine', hz: 1320, dur: 0.6, vol: 0.12 };
        o.type = f.tipo; o.frequency.value = f.hz;
        g.gain.setValueAtTime(f.vol, t); g.gain.exponentialRampToValueAtTime(0.0001, t + f.dur);
        o.connect(g); g.connect(audio.destination); o.start(t); o.stop(t + f.dur + 0.02);
      } catch (e) { /* sin audio: el IDE sigue funcionando */ }
    }
  };
  var ultimoRele = 0;
  L.on('estado', function (e) {
    var prev = L.sesion.previo;
    if (!e || !prev) return;
    if (e.instrucciones !== prev.instrucciones && Date.now() - ultimoRele > 60) { ultimoRele = Date.now(); L.sonido.tocar('rele'); }
    if (e.detenida && !prev.detenida) L.sonido.tocar('campanilla');
  });
  document.addEventListener('keydown', function (ev) {
    if (ev.target && (ev.target.tagName === 'TEXTAREA' || ev.target.id === 'consola-entrada') && ev.key.length === 1) L.sonido.tocar('teclado');
  });
  L.on('campanilla', function () { L.sonido.tocar('campanilla'); });

  // --- panel -----------------------------------------------------------------------------------------
  L.paneles.registrar({
    id: 'ajustes', titulo: 'Apariencia, sonido y accesibilidad', zona: 'der', colapsado: true,
    construir: function (cuerpo) {
      function fila(etiqueta, control) { return L.el('div', { class: 'fila' }, [L.el('label', {}, [etiqueta + ' ', control])]); }
      function check(id, texto, valor, alCambiar) {
        var c = L.el('input', { type: 'checkbox', id: id });
        c.checked = !!valor;
        c.addEventListener('change', function () { alCambiar(c.checked); aplicar(); });
        return L.el('div', { class: 'fila' }, [L.el('label', {}, [c, ' ' + texto])]);
      }

      var tema = L.el('select', { id: 'ajuste-tema', 'aria-label': 'Tema' });
      function llenarTemas() {
        tema.textContent = '';
        Object.keys(TEMAS_BASE).forEach(function (k) { tema.append(L.el('option', { value: k, texto: TEMAS_BASE[k] })); });
        Object.keys(a.propios).forEach(function (k) { tema.append(L.el('option', { value: k, texto: 'Propio: ' + k })); });
        tema.value = a.tema;
      }
      llenarTemas();
      tema.addEventListener('change', function () { a.tema = tema.value; aplicar(); pintarEditor(); });

      var tam = L.el('input', { type: 'range', id: 'ajuste-tam', min: '11', max: '28', step: '1', value: String(a.tam), 'aria-label': 'Tamaño del texto' });
      var tamTexto = L.el('span', { id: 'ajuste-tam-valor', texto: a.tam + ' px' });
      tam.addEventListener('input', function () { a.tam = +tam.value; tamTexto.textContent = a.tam + ' px'; aplicar(); });

      // Editor de temas propios (RF-IDE-026, RF-IDE-027): cada marca se puede cambiar y guardar.
      var editor = L.el('fieldset', { id: 'editor-tema' }, [L.el('legend', { texto: 'Crear un tema propio' })]);
      var nombre = L.el('input', { type: 'text', id: 'tema-nombre', 'aria-label': 'Nombre del tema', placeholder: 'Nombre del tema' });
      var entradas = {};
      function valorActual(k) { return getComputedStyle(raiz()).getPropertyValue('--' + k).trim(); }
      function aHex(c) {
        var m = /^#([0-9a-f]{3})$/i.exec(c);
        if (m) return '#' + m[1].split('').map(function (x) { return x + x; }).join('');
        if (/^#[0-9a-f]{6}$/i.test(c)) return c;
        var d = document.createElement('canvas').getContext('2d'); d.fillStyle = c; return d.fillStyle[0] === '#' ? d.fillStyle : '#808080';
      }
      function pintarEditor() {
        editor.querySelectorAll('.marca-fila').forEach(function (n) { n.remove(); });
        entradas = {};
        MARCAS.forEach(function (m) {
          var i = L.el('input', { type: 'color', id: 'marca-' + m[0], 'aria-label': m[1] });
          i.value = aHex(valorActual(m[0]));
          entradas[m[0]] = i;
          i.addEventListener('input', function () { raiz().style.setProperty('--' + m[0], i.value); });
          editor.append(L.el('div', { class: 'fila marca-fila' }, [L.el('label', {}, [i, ' ' + m[1]])]));
        });
        var fu = L.el('select', { id: 'marca-fuente', 'aria-label': 'Tipografía del editor' });
        Object.keys(FUENTES).forEach(function (k) { fu.append(L.el('option', { value: k, texto: FUENTES[k] })); });
        var tx = L.el('select', { id: 'marca-textura', 'aria-label': 'Textura de fondo' });
        Object.keys(TEXTURAS).forEach(function (k) { tx.append(L.el('option', { value: k, texto: TEXTURAS[k] })); });
        fu.addEventListener('change', function () { raiz().style.setProperty('--fuente', fu.value); });
        tx.addEventListener('change', function () { raiz().style.setProperty('--textura', tx.value); });
        entradas.fuente = fu; entradas.textura = tx;
        editor.append(L.el('div', { class: 'fila marca-fila' }, [L.el('label', {}, ['Tipografía ', fu])]), L.el('div', { class: 'fila marca-fila' }, [L.el('label', {}, ['Textura ', tx])]));
      }
      var guardarTema = L.el('button', {
        type: 'button', id: 'tema-guardar', texto: 'Guardar tema', onclick: function () {
          var n = nombre.value.trim();
          if (!n || TEMAS_BASE[n]) { L.estado('Pon un nombre distinto de los temas incluidos', 'aviso'); return; }
          var marcas = {};
          Object.keys(entradas).forEach(function (k) { marcas[k] = entradas[k].value; });
          a.propios[n] = marcas; a.tema = n; aplicar(); llenarTemas(); L.estado('Tema «' + n + '» guardado', 'ok');
        }
      });
      var borrarTema = L.el('button', {
        type: 'button', id: 'tema-borrar', texto: 'Borrar tema propio', onclick: function () {
          if (!a.propios[a.tema]) return;
          delete a.propios[a.tema]; a.tema = 'oscuro'; aplicar(); llenarTemas(); pintarEditor();
        }
      });
      var exportar = L.el('button', {
        type: 'button', id: 'tema-exportar', texto: 'Copiar tema (JSON)', onclick: function () {
          var t = JSON.stringify(a.propios[a.tema] || {}, null, 2);
          try { navigator.clipboard.writeText(t); L.estado('Tema copiado'); } catch (e) { window.prompt('Tema:', t); }
        }
      });
      editor.append(L.el('div', { class: 'fila' }, [nombre, guardarTema, borrarTema, exportar]));
      pintarEditor();

      // Atajos reasignables (RF-IDE-020)
      var tabla = L.el('table', { id: 'tabla-atajos', class: 'atajos' }, [L.el('caption', { texto: 'Atajos de teclado' })]);
      function pintarAtajos() {
        tabla.querySelectorAll('tr').forEach(function (n) { n.remove(); });
        L.atajos.ids().forEach(function (id) {
          var b = L.el('button', {
            type: 'button', 'data-atajo': id, texto: L.atajos.de(id) || '(sin atajo)', 'aria-label': 'Reasignar «' + L.atajos.nombres[id] + '», ahora ' + (L.atajos.de(id) || 'sin atajo'),
            onclick: function () {
              b.textContent = 'Pulsa la combinación… (Esc cancela)';
              L.capturandoAtajo = true;
              function tecla(ev) {
                ev.preventDefault(); ev.stopPropagation();
                if (ev.key === 'Escape') { fin(); return; }
                var c = L.atajos.combinacion(ev);
                if (!c) return;
                L.atajos.fijar(id, c); fin();
              }
              function fin() { document.removeEventListener('keydown', tecla, true); L.capturandoAtajo = false; pintarAtajos(); }
              document.addEventListener('keydown', tecla, true);
            }
          });
          tabla.append(L.el('tr', {}, [L.el('th', { scope: 'row', texto: L.atajos.nombres[id] }), L.el('td', {}, [b])]));
        });
      }
      pintarAtajos();
      L.on('atajos', pintarAtajos);
      var restaurar = L.el('button', { type: 'button', id: 'atajos-restaurar', texto: 'Restaurar atajos', onclick: function () { L.atajos.restaurar(); } });

      cuerpo.append(
        fila('Tema', tema),
        check('ajuste-contraste', 'Alto contraste', a.contraste, function (v) { a.contraste = v; }),
        L.el('div', { class: 'fila' }, [L.el('label', {}, ['Tamaño del texto ', tam, ' ']), tamTexto]),
        check('ajuste-animacion', 'Reducir animaciones', a.sinAnimacion, function (v) { a.sinAnimacion = v; }),
        L.el('fieldset', { id: 'ajustes-sonido' }, [L.el('legend', { texto: 'Sonido (todo funciona sin él)' }),
          check('sonido-rele', 'Relé (al ejecutar instrucciones)', a.sonido.rele, function (v) { a.sonido.rele = v; }),
          check('sonido-teclado', 'Teclado (al escribir)', a.sonido.teclado, function (v) { a.sonido.teclado = v; }),
          check('sonido-campanilla', 'Campanilla (al detenerse la máquina)', a.sonido.campanilla, function (v) { a.sonido.campanilla = v; })]),
        editor, tabla, restaurar,
        L.el('button', { type: 'button', id: 'disposicion-restaurar', texto: 'Restaurar la disposición de paneles', onclick: function () { L.paneles.restaurar(); } })
      );
    }
  });
})();
