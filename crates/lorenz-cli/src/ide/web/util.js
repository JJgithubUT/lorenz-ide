// Utilidades comunes del IDE. Todo cuelga del objeto global `L`.
'use strict';
var L = window.L = window.L || {};

L.el = function (tag, atributos, hijos) {
  var e = document.createElement(tag);
  Object.keys(atributos || {}).forEach(function (k) {
    var v = atributos[k];
    if (k === 'texto') e.textContent = v;
    else if (k.slice(0, 2) === 'on') e.addEventListener(k.slice(2), v);
    else if (v === true) e.setAttribute(k, '');
    else if (v !== false && v != null) e.setAttribute(k, v);
  });
  (hijos || []).forEach(function (h) { e.append(h); });
  return e;
};

// Almacenamiento ligero por navegador; nunca debe romper nada si falta.
L.guardado = {
  leer: function (clave, defecto) {
    try { var v = localStorage.getItem('lorenz.' + clave); return v == null ? defecto : JSON.parse(v); } catch (e) { return defecto; }
  },
  escribir: function (clave, valor) {
    try { localStorage.setItem('lorenz.' + clave, JSON.stringify(valor)); } catch (e) { /* sin almacenamiento */ }
  },
  borrar: function (clave) { try { localStorage.removeItem('lorenz.' + clave); } catch (e) { /* nada */ } }
};

// Eventos entre modulos.
L.oyentes = {};
L.on = function (n, f) { (L.oyentes[n] = L.oyentes[n] || []).push(f); };
L.emit = function (n, d) { (L.oyentes[n] || []).forEach(function (f) { try { f(d); } catch (e) { console.error(e); } }); };

L.estado = function (texto, clase) {
  var e = document.getElementById('estado-texto');
  if (!e) return;
  e.textContent = texto;
  e.className = clase || '';
};

// Operaciones en curso: si tardan mas de un segundo se muestra el progreso (RF-IDE-035).
L.enCurso = {};
var siguienteOperacion = 1;
L.progreso = function (etiqueta) {
  var id = siguienteOperacion++;
  var inicio = Date.now();
  var temporizador = setInterval(function () {
    var caja = document.getElementById('progreso');
    if (!caja) return;
    var seg = Math.floor((Date.now() - inicio) / 1000);
    if (seg >= 1) {
      caja.hidden = false;
      document.getElementById('progreso-texto').textContent = etiqueta + ' (' + seg + ' s)';
    }
  }, 250);
  L.enCurso[id] = temporizador;
  return function terminar() {
    clearInterval(L.enCurso[id]);
    delete L.enCurso[id];
    if (!Object.keys(L.enCurso).length) {
      var caja = document.getElementById('progreso');
      if (caja) caja.hidden = true;
    }
  };
};

// Llamada a la API. Devuelve {ok, estado, datos}; nunca lanza.
L.api = function (metodo, ruta, cuerpo, etiqueta) {
  var fin = L.progreso(etiqueta || (metodo + ' ' + ruta));
  var opciones = { method: metodo, headers: {} };
  if (cuerpo !== undefined) { opciones.body = JSON.stringify(cuerpo); opciones.headers['Content-Type'] = 'application/json'; }
  return fetch(ruta, opciones).then(function (r) {
    return r.text().then(function (t) {
      var datos = null;
      try { datos = t ? JSON.parse(t) : null; } catch (e) { datos = { error: t }; }
      return { ok: r.ok, estado: r.status, datos: datos || {} };
    });
  }).catch(function (e) {
    return { ok: false, estado: 0, datos: { error: 'sin conexion con el servidor: ' + e.message } };
  }).then(function (r) { fin(); return r; });
};

L.hex = function (n, ancho) { var s = Number(n).toString(16); while (s.length < (ancho || 4)) s = '0' + s; return s; };
L.escapar = function (t) { return String(t).replace(/[&<>"]/g, function (c) { return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]; }); };
