// Version web: aviso de lo que no esta disponible frente al escritorio, compartir, derivar, galeria
// y exportacion (RF-ECO-010 a 013, RF-ECO-030 a 033). En el IDE local solo explica como activarlo.
'use strict';
(function () {
  var L = window.L;
  var modo = { alojado: false, limitaciones: [], cuota: null };

  function absoluta(enlace) { return location.origin + enlace; }
  function recargarProyecto() { return L.cargarProyecto ? L.cargarProyecto() : Promise.resolve(); }

  // --- aviso (RF-ECO-033) -------------------------------------------------------------------------
  (L.modulos = L.modulos || []).push(function () {
    L.api('GET', '/api/modo').then(function (r) {
      if (!r.ok) return;
      modo = r.datos; L.modo = modo;
      document.body.setAttribute('data-alojado', modo.alojado ? '1' : '0');
      L.emit('modo', modo);
      if (!modo.alojado) return;
      var banda = document.getElementById('banda-web');
      var lista = L.el('ul', { id: 'web-limitaciones', class: 'lista' }, modo.limitaciones.map(function (l) {
        return L.el('li', {}, [L.el('strong', { texto: l.funcion + ': ' }), L.el('span', { texto: l.motivo })]);
      }));
      var cuota = modo.cuota ? 'Espacio: hasta ' + modo.cuota.archivos + ' archivos y ' + Math.round(modo.cuota.bytes / 1048576) + ' MiB.' : '';
      banda.append(
        L.el('strong', { texto: 'Versión web. ' }),
        L.el('span', { texto: 'Tu trabajo se guarda en este navegador y en el servidor. ' + cuota }),
        L.el('details', { id: 'web-diferencias' }, [L.el('summary', { texto: 'Qué no hay frente al escritorio' }), lista])
      );
      banda.hidden = false;
    });
  });

  // --- abrir un enlace compartido: /c/<id> redirige a /?abrir=...&programa=... -----------------------
  L.abrirDeEnlace = function () {
    var q = new URLSearchParams(location.search), m = q.get('abrir'), p = q.get('programa');
    if (!m) return;
    var sm = document.getElementById('sel-maquina'), sp = document.getElementById('sel-programa');
    function elegir(sel, valor, clave) {
      if (!valor) return false;
      var hay = Array.prototype.some.call(sel.options, function (o) { return o.value === valor; });
      if (hay) { sel.value = valor; L.proyecto[clave] = valor; L.guardado.escribir(clave, valor); }
      return hay;
    }
    // La carpeta compartida se creo al visitar /c/<id>, antes de cargar la pagina: ya esta en la lista.
    if (elegir(sm, m, 'maquina')) L.emit('maquina', m);
    if (elegir(sp, p, 'programa')) { L.emit('programa', p); L.editor && L.editor.abrir(p); }
    history.replaceState(null, '', location.pathname);
    L.estado('Abierto lo compartido contigo: se puede ejecutar; para editarlo, deriva una copia.', 'ok');
  };

  // --- panel: compartir, derivar, galeria, exportar -----------------------------------------------------
  L.paneles.registrar({
    id: 'compartir', titulo: 'Compartir y galería', zona: 'der', colapsado: true,
    construir: function (cuerpo) {
      var aviso = L.el('p', { id: 'compartir-aviso', class: 'suave', role: 'status', 'aria-live': 'polite' });
      var enlace = L.el('input', { type: 'text', id: 'compartir-enlace', readonly: true, size: '28', 'aria-label': 'Enlace para compartir' });
      var titulo = L.el('input', { type: 'text', id: 'galeria-titulo', size: '24', 'aria-label': 'Título para la galería', maxlength: '80' });
      var desc = L.el('input', { type: 'text', id: 'galeria-descripcion', size: '24', 'aria-label': 'Descripción', maxlength: '500' });
      var etiquetas = L.el('input', { type: 'text', id: 'galeria-etiquetas', size: '24', 'aria-label': 'Etiquetas separadas por comas' });
      var busqueda = L.el('input', { type: 'search', id: 'galeria-q', size: '20', 'aria-label': 'Buscar en la galería', placeholder: 'buscar…' });
      var resultados = L.el('ul', { id: 'galeria-lista', class: 'lista', 'aria-label': 'Máquinas publicadas' });
      var copia = L.el('input', { type: 'text', id: 'derivar-nombre', size: '16', 'aria-label': 'Nombre de la copia', placeholder: 'mi-copia' });
      var idActual = null;

      function boton(id, texto, f) { var b = L.el('button', { type: 'button', id: id, texto: texto }); b.addEventListener('click', f); return b; }
      function ids() {
        var m = /^compartido\/([0-9a-f]{20})\//.exec(document.getElementById('sel-maquina').value || '');
        return m ? m[1] : null;
      }

      var seccionLocal = L.el('p', { id: 'compartir-solo-web', class: 'suave', texto: 'Compartir, derivar y la galería son de la versión web: arranca el servicio con «lorenz servir».' });
      var seccionWeb = L.el('div', { id: 'compartir-web' });
      seccionWeb.append(
        L.el('h3', { texto: 'Compartir' }),
        L.el('div', { class: 'fila' }, [
          boton('compartir-hacer', 'Compartir la máquina y el programa elegidos', function () {
            var m = document.getElementById('sel-maquina').value, p = document.getElementById('sel-programa').value;
            if (!m || !p) { aviso.textContent = 'Elige una máquina y un programa.'; return; }
            L.guardarTodo().then(function () {
              return L.api('POST', '/api/compartir', { maquina: m, programa: p });
            }).then(function (r) {
              if (!r.ok) { aviso.textContent = 'No se pudo compartir: ' + (r.datos.error || r.estado); return; }
              idActual = r.datos.id; enlace.value = absoluta(r.datos.enlace);
              aviso.textContent = 'Listo: quien tenga el enlace puede ejecutarlo sin instalar nada (' + r.datos.archivos + ' archivos). El enlace depende del contenido: si lo cambias, será otro.';
            });
          }),
          enlace,
          boton('compartir-copiar', 'Copiar', function () {
            if (!enlace.value) return;
            enlace.select();
            if (navigator.clipboard) navigator.clipboard.writeText(enlace.value).then(function () { aviso.textContent = 'Enlace copiado.'; }, function () {});
          })
        ]),
        L.el('h3', { texto: 'Publicar en la galería' }),
        L.el('label', { class: 'campo-arq' }, ['Título ', titulo]),
        L.el('label', { class: 'campo-arq' }, ['Descripción ', desc]),
        L.el('label', { class: 'campo-arq' }, ['Etiquetas ', etiquetas]),
        boton('galeria-publicar', 'Publicar lo que acabas de compartir', function () {
          if (!idActual) { aviso.textContent = 'Comparte primero para obtener el enlace.'; return; }
          var et = etiquetas.value.split(',').map(function (x) { return x.trim(); }).filter(Boolean);
          L.api('POST', '/api/galeria', { id: idActual, titulo: titulo.value, descripcion: desc.value, etiquetas: et }).then(function (r) {
            aviso.textContent = r.ok ? 'Publicado en la galería.' : 'No se pudo publicar: ' + (r.datos.error || r.estado);
            if (r.ok) buscar();
          });
        }),
        L.el('h3', { texto: 'Derivar una copia propia' }),
        L.el('p', { class: 'suave', texto: 'Lo compartido por otros se ejecuta, pero no se edita. Una copia sí.' }),
        L.el('label', { class: 'campo-arq' }, ['Nombre ', copia]),
        boton('derivar-hacer', 'Derivar copia de lo que tienes abierto', function () {
          var id = ids();
          if (!id) { aviso.textContent = 'Abre primero algo compartido (un enlace /c/…).'; return; }
          derivar(id, copia.value || 'mi-copia');
        }),
        L.el('h3', { texto: 'Galería' }),
        L.el('div', { class: 'fila' }, [busqueda, boton('galeria-buscar', 'Buscar', function () { buscar(); })]),
        resultados,
        L.el('h3', { texto: 'Llevarlo al escritorio' }),
        L.el('a', { id: 'exportar-lpkg', href: '/api/exportar', download: 'proyecto.lpkg', texto: 'Exportar el proyecto (.lpkg)' })
      );

      function derivar(id, nombre) {
        L.api('POST', '/api/derivar', { id: id, nombre: nombre }).then(function (r) {
          if (!r.ok) { aviso.textContent = 'No se pudo derivar: ' + (r.datos.error || r.estado); return; }
          aviso.textContent = 'Copia creada en «' + r.datos.carpeta + '». Ya puedes editarla.';
          recargarProyecto().then(function () {
            var sm = document.getElementById('sel-maquina'), sp = document.getElementById('sel-programa');
            sm.value = r.datos.maquina; sp.value = r.datos.programa;
            L.proyecto.maquina = sm.value; L.proyecto.programa = sp.value;
            L.emit('maquina', sm.value); L.emit('programa', sp.value);
            if (L.editor) L.editor.abrir(r.datos.programa);
          });
        });
      }

      function buscar() {
        L.api('GET', '/api/galeria?q=' + encodeURIComponent(busqueda.value)).then(function (r) {
          resultados.textContent = '';
          if (!r.ok) return;
          var lista = r.datos.resultados;
          if (!lista.length) { resultados.append(L.el('li', { class: 'suave', id: 'galeria-vacia', texto: 'Nada publicado con eso.' })); return; }
          lista.forEach(function (e) {
            var acciones = [
              L.el('a', { href: e.enlace, class: 'abrir-galeria', texto: 'Abrir' }),
              ' ', boton('derivar-' + e.id, 'Derivar', function () { derivar(e.id, (e.maquina || 'copia').toLowerCase().replace(/[^a-z0-9_-]+/g, '-').replace(/^[-_]+/, '').slice(0, 30) || 'copia'); })
            ];
            if (e.mia) acciones.push(' ', boton('retirar-' + e.id, 'Retirar', function () {
              L.api('DELETE', '/api/galeria/' + e.id).then(function () { buscar(); });
            }));
            resultados.append(L.el('li', { class: 'entrada-galeria', 'data-id': e.id }, [
              L.el('strong', { texto: e.titulo }), L.el('span', { class: 'suave', texto: e.maquina ? ' · ' + e.maquina : '' }),
              L.el('div', { texto: e.descripcion }),
              L.el('div', { class: 'suave', texto: e.etiquetas.map(function (t) { return '#' + t; }).join(' ') }),
              L.el('div', {}, acciones)
            ]));
          });
        });
      }
      busqueda.addEventListener('keydown', function (ev) { if (ev.key === 'Enter') buscar(); });

      cuerpo.append(aviso, seccionLocal, seccionWeb);
      function ajustar() { seccionLocal.hidden = modo.alojado; seccionWeb.hidden = !modo.alojado; if (modo.alojado) buscar(); }
      seccionLocal.hidden = false; seccionWeb.hidden = true;
      L.on('modo', ajustar);
    }
  });
})();
