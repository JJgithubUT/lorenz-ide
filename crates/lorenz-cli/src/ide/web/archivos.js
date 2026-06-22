// Panel de archivos del proyecto.
'use strict';
(function () {
  var L = window.L;
  var lista;
  function pintar() {
    if (!lista) return;
    lista.textContent = '';
    L.proyecto.archivos.forEach(function (a) {
      var profundidad = a.ruta.split('/').length - 1;
      var nombre = a.ruta.split('/').pop();
      var li = L.el('li', {
        role: 'option', tabindex: '0', 'data-ruta': a.ruta, 'aria-selected': 'false',
        style: 'padding-left:' + (0.4 + profundidad * 0.9) + 'rem',
        texto: (a.directorio ? '▸ ' : '') + nombre
      });
      if (a.directorio) li.className = 'dir';
      else {
        var abrir = function () { L.emit('abrir-archivo', { ruta: a.ruta }); };
        li.addEventListener('click', abrir);
        li.addEventListener('keydown', function (ev) { if (ev.key === 'Enter' || ev.key === ' ') { ev.preventDefault(); abrir(); } });
      }
      lista.append(li);
    });
  }
  L.paneles.registrar({
    id: 'archivos', titulo: 'Archivos', zona: 'izq',
    construir: function (cuerpo) {
      lista = L.el('ul', { class: 'lista', role: 'listbox', 'aria-label': 'Archivos del proyecto' });
      var nuevo = L.el('button', {
        type: 'button', texto: 'Nuevo archivo…', onclick: function () {
          var ruta = window.prompt('Ruta del archivo nuevo (relativa al proyecto):', 'nuevo.s');
          if (!ruta) return;
          L.api('PUT', '/api/archivo', { ruta: ruta, texto: '' }).then(function (r) {
            if (!r.ok) { L.estado(r.datos.error || 'No se pudo crear', 'error'); return; }
            L.cargarProyecto().then(function () { L.emit('abrir-archivo', { ruta: ruta }); });
          });
        }
      });
      cuerpo.append(nuevo, lista);
    }
  });
  L.on('proyecto', pintar);
})();
