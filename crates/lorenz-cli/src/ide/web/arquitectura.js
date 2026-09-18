// Editor visual de arquitectura (RF-ARQ-038 a 041): un diagrama de bloques que sale del texto de la
// descripcion y que, al cambiarlo, cambia ese mismo texto sin perder comentarios ni formato.
'use strict';
(function () {
  var L = window.L;
  var NS = 'http://www.w3.org/2000/svg';
  var ANCHO = 168, ALTO = 52, SEP = 16;
  var COLUMNAS = { registro: 0, bandera: 1, region: 2, dispositivo: 3 };
  var ROTULO = { registro: 'Registro', bandera: 'Bandera', region: 'Región de memoria', dispositivo: 'Dispositivo' };
  var PLURAL = { registro: 'Registros', bandera: 'Banderas', region: 'Memoria', dispositivo: 'Dispositivos' };

  function svg(tag, attrs, hijos) {
    var e = document.createElementNS(NS, tag);
    Object.keys(attrs || {}).forEach(function (k) { e.setAttribute(k, attrs[k]); });
    (hijos || []).forEach(function (h) { e.append(h); });
    return e;
  }
  function campo(b, k) {
    var c = b.campos.filter(function (x) { return x.clave === k; })[0];
    return c ? (c.valor != null ? c.valor : c.texto) : null;
  }
  function hexa(n) { return '0x' + L.hex(n, 4); }

  function subtitulo(b) {
    switch (b.clase) {
      case 'registro': return campo(b, 'bits') + ' bits · ' + campo(b, 'rol');
      case 'bandera': return 'bit ' + campo(b, 'bit');
      case 'region': return hexa(campo(b, 'desde')) + '–' + hexa(campo(b, 'hasta')) + ' · ' + campo(b, 'acceso');
      case 'dispositivo': return campo(b, 'tipo') || '';
      case 'maquina': return campo(b, 'bits_palabra') + ' bits de palabra · ' + campo(b, 'bits_direccion') + ' de dirección';
      default: return '';
    }
  }

  L.paneles.registrar({
    id: 'arquitectura', titulo: 'Editor visual de arquitectura', zona: 'centro', colapsado: true,
    construir: function (cuerpo) {
      var estado = { ruta: null, bloques: [], errores: [], elegido: null, pila: [], valida: true };
      var pos = L.guardado.leer('arq-posiciones', {});
      var aviso = L.el('p', { id: 'arq-aviso', class: 'suave', role: 'status', 'aria-live': 'polite', texto: 'Abre una descripción de arquitectura (.toml) en el editor.' });
      var paleta = L.el('div', { class: 'fila', id: 'arq-paleta', role: 'toolbar', 'aria-label': 'Bloques que se pueden añadir' });
      var lienzo = svg('svg', { id: 'arq-lienzo', width: '100%', height: '260', role: 'group', 'aria-label': 'Diagrama de bloques de la arquitectura' });
      var marco = L.el('div', { id: 'arq-marco', class: 'arq-marco' }, [lienzo]);
      var formulario = L.el('div', { id: 'arq-formulario', class: 'arq-formulario', 'aria-live': 'polite' });
      var errores = L.el('ul', { id: 'arq-errores', class: 'lista', 'aria-label': 'Errores de la descripción' });
      var deshacer = L.el('button', { type: 'button', id: 'arq-deshacer', texto: '↶ Deshacer', disabled: true, title: 'Deshace el último cambio hecho desde el diagrama' });

      // --- comunicacion con el editor ----------------------------------------------------------
      function rutaActual() { var r = L.editor && L.editor.activo(); return r && /\.toml$/.test(r) ? r : null; }

      var pendiente = null;
      function analizar() {
        var ruta = rutaActual();
        if (!ruta) { estado.ruta = null; estado.bloques = []; estado.errores = []; pintar(); return Promise.resolve(); }
        estado.ruta = ruta;
        return L.api('POST', '/api/arquitectura/analizar', { texto: L.editor.texto(), ruta: ruta }).then(function (r) {
          if (!r.ok || rutaActual() !== ruta) return;
          aplicarVista(r.datos);
        });
      }
      function aplicarVista(d) {
        estado.bloques = d.bloques; estado.errores = d.errores; estado.valida = d.valida;
        if (estado.elegido != null && !estado.bloques[estado.elegido]) estado.elegido = null;
        pintar();
      }
      function programar() { clearTimeout(pendiente); pendiente = setTimeout(analizar, 250); }

      function cambiar(cambio) {
        var ruta = rutaActual();
        if (!ruta) return Promise.resolve(false);
        var antes = L.editor.texto();
        return L.api('POST', '/api/arquitectura/editar', { texto: antes, ruta: ruta, cambio: cambio }).then(function (r) {
          if (!r.ok) { aviso.textContent = 'No se pudo aplicar el cambio: ' + (r.datos.error || r.estado); L.estado(aviso.textContent, 'error'); return false; }
          estado.pila.push({ ruta: ruta, texto: antes });
          L.editor.ponerTexto(ruta, r.datos.texto);
          aplicarVista(r.datos);
          deshacer.disabled = false;
          return true;
        });
      }
      deshacer.addEventListener('click', function () {
        var u = estado.pila.pop();
        if (!u) return;
        if (L.editor.activo() !== u.ruta) L.editor.abrir(u.ruta);
        L.editor.ponerTexto(u.ruta, u.texto);
        deshacer.disabled = estado.pila.length === 0;
        analizar();
      });

      // --- dibujo --------------------------------------------------------------------------------
      function clave(b) { return estado.ruta + '|' + b.clase + '|' + b.nombre; }

      function disponer() {
        var filas = { 0: 0, 1: 0, 2: 0, 3: 0 }, lugar = [], otros = 0;
        estado.bloques.forEach(function (b, i) {
          if (b.clase === 'instruccion') return;
          var p = pos[clave(b)];
          var x, y;
          if (b.clase in COLUMNAS) { x = 10 + COLUMNAS[b.clase] * (ANCHO + SEP); y = 78 + filas[COLUMNAS[b.clase]]++ * (ALTO + SEP); }
          else { x = 10 + otros++ * (ANCHO + SEP); y = 10; }
          if (p) { x = p.x; y = p.y; }
          lugar[i] = { x: x, y: y };
        });
        return lugar;
      }

      function pintar() {
        lienzo.textContent = '';
        errores.textContent = '';
        var instrucciones = estado.bloques.filter(function (b) { return b.clase === 'instruccion'; }).length;
        if (!estado.ruta) { aviso.textContent = 'Abre una descripción de arquitectura (.toml) en el editor.'; formulario.textContent = ''; return; }
        aviso.textContent = estado.ruta + ' · ' + estado.bloques.length + ' bloques · ' + instrucciones + ' instrucciones · ' + (estado.valida ? 'válida' : estado.errores.length + ' error(es)');
        var lugar = disponer(), maxX = 600, maxY = 200;
        Object.keys(PLURAL).forEach(function (c) {
          var t = svg('text', { x: 10 + COLUMNAS[c] * (ANCHO + SEP), y: 66, class: 'arq-columna' }); t.textContent = PLURAL[c]; lienzo.append(t);
        });
        estado.bloques.forEach(function (b, i) {
          if (!lugar[i]) return;
          var malos = estado.errores.filter(function (e) { return e.bloque === i; });
          var g = svg('g', {
            class: 'arq-bloque clase-' + b.clase + (malos.length ? ' con-error' : '') + (estado.elegido === i ? ' elegido' : ''),
            transform: 'translate(' + lugar[i].x + ',' + lugar[i].y + ')', tabindex: '0', role: 'button',
            'data-clase': b.clase, 'data-nombre': b.nombre, 'data-indice': i,
            'aria-label': (ROTULO[b.clase] || 'Bloque') + ' ' + b.nombre + (subtitulo(b) ? ', ' + subtitulo(b) : '') + (malos.length ? '. Con ' + malos.length + ' error(es): ' + malos[0].mensaje : '')
          });
          g.append(svg('rect', { width: ANCHO, height: ALTO, rx: 6 }));
          var t1 = svg('text', { x: 10, y: 21, class: 'arq-nombre' }); t1.textContent = (malos.length ? '⚠ ' : '') + b.nombre; g.append(t1);
          var t2 = svg('text', { x: 10, y: 39, class: 'arq-sub' }); t2.textContent = subtitulo(b); g.append(t2);
          if (malos.length) { var tt = svg('title'); tt.textContent = malos.map(function (m) { return m.mensaje; }).join('\n'); g.append(tt); }
          arrastrable(g, b, i);
          g.addEventListener('keydown', function (ev) {
            if (ev.key === 'Enter' || ev.key === ' ') { ev.preventDefault(); elegir(i); }
            else if (ev.key === 'Delete' && quitable(b)) { ev.preventDefault(); quitar(b); }
          });
          lienzo.append(g);
          maxX = Math.max(maxX, lugar[i].x + ANCHO + 10); maxY = Math.max(maxY, lugar[i].y + ALTO + 10);
        });
        lienzo.setAttribute('viewBox', '0 0 ' + maxX + ' ' + maxY);
        lienzo.setAttribute('height', String(Math.min(420, maxY)));
        lienzo.setAttribute('data-bloques', String(estado.bloques.length));
        estado.errores.forEach(function (e) {
          var li = L.el('li', { class: 'error-arq', 'data-linea': e.linea }, [
            L.el('button', { type: 'button', class: 'enlace', texto: '✖ línea ' + e.linea + ' [' + e.codigo + '] ' + e.mensaje, onclick: function () {
              if (e.bloque != null) elegir(e.bloque);
              L.editor.irALinea(e.linea);
            } })]);
          errores.append(li);
        });
        pintarFormulario();
      }

      function quitable(b) { return ['registro', 'region', 'dispositivo', 'bandera'].indexOf(b.clase) >= 0; }
      function quitar(b) { return cambiar({ tipo: 'quitar', ruta: b.ruta }).then(function (ok) { if (ok) { estado.elegido = null; pintar(); } }); }
      function elegir(i) { estado.elegido = i; pintar(); var g = lienzo.querySelector('[data-indice="' + i + '"]'); if (g) g.focus(); }

      function arrastrable(g, b, i) {
        g.addEventListener('pointerdown', function (ev) {
          if (ev.button !== 0) return;
          var r = lienzo.getBoundingClientRect(), vb = lienzo.viewBox.baseVal, k = vb.width / r.width;
          var m = /translate\(([-\d.]+),([-\d.]+)\)/.exec(g.getAttribute('transform'));
          var x0 = +m[1], y0 = +m[2], px = ev.clientX, py = ev.clientY, movido = false;
          g.setPointerCapture(ev.pointerId);
          function mover(e) {
            var dx = (e.clientX - px) * k, dy = (e.clientY - py) * k;
            if (Math.abs(dx) + Math.abs(dy) > 3) movido = true;
            if (movido) g.setAttribute('transform', 'translate(' + Math.max(0, x0 + dx) + ',' + Math.max(0, y0 + dy) + ')');
          }
          function soltar() {
            g.removeEventListener('pointermove', mover); g.removeEventListener('pointerup', soltar);
            if (movido) {
              var n = /translate\(([-\d.]+),([-\d.]+)\)/.exec(g.getAttribute('transform'));
              pos[clave(b)] = { x: Math.round(+n[1]), y: Math.round(+n[2]) };
              L.guardado.escribir('arq-posiciones', pos);
              lienzo.setAttribute('data-movidos', String(Object.keys(pos).length));
            } else elegir(i);
          }
          g.addEventListener('pointermove', mover); g.addEventListener('pointerup', soltar);
        });
      }

      function pintarFormulario() {
        formulario.textContent = '';
        var b = estado.bloques[estado.elegido];
        if (!b) { formulario.append(L.el('p', { class: 'suave', texto: 'Elige un bloque para ver y cambiar sus campos.' })); return; }
        formulario.append(L.el('h3', { texto: (ROTULO[b.clase] || 'Bloque') + ' ' + b.nombre + ' (línea ' + b.linea + ')' }));
        b.campos.forEach(function (c) {
          var id = 'arq-campo-' + c.clave;
          var simple = c.valor !== null && c.valor !== undefined;
          var entrada = L.el('input', { type: 'text', id: id, value: c.texto.replace(/^"|"$/g, ''), 'data-clave': c.clave, disabled: !simple, size: '14', 'aria-label': c.clave });
          if (simple) {
            var confirmar = function () {
              var t = entrada.value.trim(), v;
              if (typeof c.valor === 'number') { v = /^0x/i.test(t) ? parseInt(t, 16) : parseInt(t, 10); if (isNaN(v)) { aviso.textContent = c.clave + ' debe ser un número'; entrada.value = c.texto; return; } }
              else if (typeof c.valor === 'boolean') v = t === 'true'; else v = t;
              if (v === c.valor) return;
              cambiar({ tipo: 'fijar', ruta: b.ruta.concat([c.clave]), valor: v });
            };
            entrada.addEventListener('change', confirmar);
          }
          formulario.append(L.el('label', { class: 'campo-arq' }, [c.clave + ' ', entrada, simple ? '' : L.el('span', { class: 'suave', texto: ' (se edita en el texto)' })]));
        });
        if (quitable(b)) formulario.append(L.el('button', { type: 'button', id: 'arq-quitar', texto: 'Quitar este bloque', onclick: function () { quitar(b); } }));
      }

      // --- paleta: arrastrar o pulsar para anadir -------------------------------------------------
      function unico(prefijo, clase) {
        var usados = estado.bloques.filter(function (b) { return b.clase === clase; }).map(function (b) { return b.nombre; });
        var n = usados.length + 1;
        while (usados.indexOf(prefijo + n) >= 0) n++;
        return prefijo + n;
      }
      function nuevo(clase) {
        var maq = estado.bloques.filter(function (b) { return b.clase === 'maquina'; })[0];
        var bits = maq ? (campo(maq, 'bits_palabra') || 8) : 8;
        var nombre;
        if (clase === 'registro') { nombre = unico('R', clase); return { nombre: nombre, campos: [['nombre', nombre], ['bits', bits], ['rol', 'general'], ['inicial', 0]], lista: 'registro' }; }
        if (clase === 'bandera') { nombre = unico('F', clase); return { nombre: nombre, campos: [['nombre', nombre], ['bit', estado.bloques.filter(function (b) { return b.clase === 'bandera'; }).length], ['descripcion', nombre]], lista: 'flags' }; }
        if (clase === 'region') {
          var fin = -1;
          estado.bloques.forEach(function (b) { if (b.clase === 'region') fin = Math.max(fin, campo(b, 'hasta')); });
          nombre = unico('mem', clase);
          return { nombre: nombre, campos: [['nombre', nombre], ['desde', fin + 1], ['hasta', fin + 256], ['acceso', 'rw']], lista: 'region' };
        }
        nombre = unico('disp', clase);
        return { nombre: nombre, campos: [['nombre', nombre], ['tipo', 'temporizador'], ['periodo', 1000]], lista: 'dispositivo' };
      }
      function anadir(clase, lugar) {
        if (!rutaActual()) { aviso.textContent = 'Abre primero una descripción (.toml) en el editor.'; return; }
        var n = nuevo(clase);
        cambiar({ tipo: 'anadir', lista: n.lista, campos: n.campos.map(function (c) { return { clave: c[0], valor: c[1] }; }) }).then(function (ok) {
          if (!ok) return;
          if (lugar) { pos[estado.ruta + '|' + clase + '|' + n.nombre] = lugar; L.guardado.escribir('arq-posiciones', pos); }
          var i = -1;
          estado.bloques.forEach(function (b, j) { if (b.clase === clase && b.nombre === n.nombre) i = j; });
          if (i >= 0) elegir(i); else pintar();
        });
      }
      Object.keys(ROTULO).forEach(function (c) {
        var b = L.el('button', { type: 'button', draggable: 'true', 'data-nuevo': c, texto: '+ ' + ROTULO[c], title: 'Pulsa o arrastra al diagrama para añadir', onclick: function () { anadir(c, null); } });
        b.addEventListener('dragstart', function (ev) { ev.dataTransfer.setData('text/plain', 'lorenz-bloque:' + c); ev.dataTransfer.effectAllowed = 'copy'; });
        paleta.append(b);
      });
      paleta.append(deshacer);
      marco.addEventListener('dragover', function (ev) { ev.preventDefault(); ev.stopPropagation(); ev.dataTransfer.dropEffect = 'copy'; marco.classList.add('sobre'); });
      marco.addEventListener('dragleave', function () { marco.classList.remove('sobre'); });
      marco.addEventListener('drop', function (ev) {
        ev.preventDefault(); ev.stopPropagation(); marco.classList.remove('sobre');
        var t = ev.dataTransfer.getData('text/plain') || '';
        var m = /^lorenz-bloque:(\w+)$/.exec(t);
        if (!m) return;
        var r = lienzo.getBoundingClientRect(), vb = lienzo.viewBox.baseVal, k = vb.width / r.width;
        anadir(m[1], { x: Math.max(0, Math.round((ev.clientX - r.left) * k - ANCHO / 2)), y: Math.max(0, Math.round((ev.clientY - r.top) * k - ALTO / 2)) });
      });

      // --- borrador desde texto (RF-ECO-045) ---------------------------------------------------
      var bTexto = L.el('textarea', { id: 'borrador-texto', rows: '3', 'aria-label': 'Describe la máquina con tus palabras', placeholder: 'Por ejemplo: procesador de 16 bits con 8 registros, 64 KiB de memoria, suma y resta' });
      var bCrear = L.el('button', { type: 'button', id: 'borrador-crear', texto: 'Crear borrador' });
      var bAviso = L.el('p', { id: 'borrador-aviso', class: 'suave', role: 'status', 'aria-live': 'polite' });
      var bLista = L.el('ul', { id: 'borrador-supuestos', class: 'lista', 'aria-label': 'Supuestos del borrador' });
      var borrador = L.el('details', { id: 'arq-borrador' }, [
        L.el('summary', { texto: 'Borrador desde una descripción' }),
        L.el('p', { class: 'suave', texto: 'Sin IA: lee números y palabras clave. Lo que no diga, lo supone y te lo cuenta; después lo editas como cualquier descripción.' }),
        bTexto, L.el('div', { class: 'fila' }, [bCrear]), bAviso, bLista
      ]);
      function nombreLibre() {
        var usados = (L.proyecto.archivos || []).map(function (a) { return a.ruta; });
        var n = 'borrador.toml', i = 2;
        while (usados.indexOf(n) >= 0) { n = 'borrador-' + i + '.toml'; i++; }
        return n;
      }
      bCrear.addEventListener('click', function () {
        bLista.textContent = '';
        if (!bTexto.value.trim()) { bAviso.textContent = 'Escribe primero la descripción.'; return; }
        L.api('POST', '/api/arquitectura/borrador', { descripcion: bTexto.value }).then(function (r) {
          if (!r.ok) { bAviso.textContent = 'No se pudo: ' + (r.datos.error || r.estado); return; }
          var ruta = nombreLibre();
          return L.api('PUT', '/api/archivo', { ruta: ruta, texto: r.datos.texto }).then(function (w) {
            if (!w.ok) { bAviso.textContent = 'No se pudo guardar ' + ruta + ': ' + (w.datos.error || w.estado); return; }
            (r.datos.supuestos || []).forEach(function (t) { bLista.append(L.el('li', { texto: t })); });
            bAviso.textContent = 'Borrador guardado en ' + ruta + '. Revisa estos supuestos:';
            return L.cargarProyecto().then(function () { L.emit('abrir-archivo', { ruta: ruta }); });
          });
        });
      });

      cuerpo.append(aviso, borrador, paleta, marco, formulario, errores);
      L.on('texto-editado', function (d) { if (d.ruta === rutaActual()) programar(); });
      L.on('editor-activo', function () { estado.elegido = null; analizar(); });
      L.on('disposicion', function () { if (!cuerpo.closest('.panel').classList.contains('colapsado')) analizar(); });
      L.arquitectura = { analizar: analizar, estado: function () { return estado; } };
    }
  });
})();
