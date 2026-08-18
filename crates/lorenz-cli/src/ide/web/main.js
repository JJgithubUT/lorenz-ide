// Arranque del IDE: paneles, listas de archivos y seleccion de maquina y programa.
'use strict';
(function () {
  var L = window.L;
  L.proyecto = { archivos: [], maquina: null, programa: null };

  L.cargarProyecto = function () {
    return L.api('GET', '/api/proyecto').then(function (r) {
      if (!r.ok) { L.estado('No se pudo leer el proyecto: ' + (r.datos.error || r.estado), 'error'); return; }
      L.proyecto.archivos = r.datos.archivos || [];
      var maq = L.proyecto.archivos.filter(function (a) { return !a.directorio && /\.toml$/.test(a.ruta) && !/proyecto\.toml$/.test(a.ruta); });
      var prog = L.proyecto.archivos.filter(function (a) { return !a.directorio && /\.(s|asm|lasm|sgt|hex)$/.test(a.ruta); });
      function llenar(sel, lista, actual) {
        sel.textContent = '';
        lista.forEach(function (a) { sel.append(L.el('option', { value: a.ruta, texto: a.ruta })); });
        if (actual && lista.some(function (a) { return a.ruta === actual; })) sel.value = actual;
      }
      var sm = document.getElementById('sel-maquina'), sp = document.getElementById('sel-programa');
      llenar(sm, maq, L.guardado.leer('maquina', null));
      llenar(sp, prog, L.guardado.leer('programa', null));
      L.proyecto.maquina = sm.value || null;
      L.proyecto.programa = sp.value || null;
      L.emit('proyecto', L.proyecto);
    });
  };

  document.addEventListener('DOMContentLoaded', function () {
    var sm = document.getElementById('sel-maquina'), sp = document.getElementById('sel-programa');
    sm.addEventListener('change', function () { L.proyecto.maquina = sm.value; L.guardado.escribir('maquina', sm.value); L.emit('maquina', sm.value); });
    sp.addEventListener('change', function () { L.proyecto.programa = sp.value; L.guardado.escribir('programa', sp.value); L.emit('programa', sp.value); });
    (L.modulos || []).forEach(function (f) { f(); });
    L.paneles.iniciar();
    L.cargarProyecto().then(function () { if (L.abrirDeEnlace) L.abrirDeEnlace(); L.emit('listo'); L.estado('Listo'); document.body.setAttribute('data-listo', '1'); });
  });
})();
