// Buscar y reemplazar en el proyecto, lista de problemas y documentacion de la arquitectura
// (RF-IDE-016, RF-IDE-038).
'use strict';
(function () {
  var L = window.L;

  // --- buscar y reemplazar --------------------------------------------------------------
  L.paneles.registrar({
    id: 'buscar', titulo: 'Buscar y reemplazar', corto: 'Buscar', zona: 'abajo',
    construir: function (cuerpo) {
      var texto = L.el('input', { type: 'search', id: 'buscar-texto', 'aria-label': 'Texto a buscar', placeholder: 'Buscar…' });
      var por = L.el('input', { type: 'text', id: 'buscar-por', 'aria-label': 'Texto de reemplazo', placeholder: 'Reemplazar por…' });
      var may = L.el('input', { type: 'checkbox', id: 'buscar-mayusculas' });
      var res = L.el('ul', { class: 'lista', id: 'buscar-resultados', role: 'listbox', 'aria-label': 'Resultados de la búsqueda' });
      var info = L.el('span', { class: 'suave', id: 'buscar-info', role: 'status' });

      function buscar() {
        if (!texto.value) { info.textContent = 'Escribe qué buscar'; return Promise.resolve(); }
        return L.api('POST', '/api/buscar', { texto: texto.value, mayusculas: may.checked }, 'Buscando').then(function (r) {
          res.textContent = '';
          if (!r.ok) { info.textContent = r.datos.error || 'Error'; info.className = 'error'; return; }
          var lista = r.datos.resultados;
          info.className = 'suave';
          info.textContent = lista.length + (r.datos.truncado ? '+' : '') + ' coincidencia(s)';
          lista.forEach(function (x) {
            var li = L.el('li', { role: 'option', tabindex: '0', 'aria-selected': 'false' }, [
              L.el('span', { class: 'mono', texto: x.ruta + ':' + x.linea + ' ' }), L.el('span', { class: 'suave mono', texto: x.texto.trim() })
            ]);
            var ir = function () { L.emit('abrir-archivo', { ruta: x.ruta, linea: x.linea }); };
            li.addEventListener('click', ir);
            li.addEventListener('keydown', function (ev) { if (ev.key === 'Enter') ir(); });
            res.append(li);
          });
        });
      }
      function reemplazar() {
        if (!texto.value) return;
        if (!window.confirm('¿Reemplazar «' + texto.value + '» por «' + por.value + '» en todo el proyecto?')) return;
        L.guardarTodo().then(function () {
          return L.api('POST', '/api/reemplazar', { texto: texto.value, por: por.value, mayusculas: may.checked }, 'Reemplazando');
        }).then(function (r) {
          if (!r || !r.ok) { info.textContent = (r && r.datos.error) || 'Error'; info.className = 'error'; return; }
          var resumen = r.datos.cambios + ' reemplazo(s) en ' + r.datos.archivos.length + ' archivo(s)';
          L.editor.recargar().then(buscar).then(function () { info.className = 'ok'; info.textContent = resumen; });
        });
      }
      texto.addEventListener('keydown', function (ev) { if (ev.key === 'Enter') buscar(); });
      cuerpo.append(
        L.el('div', { class: 'fila' }, [texto, por,
          L.el('label', {}, [may, ' Distinguir mayúsculas']),
          L.el('button', { type: 'button', id: 'buscar-ir', texto: 'Buscar', onclick: buscar }),
          L.el('button', { type: 'button', id: 'reemplazar-ir', texto: 'Reemplazar todo', onclick: reemplazar }), info]),
        res);
    }
  });
  L.atajos.registrar('buscar', function () {
    L.paneles.mostrar('buscar');
    var t = document.getElementById('buscar-texto');
    var sel = window.getSelection && String(window.getSelection());
    if (t) { if (sel) t.value = sel; t.focus(); t.select(); }
  });

  // --- problemas ---------------------------------------------------------------------------
  L.paneles.registrar({
    id: 'problemas', titulo: 'Problemas', corto: 'Problemas', zona: 'abajo',
    construir: function (cuerpo) {
      var lista = L.el('ul', { class: 'lista', id: 'problemas-lista', role: 'listbox', 'aria-label': 'Problemas del último compilado' });
      cuerpo.append(L.el('p', { class: 'suave', id: 'problemas-vacio', texto: 'Sin problemas en el último compilado.' }), lista);
      L.on('problemas', function (ds) {
        lista.textContent = '';
        if (ds.length) L.paneles.activar('problemas');
        document.getElementById('problemas-vacio').hidden = ds.length > 0;
        ds.forEach(function (d) {
          var li = L.el('li', { role: 'option', tabindex: '0', 'aria-selected': 'false' }, [
            L.el('span', { class: 'error', texto: '✖ Error ' }), L.el('span', { class: 'mono', texto: d.archivo + ':' + d.linea + ' ' }), L.el('span', { texto: d.mensaje + (d.sugerencia ? ' — ' + d.sugerencia : '') })]);
          var ir = function () { L.emit('abrir-archivo', { ruta: d.archivo, linea: d.linea }); };
          li.addEventListener('click', ir);
          li.addEventListener('keydown', function (ev) { if (ev.key === 'Enter') ir(); });
          lista.append(li);
        });
      });
    }
  });

  // --- documentacion de la arquitectura activa --------------------------------------------
  L.paneles.registrar({
    id: 'documentacion', titulo: 'Documentación de la arquitectura', zona: 'der', colapsado: true,
    construir: function (cuerpo) {
      var marco = L.el('div', { class: 'documento', id: 'documento', tabindex: '0', 'aria-label': 'Documentación de la arquitectura activa' });
      cuerpo.append(marco);
      function cargar() {
        if (!L.proyecto.maquina) { marco.textContent = 'Elige una máquina.'; return; }
        L.api('GET', '/api/documentacion?maquina=' + encodeURIComponent(L.proyecto.maquina), undefined, 'Documentando').then(function (r) {
          if (!r.ok) { marco.textContent = r.datos.error || 'No se pudo generar la documentación'; return; }
          // El HTML lo genera el propio programa con el texto escapado; aun asi se aisla en un marco sin scripts.
          var f = L.el('iframe', { sandbox: '', title: 'Documentación de ' + L.proyecto.maquina, class: 'doc-marco', srcdoc: r.datos.html });
          marco.textContent = ''; marco.append(f);
        });
      }
      L.on('maquina', cargar);
      L.on('proyecto', cargar);
    }
  });
})();
