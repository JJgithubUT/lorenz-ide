// Paneles acoplables, separables y reordenables, con la disposicion recordada (RF-IDE-017, RF-IDE-018).
'use strict';
(function () {
  var L = window.L;
  var ZONAS = ['izq', 'centro', 'der', 'abajo'];
  var NOMBRES_ZONA = { izq: 'izquierda', centro: 'centro', der: 'derecha', abajo: 'abajo' };
  var registro = {};     // id -> definicion
  var elementos = {};    // id -> elemento .panel
  var disposicion = {};  // id -> {zona, orden, flotante, x, y, colapsado}

  function guardar() { L.guardado.escribir('disposicion', disposicion); setTimeout(function () { if (L.emit) L.emit('disposicion'); }, 0); }

  function zona(z) { return document.getElementById('zona-' + z); }

  function ordenar(z) {
    var ids = Object.keys(disposicion).filter(function (i) { return disposicion[i].zona === z && !disposicion[i].flotante && elementos[i]; });
    ids.sort(function (a, b) { return disposicion[a].orden - disposicion[b].orden; });
    ids.forEach(function (id, n) { disposicion[id].orden = n; zona(z).appendChild(elementos[id]); });
    var tira = document.getElementById('pestanas-abajo'); if (z === 'abajo' && tira) zona(z).insertBefore(tira, zona(z).firstChild);
  }

  function pintar() {
    ZONAS.forEach(ordenar);
    Object.keys(disposicion).forEach(function (id) {
      var el = elementos[id], d = disposicion[id];
      if (!el) return;
      el.classList.toggle('colapsado', !!d.colapsado);
      el.classList.toggle('flotante', !!d.flotante);
      if (d.flotante) {
        document.getElementById('flotantes').appendChild(el);
        el.style.left = Math.max(0, d.x || 80) + 'px';
        el.style.top = Math.max(0, d.y || 80) + 'px';
      } else {
        el.style.left = el.style.top = '';
      }
      var sep = el.querySelector('[data-accion="separar"]');
      if (sep) { sep.textContent = d.flotante ? 'Acoplar' : 'Separar'; }
      var h = el.querySelector('[data-accion="colapsar"]');
      if (h) h.setAttribute('aria-expanded', d.colapsado ? 'false' : 'true');
    });
    ZONAS.forEach(function (z) { zona(z).classList.toggle('vacia', !zona(z).querySelector('.panel')); });
    pintarPestanas();
  }

  // La zona inferior reune sus paneles en pestanas: solo se ve uno a la vez y todos se alcanzan de
  // un vistazo. Pasa por la cabecera de cada panel: se queda solo su menu «⋯».
  var activa = null;
  function enPestanas() {
    return Object.keys(disposicion).filter(function (i) { return disposicion[i].zona === 'abajo' && !disposicion[i].flotante && elementos[i]; })
      .sort(function (a, b) { return disposicion[a].orden - disposicion[b].orden; });
  }
  function activar(id, enfocar) {
    activa = id; L.guardado.escribir('pestana-abajo', id); pintarPestanas();
    var t = document.getElementById('pestana-' + id);
    if (enfocar && t) t.focus();
    if (L.emit) L.emit('pestana', id);
  }
  function pintarPestanas() {
    var z = zona('abajo'), ids = enPestanas();
    var tira = document.getElementById('pestanas-abajo');
    if (!tira) {
      tira = L.el('div', { id: 'pestanas-abajo', class: 'tira-pestanas', role: 'tablist', 'aria-label': 'Paneles inferiores' });
      tira.addEventListener('keydown', function (ev) {
        var i = enPestanas().indexOf(activa), n = enPestanas().length, d = ev.key === 'ArrowRight' ? 1 : ev.key === 'ArrowLeft' ? -1 : 0;
        if (ev.key === 'Home') { activar(enPestanas()[0], true); ev.preventDefault(); }
        else if (ev.key === 'End') { activar(enPestanas()[n - 1], true); ev.preventDefault(); }
        else if (d) { activar(enPestanas()[(i + d + n) % n], true); ev.preventDefault(); }
      });
      z.insertBefore(tira, z.firstChild);
    }
    if (!ids.length) { tira.hidden = true; z.classList.remove('con-pestanas'); return; }
    tira.hidden = false; z.classList.add('con-pestanas');
    if (ids.indexOf(activa) < 0) { var g = L.guardado.leer('pestana-abajo', null); activa = ids.indexOf(g) >= 0 ? g : (ids.indexOf('consola') >= 0 ? 'consola' : ids[0]); }
    var foco = document.activeElement && document.activeElement.parentNode === tira ? document.activeElement.id : null;
    tira.textContent = '';
    ids.forEach(function (id) {
      var sel = id === activa;
      tira.append(L.el('button', {
        type: 'button', role: 'tab', id: 'pestana-' + id, class: 'pestana-abajo' + (sel ? ' activa' : ''), texto: registro[id].corto || registro[id].titulo,
        'aria-selected': sel ? 'true' : 'false', 'aria-controls': 'panel-' + id, tabindex: sel ? '0' : '-1', title: registro[id].titulo,
        onclick: function () { activar(id); }
      }));
      elementos[id].classList.toggle('en-pestana-oculta', !sel);
      elementos[id].setAttribute('role', 'tabpanel');
    });
    if (foco && document.getElementById(foco)) document.getElementById(foco).focus();
    // El panel a la vista no puede estar plegado: sus dibujos solo se actualizan desplegado.
    var da = disposicion[activa];
    if (da && da.colapsado) { da.colapsado = false; elementos[activa].classList.remove('colapsado'); guardar(); }
    // Los paneles que salen de la zona vuelven a ser paneles normales.
    Object.keys(elementos).forEach(function (id) { if (ids.indexOf(id) < 0) { elementos[id].classList.remove('en-pestana-oculta'); elementos[id].removeAttribute('role'); } });
  }

  function mover(id, hacia) {
    var d = disposicion[id];
    if (hacia === 'sube' || hacia === 'baja') {
      var vecinos = Object.keys(disposicion).filter(function (i) { return disposicion[i].zona === d.zona && !disposicion[i].flotante && elementos[i]; })
        .sort(function (a, b) { return disposicion[a].orden - disposicion[b].orden; });
      var pos = vecinos.indexOf(id), otro = vecinos[pos + (hacia === 'sube' ? -1 : 1)];
      if (otro) { var t = disposicion[otro].orden; disposicion[otro].orden = d.orden; d.orden = t; }
    } else {
      d.zona = hacia; d.flotante = false; d.orden = 1e6;
    }
    guardar(); pintar();
    L.estado('Panel «' + registro[id].titulo + '» en ' + (d.flotante ? 'flotante' : NOMBRES_ZONA[d.zona]));
  }

  function construir(def) {
    var cuerpo = L.el('div', { class: 'cuerpo', id: 'cuerpo-' + def.id });
    var el = L.el('section', { class: 'panel', id: 'panel-' + def.id, 'aria-labelledby': 'titulo-' + def.id, 'data-panel': def.id });
    var cab = L.el('header', { draggable: 'true' });
    var h2Titulo;
    cab.append(L.el('h2', { id: 'titulo-' + def.id, texto: def.titulo }));
    var menu = L.el('div', { class: 'menu', role: 'group', 'aria-label': 'Opciones del panel ' + def.titulo });
    var abrirMenu = L.el('details', { class: 'menu-panel' }, [
      L.el('summary', { texto: '⋯', title: 'Mover, separar o acoplar este panel', 'aria-label': 'Opciones del panel ' + def.titulo }), menu
    ]);
    function accionar(que) {
      var d = disposicion[def.id];
      if (que === 'colapsar') { d.colapsado = !d.colapsado; guardar(); pintar(); }
      else if (que === 'separar') { d.flotante = !d.flotante; if (d.flotante) { d.x = 100 + 30 * Object.keys(disposicion).length % 300; d.y = 100; } guardar(); pintar(); }
      else mover(def.id, que);
    }
    // Plegar vive en la cabecera; lo demas (mover, separar) en el menu «⋯», para que la cabecera
    // sea solo el nombre del panel.
    cab.append(L.el('button', {
      type: 'button', class: 'plegar', 'data-accion': 'colapsar', texto: '▾', 'aria-label': 'Plegar o desplegar el panel ' + def.titulo, title: 'Plegar o desplegar',
      onclick: function (ev) { ev.stopPropagation(); accionar('colapsar'); }
    }));
    cab.append(cab.querySelector('h2'));
    cab.append(abrirMenu);
    [['sube', 'Subir', 'Subir'], ['baja', 'Bajar', 'Bajar'],
     ['izq', 'A la izquierda', 'Mover a la izquierda'], ['centro', 'Al centro', 'Mover al centro'], ['der', 'A la derecha', 'Mover a la derecha'], ['abajo', 'Abajo', 'Mover abajo'],
     ['separar', 'Separar', 'Separar o acoplar']].forEach(function (b) {
      menu.append(L.el('button', {
        type: 'button', 'data-accion': b[0], texto: b[1], 'aria-label': b[2] + ' el panel ' + def.titulo, title: b[2],
        onclick: function (ev) { ev.stopPropagation(); abrirMenu.open = false; accionar(b[0]); }
      }));
    });
    h2Titulo = cab.querySelector('h2');
    cab.addEventListener('dblclick', function (ev) { if (ev.target === cab || ev.target === h2Titulo) accionar('colapsar'); });
    cab.addEventListener('dragstart', function (ev) { ev.dataTransfer.setData('text/plain', def.id); ev.dataTransfer.effectAllowed = 'move'; document.body.classList.add('arrastrando'); });
    cab.addEventListener('dragend', function () { document.body.classList.remove('arrastrando'); });
    // Los paneles flotantes se arrastran con el raton por su cabecera.
    cab.addEventListener('mousedown', function (ev) {
      var d = disposicion[def.id];
      if (!d.flotante || ev.target.tagName === 'BUTTON' || ev.target.closest('.menu-panel')) return;
      var dx = ev.clientX - (d.x || 0), dy = ev.clientY - (d.y || 0);
      function mueve(e) { d.x = e.clientX - dx; d.y = e.clientY - dy; el.style.left = d.x + 'px'; el.style.top = d.y + 'px'; }
      function suelta() { document.removeEventListener('mousemove', mueve); document.removeEventListener('mouseup', suelta); guardar(); }
      document.addEventListener('mousemove', mueve); document.addEventListener('mouseup', suelta);
    });
    el.append(cab, cuerpo);
    elementos[def.id] = el;
    def.construir(cuerpo, el);
  }

  L.paneles = {
    zonas: ZONAS,
    registrar: function (def) { registro[def.id] = def; },
    ids: function () { return Object.keys(registro); },
    disposicion: function () { return JSON.parse(JSON.stringify(disposicion)); },
    mover: mover,
    activar: activar,
    activa: function () { return activa; },
    mostrar: function (id) {
      var el = elementos[id];
      if (!el) return;
      disposicion[id].colapsado = false; guardar(); pintar();
      if (disposicion[id].zona === 'abajo' && !disposicion[id].flotante) activar(id);
      var primero = el.querySelector('textarea, input, button:not([data-accion]), [tabindex]');
      (primero || el.querySelector('h2')).focus && (primero || el).focus();
    },
    restaurar: function () { L.guardado.borrar('disposicion'); disposicion = {}; L.paneles.iniciar(); },
    iniciar: function () {
      var guardada = L.guardado.leer('disposicion', {});
      var contador = {};
      Object.keys(registro).forEach(function (id) {
        var def = registro[id];
        var d = guardada[id];
        if (!d || ZONAS.indexOf(d.zona) < 0) { contador[def.zona] = (contador[def.zona] || 0) + 1; d = { zona: def.zona, orden: contador[def.zona], flotante: false, colapsado: !!def.colapsado }; }
        disposicion[id] = d;
        if (!elementos[id]) construir(def);
      });
      pintar(); guardar();
    }
  };

  // El menu «⋯» se cierra al pulsar fuera o con Escape.
  document.addEventListener('click', function (ev) {
    document.querySelectorAll('.menu-panel[open]').forEach(function (m) { if (!m.contains(ev.target)) m.open = false; });
  });
  document.addEventListener('keydown', function (ev) {
    if (ev.key !== 'Escape') return;
    document.querySelectorAll('.menu-panel[open]').forEach(function (m) { m.open = false; var r = m.querySelector('summary'); if (r) r.focus(); });
  });

  // Soltar un panel sobre una zona lo acopla alli.
  document.addEventListener('DOMContentLoaded', function () {
    ZONAS.forEach(function (z) {
      var e = zona(z);
      e.addEventListener('dragover', function (ev) { ev.preventDefault(); e.classList.add('sobre'); });
      e.addEventListener('dragleave', function () { e.classList.remove('sobre'); });
      e.addEventListener('drop', function (ev) {
        ev.preventDefault(); e.classList.remove('sobre'); document.body.classList.remove('arrastrando');
        var id = ev.dataTransfer.getData('text/plain');
        if (registro[id]) mover(id, z);
      });
    });
  });

  // Teclado: pasar de un panel al siguiente (RF-IDE-033).
  function enfocarPanel(delta) {
    var visibles = Array.prototype.slice.call(document.querySelectorAll('.panel')).filter(function (p) { return p.offsetParent !== null || p.classList.contains('flotante'); });
    if (!visibles.length) return;
    var actual = document.activeElement && document.activeElement.closest('.panel');
    var i = visibles.indexOf(actual);
    var sig = visibles[(i + delta + visibles.length) % visibles.length];
    var foco = sig.querySelector('textarea, input, select, button:not([data-accion]), [tabindex="0"]') || sig.querySelector('h2');
    if (foco) { if (!foco.hasAttribute('tabindex') && foco.tagName === 'H2') foco.setAttribute('tabindex', '-1'); foco.focus(); }
  }
  L.atajos.registrar('siguiente-panel', function () { enfocarPanel(1); });
  L.atajos.registrar('panel-anterior', function () { enfocarPanel(-1); });
})();
