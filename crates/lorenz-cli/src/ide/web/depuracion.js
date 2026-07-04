// Depuracion grafica avanzada: instante y velocidad, camino del dato, mapa de memoria, linea de
// tiempo de eventos y consola de ordenes (RF-DEP-022, 033, 034, 036, 038, 044).
'use strict';
(function () {
  var L = window.L;
  var A = L.depurador, sesion = L.sesion;

  // --- instante (control deslizante) y velocidad -------------------------------------------------
  L.paneles.registrar({
    id: 'instante', titulo: 'Instante y velocidad', zona: 'abajo',
    construir: function (cuerpo) {
      var desliz = L.el('input', { type: 'range', id: 'instante-desliz', min: '0', max: '0', value: '0', 'aria-label': 'Instante de la ejecución', disabled: true });
      var texto = L.el('output', { id: 'instante-texto', class: 'mono', 'aria-live': 'polite' });
      var vel = L.el('select', { id: 'velocidad', 'aria-label': 'Velocidad de ejecución' }, [
        ['0', 'Rápida'], ['50', 'Lenta (50 ms por instrucción)'], ['250', 'Muy lenta (250 ms)'], ['800', 'Paso a paso animado (800 ms)']
      ].map(function (o) { return L.el('option', { value: o[0], texto: o[1] }); }));
      vel.value = String(L.guardado.leer('velocidad', 0));
      sesion.lento = +vel.value;
      vel.addEventListener('change', function () { sesion.lento = +vel.value; L.guardado.escribir('velocidad', sesion.lento); });
      var pendiente = null;
      desliz.addEventListener('input', function () {
        texto.textContent = 'Instante ' + desliz.value + ' de ' + desliz.max;
        clearTimeout(pendiente);
        pendiente = setTimeout(function () { A.editar('instante', { n: +desliz.value }); }, 80);
      });
      cuerpo.append(L.el('div', { class: 'fila' }, [L.el('label', {}, ['Instante ', desliz]), texto]), L.el('div', { class: 'fila' }, [L.el('label', {}, ['Velocidad ', vel])]));
      L.on('estado', function (e) {
        if (!e) { desliz.disabled = true; desliz.max = '0'; desliz.value = '0'; texto.textContent = ''; return; }
        desliz.disabled = sesion.corriendo; desliz.max = String(Math.max(e.traza_longitud, e.instante));
        desliz.value = String(e.instante);
        texto.textContent = 'Instante ' + e.instante + ' de ' + desliz.max;
      });
    }
  });

  // --- camino del dato (RF-DEP-033) -------------------------------------------------------------------
  var NS = 'http://www.w3.org/2000/svg';
  function svg(tag, attrs, hijos) {
    var e = document.createElementNS(NS, tag);
    Object.keys(attrs || {}).forEach(function (k) { e.setAttribute(k, attrs[k]); });
    (hijos || []).forEach(function (h) { e.append(h); });
    return e;
  }
  var CAJAS = { memoria: [20, 30, 'Memoria'], registros: [200, 30, 'Registros'], alu: [200, 120, 'Unidad aritmética'], pc: [380, 120, 'PC'], puertos: [380, 30, 'Puertos E/S'] };
  var FLUJOS = [
    ['memoria', 'registros', 'carga'], ['registros', 'memoria', 'escritura'], ['registros', 'alu', 'operandos'],
    ['alu', 'registros', 'resultado'], ['alu', 'pc', 'salto'], ['puertos', 'registros', 'entrada'], ['registros', 'puertos', 'salida']
  ];
  function centro(id) { var c = CAJAS[id]; return [c[0] + 50, c[1] + 20]; }

  function flujosDe(x) {
    var ops = (x.operaciones || []).join(' ; ');
    var f = {};
    var lee = /=[^;]*mem\[/i.test(ops);
    if (x.escrituras && x.escrituras.length) { f.escritura = true; f.operandos = true; }
    if (x.registros && x.registros.length) {
      if (lee) f.carga = true; else { f.operandos = true; f.resultado = true; }
    }
    if (/\bin\[/i.test(ops)) f.entrada = true;
    if (/\bout\[/i.test(ops)) f.salida = true;
    if (x.salto) f.salto = true;
    return f;
  }

  L.paneles.registrar({
    id: 'dato', titulo: 'Camino del dato', zona: 'abajo',
    construir: function (cuerpo) {
      var lienzo = svg('svg', { viewBox: '0 0 500 190', width: '100%', role: 'img', 'aria-label': 'Camino de los datos de la próxima instrucción', id: 'dato-svg' });
      var lineas = {};
      FLUJOS.forEach(function (f) {
        var a = centro(f[0]), b = centro(f[1]);
        var desv = (f[2] === 'resultado' || f[2] === 'escritura' || f[2] === 'salida') ? 14 : 0;
        var d = 'M' + (a[0] + desv) + ',' + (a[1] + desv) + ' L' + (b[0] + desv) + ',' + (b[1] + desv);
        var p = svg('path', { d: d, class: 'flujo', 'data-flujo': f[2], 'marker-end': 'url(#punta)' });
        lineas[f[2]] = p;
      });
      var defs = svg('defs', {}, [svg('marker', { id: 'punta', markerWidth: '8', markerHeight: '8', refX: '6', refY: '3', orient: 'auto' }, [svg('path', { d: 'M0,0 L6,3 L0,6 z', class: 'punta' })])]);
      lienzo.append(defs);
      Object.keys(lineas).forEach(function (k) { lienzo.append(lineas[k]); });
      Object.keys(CAJAS).forEach(function (k) {
        var c = CAJAS[k];
        lienzo.append(svg('rect', { x: c[0], y: c[1], width: 100, height: 40, rx: 4, class: 'caja', 'data-caja': k }));
        var t = svg('text', { x: c[0] + 50, y: c[1] + 25, 'text-anchor': 'middle', class: 'caja-texto' }); t.textContent = c[2]; lienzo.append(t);
      });
      var rotulo = L.el('p', { id: 'dato-rotulo', class: 'mono', role: 'status', 'aria-live': 'polite' });
      var detalle = L.el('p', { id: 'dato-detalle', class: 'suave' });
      cuerpo.append(lienzo, rotulo, detalle);

      var etiquetas = { carga: 'Memoria → Registros (carga)', escritura: 'Registros → Memoria (escritura)', operandos: 'Registros → Unidad aritmética', resultado: 'Unidad aritmética → Registros', salto: 'Unidad aritmética → PC (salto)', entrada: 'Puertos → Registros (entrada)', salida: 'Registros → Puertos (salida)' };
      function animar(activos) {
        Object.keys(lineas).forEach(function (k) {
          var on = !!activos[k];
          lineas[k].classList.toggle('activo', on);
          lineas[k].setAttribute('aria-label', etiquetas[k] + (on ? ': activo' : ''));
          // Los puntos que viajan por el camino: se recrean en cada paso.
          lienzo.querySelectorAll('.dato-punto[data-flujo="' + k + '"]').forEach(function (n) { n.remove(); });
          if (on && !document.documentElement.hasAttribute('data-sin-animacion')) {
            var punto = svg('circle', { r: 5, class: 'dato-punto', 'data-flujo': k });
            punto.append(svg('animateMotion', { dur: '0.9s', repeatCount: '2', path: lineas[k].getAttribute('d') }));
            lienzo.append(punto);
          }
        });
        Object.keys(CAJAS).forEach(function (c) {
          var usadas = { memoria: ['carga', 'escritura'], registros: ['carga', 'escritura', 'operandos', 'resultado', 'entrada', 'salida'], alu: ['operandos', 'resultado', 'salto'], pc: ['salto'], puertos: ['entrada', 'salida'] }[c];
          var r = lienzo.querySelector('rect[data-caja="' + c + '"]');
          r.classList.toggle('activa', usadas.some(function (k) { return activos[k]; }));
        });
        var nombres = Object.keys(activos).filter(function (k) { return activos[k]; }).map(function (k) { return etiquetas[k]; });
        rotulo.textContent = nombres.length ? nombres.join(' · ') : 'Sin movimiento de datos';
      }
      function pedir() {
        if (!sesion.estado) { animar({}); rotulo.textContent = 'Sin sesión'; detalle.textContent = ''; return; }
        A.pedir('explicacion').then(function (r) {
          if (!r.ok) { animar({}); rotulo.textContent = r.datos.error || ''; return; }
          detalle.textContent = '0x' + L.hex(r.datos.direccion, 4) + '  ' + r.datos.texto + (r.datos.descripcion ? ' — ' + r.datos.descripcion : '');
          animar(flujosDe(r.datos));
        });
      }
      L.on('estado', pedir);
      animar({});
    }
  });

  // --- mapa de memoria (RF-DEP-036) ---------------------------------------------------------------------
  L.paneles.registrar({
    id: 'mapa', titulo: 'Mapa de memoria', zona: 'abajo', colapsado: true,
    construir: function (cuerpo) {
      var lienzo = L.el('canvas', { id: 'mapa-lienzo', width: '512', height: '160', tabindex: '0', role: 'img', 'aria-label': 'Mapa de la memoria por regiones y ocupación' });
      var leyenda = L.el('ul', { id: 'mapa-leyenda', class: 'lista' });
      var pista = L.el('p', { class: 'suave', id: 'mapa-pista', role: 'status', texto: 'Pasa el ratón o pulsa un bloque para ir a esa dirección.' });
      cuerpo.append(lienzo, pista, leyenda);
      var celdas = [], ctx = lienzo.getContext('2d');
      function color(r) { return r.dispositivo ? [120, 170, 255] : (r.escritura ? [111, 207, 142] : [224, 164, 58]); }
      function pintar(m) {
        var total = m.regiones.reduce(function (s, r) { return s + r.ocupacion.length; }, 0) || 1;
        var lado = Math.max(4, Math.floor(Math.sqrt(lienzo.width * lienzo.height / total)) - 1);
        var cols = Math.floor(lienzo.width / (lado + 1));
        var filas = Math.ceil(total / cols);
        lienzo.height = Math.max(160, filas * (lado + 1) + 2);
        ctx.clearRect(0, 0, lienzo.width, lienzo.height);
        celdas = []; leyenda.textContent = '';
        var n = 0;
        m.regiones.forEach(function (r) {
          var c = color(r);
          leyenda.append(L.el('li', {}, [L.el('span', { class: 'mono', texto: r.nombre + ' 0x' + L.hex(r.desde, 4) + '–0x' + L.hex(r.hasta, 4) + ' ' }),
            L.el('span', { class: 'suave', texto: (r.dispositivo ? 'dispositivo ' + r.dispositivo : (r.escritura ? 'lectura/escritura' : 'solo lectura')) })]));
          r.ocupacion.forEach(function (oc, i) {
            var x = (n % cols) * (lado + 1), y = Math.floor(n / cols) * (lado + 1);
            var frac = Math.min(1, oc / m.bloque);
            ctx.fillStyle = 'rgba(' + c[0] + ',' + c[1] + ',' + c[2] + ',' + (0.15 + 0.85 * frac) + ')';
            ctx.fillRect(x, y, lado, lado);
            if (!r.escritura && !r.dispositivo) { ctx.strokeStyle = '#e0a43a'; ctx.strokeRect(x + .5, y + .5, lado - 1, lado - 1); }
            var ini = r.desde + i * m.bloque;
            celdas.push({ x: x, y: y, lado: lado, dir: ini, region: r.nombre, ocupadas: oc });
            if (m.pc >= ini && m.pc < ini + m.bloque && m.pc <= r.hasta) { ctx.fillStyle = '#fff'; ctx.fillRect(x + lado / 2 - 1, y + lado / 2 - 1, 3, 3); celdas[celdas.length - 1].pc = true; }
            if (m.sp != null && m.sp >= ini && m.sp < ini + m.bloque && m.sp <= r.hasta) { ctx.strokeStyle = '#ff6b6b'; ctx.strokeRect(x, y, lado, lado); celdas[celdas.length - 1].sp = true; }
            n++;
          });
        });
        lienzo.setAttribute('data-bloques', String(celdas.length));
      }
      function bajo(ev) {
        var r = lienzo.getBoundingClientRect(), k = lienzo.width / r.width;
        var x = (ev.clientX - r.left) * k, y = (ev.clientY - r.top) * k;
        return celdas.filter(function (c) { return x >= c.x && x < c.x + c.lado + 1 && y >= c.y && y < c.y + c.lado + 1; })[0];
      }
      lienzo.addEventListener('mousemove', function (ev) {
        var c = bajo(ev);
        pista.textContent = c ? c.region + ' 0x' + L.hex(c.dir, 4) + ' · ' + c.ocupadas + ' celdas ocupadas' + (c.pc ? ' · aquí está el PC' : '') + (c.sp ? ' · aquí está la pila' : '') : 'Pasa el ratón o pulsa un bloque.';
      });
      lienzo.addEventListener('click', function (ev) { var c = bajo(ev); if (c && L.memoria) L.memoria.irA(c.dir); });
      function tomar() {
        if (!sesion.estado) { ctx.clearRect(0, 0, lienzo.width, lienzo.height); return; }
        if (cuerpo.closest('.panel').classList.contains('colapsado')) return;
        A.pedir('mapa?bloque=64').then(function (r) { if (r.ok) pintar(r.datos); });
      }
      L.on('estado', tomar);
      cuerpo.closest('.panel') ; L.on('disposicion', tomar);
      lienzo.mapa = tomar;
    }
  });

  // --- linea de tiempo de eventos (RF-DEP-038) -----------------------------------------------------------------
  var registro = [];   // eventos del cliente: paradas y salida por la UART
  L.paneles.registrar({
    id: 'linea-tiempo', titulo: 'Línea de tiempo de eventos', zona: 'abajo', colapsado: true,
    construir: function (cuerpo) {
      var lista = L.el('ul', { class: 'lista', id: 'eventos-lista', 'aria-label': 'Eventos en orden', role: 'list' });
      var vacio = L.el('p', { class: 'suave', id: 'eventos-vacio', texto: 'Sin eventos todavía.' });
      cuerpo.append(vacio, lista);
      var vistoUart = 0, ultimaParada = '';
      function pintar(extra) {
        var todos = extra.concat(registro).sort(function (a, b) { return a.instruccion - b.instruccion; });
        lista.textContent = ''; vacio.hidden = todos.length > 0;
        todos.forEach(function (ev) {
          lista.append(L.el('li', { role: 'listitem', 'data-tipo': ev.tipo }, [
            L.el('span', { class: 'mono', texto: 'instr ' + ev.instruccion + ' · ' }), L.el('span', { class: 'etiqueta-evento', texto: ev.etiqueta + ' ' }), L.el('span', { texto: ev.texto })]));
        });
      }
      function tomar(e) {
        if (!e) { registro = []; vistoUart = 0; ultimaParada = ''; pintar([]); return; }
        if (e.detenida && ultimaParada !== 'detenida:' + e.instrucciones) { ultimaParada = 'detenida:' + e.instrucciones; registro.push({ instruccion: e.instrucciones, tipo: 'parada', etiqueta: '■ PARADA', texto: 'La máquina se detuvo' }); }
        A.pedir('eventos').then(function (r) {
          var ints = (r.datos.interrupciones || []).map(function (i) { return { instruccion: i.instruccion, tipo: 'interrupcion', etiqueta: '⚡ INTERRUPCIÓN', texto: 'línea ' + i.linea + ': salta a 0x' + L.hex(i.manejador, 4) + ' (interrumpió 0x' + L.hex(i.pc_interrumpido, 4) + ')' }; });
          pintar(ints);
        });
        var d = (e.dispositivos || []).filter(function (x) { return x.tipo === 'uart'; })[0];
        if (d) A.pedir('serie/' + encodeURIComponent(d.nombre)).then(function (r) {
          var t = (r.datos && r.datos.texto) || '';
          if (t.length > vistoUart) {
            registro.push({ instruccion: e.instrucciones, tipo: 'es', etiqueta: '⇄ E/S', texto: 'UART ' + d.nombre + ' envió «' + t.slice(vistoUart).replace(/\n/g, '↵') + '»' });
            vistoUart = t.length;
            pintar([]);
          }
        });
      }
      L.on('estado', tomar);
    }
  });

  // --- consola de ordenes: las mismas que el depurador de linea de comandos (RF-DEP-044) -------------------------
  L.paneles.registrar({
    id: 'ordenes', titulo: 'Órdenes del depurador', zona: 'abajo', colapsado: true,
    construir: function (cuerpo) {
      var salida = L.el('pre', { id: 'ordenes-salida', class: 'teletipo', role: 'log', 'aria-label': 'Resultado de las órdenes', tabindex: '0' });
      var entrada = L.el('input', { type: 'text', id: 'ordenes-entrada', 'aria-label': 'Orden del depurador (ayuda lista todas)', placeholder: 'paso 3, ruptura bucle si R1 == 5, atras, ayuda…' });
      var historial = [], pos = 0;
      entrada.addEventListener('keydown', function (ev) {
        if (ev.key === 'ArrowUp' && historial.length) { pos = Math.max(0, pos - 1); entrada.value = historial[pos]; ev.preventDefault(); }
        else if (ev.key === 'ArrowDown') { pos = Math.min(historial.length, pos + 1); entrada.value = historial[pos] || ''; ev.preventDefault(); }
        else if (ev.key === 'Enter') {
          var linea = entrada.value;
          if (linea.trim()) { historial.push(linea); pos = historial.length; }
          entrada.value = '';
          A.orden(linea).then(function (r) {
            salida.append(document.createTextNode('(lorenz) ' + linea + '\n' + (r.texto ? r.texto.replace(/\n?$/, '\n') : '')));
            salida.scrollTop = salida.scrollHeight;
          });
        }
      });
      cuerpo.append(salida, entrada);
    }
  });

  A.orden = function (linea) {
    return (sesion.id == null ? L.depurador.abrir() : Promise.resolve(true)).then(function (ok) {
      if (!ok) return { texto: 'No hay sesión: elige máquina y programa.' };
      return L.api('POST', '/api/sesiones/' + sesion.id + '/orden', { linea: linea }).then(function (r) {
        if (!r.ok) return { texto: 'error: ' + (r.datos.error || r.estado) };
        L.depurador.aplicarEstado(r.datos.estado);
        L.depurador.refrescarRupturas();
        return r.datos;
      });
    });
  };
})();
