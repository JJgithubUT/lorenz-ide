// Editor: pestanas, resaltado, autocompletado, codificacion bajo el cursor, errores en linea,
// ir a la definicion, guardado automatico y recuperacion (RF-IDE-010 a RF-IDE-016, 036, 037).
'use strict';
(function () {
  var L = window.L;
  var abiertos = {};       // ruta -> {texto, original, sucio}
  var orden = [];          // rutas abiertas, en orden de pestanas
  var actual = null;
  var diagnosticos = {};   // ruta -> {linea: mensaje}
  var simbolos = [];       // del ultimo ensamblado correcto
  var ui = {};
  var autoPendiente = null;

  // La linea que la maquina esta a punto de ejecutar, si es de este archivo.
  var ejecucion = null;
  function lineaEnEjecucion() {
    if (!ejecucion || !actual || !ejecucion.archivo) return 0;
    var a = String(ejecucion.archivo).replace(/\\/g, '/');
    return (a === actual || a.slice(-(actual.length + 1)) === '/' + actual) ? ejecucion.linea : 0;
  }

  function lenguaje() { return L.lenguajeDe(actual || ''); }

  // --- pintado -------------------------------------------------------------------------
  function pintarTextoResaltado() {
    if (!actual) { ui.resalte.innerHTML = ''; return; }
    var lineas = L.resaltar(abiertos[actual].texto, lenguaje());
    var errs = diagnosticos[actual] || {};
    var ahora = lineaEnEjecucion();
    ui.resalte.innerHTML = lineas.map(function (h, i) {
      var e = errs[i + 1];
      return '<span class="linea' + (e ? ' con-error' : '') + (ahora === i + 1 ? ' ejecutando' : '') + '" data-n="' + (i + 1) + '"' + (e ? ' title="' + L.escapar(e) + '"' : '') + '>' + (h || ' ') + (e ? '<span class="marca-error" aria-hidden="true"> ✖ ' + L.escapar(e) + '</span>' : '') + '</span>';
    }).join('\n');
    ui.numeros.innerHTML = lineas.map(function (_, i) {
      var e = errs[i + 1];
      if (ahora === i + 1 && !e) return '<div class="ejecutando" aria-label="línea ' + (i + 1) + ', la siguiente en ejecutarse">▶' + (i + 1) + '</div>';
      return '<div' + (e ? ' class="con-error" aria-label="línea ' + (i + 1) + ' con error"' : '') + '>' + (e ? '✖' : '') + (i + 1) + '</div>';
    }).join('');
  }

  function pintarPestanas() {
    ui.pestanas.textContent = '';
    orden.forEach(function (r) {
      var d = abiertos[r];
      var b = L.el('button', {
        type: 'button', role: 'tab', 'aria-selected': r === actual ? 'true' : 'false', class: 'pestana' + (r === actual ? ' activa' : ''),
        texto: r.split('/').pop() + (d.sucio ? ' •' : ''), title: r + (d.sucio ? ' (sin guardar)' : ''),
        onclick: function () { activar(r); }
      });
      var x = L.el('button', { type: 'button', class: 'cerrar', texto: '×', 'aria-label': 'Cerrar ' + r, onclick: function (ev) { ev.stopPropagation(); cerrar(r); } });
      ui.pestanas.append(L.el('span', { class: 'par-pestana' }, [b, x]));
    });
  }

  function sincronizarDesplazamiento() {
    ui.capa.style.transform = 'translate(' + (-ui.area.scrollLeft) + 'px,' + (-ui.area.scrollTop) + 'px)';
    ui.numeros.style.transform = 'translateY(' + (-ui.area.scrollTop) + 'px)';
  }

  function activar(ruta) {
    actual = ruta;
    var d = abiertos[ruta];
    ui.area.value = d ? d.texto : '';
    ui.area.disabled = !d;
    ui.area.setAttribute('aria-label', d ? 'Editor de ' + ruta : 'Editor');
    pintarPestanas();
    pintarTextoResaltado();
    sincronizarDesplazamiento();
    L.guardado.escribir('actual', ruta);
    L.emit('editor-activo', ruta);
    cerrarSugerencias();
    mostrarCodificacion();
  }

  function cerrar(ruta) {
    var d = abiertos[ruta];
    if (d && d.sucio && !window.confirm(ruta + ' tiene cambios sin guardar. ¿Descartarlos?')) return;
    borrarBorrador(ruta);
    delete abiertos[ruta];
    orden = orden.filter(function (r) { return r !== ruta; });
    guardarSesion();
    if (actual === ruta) activar(orden[orden.length - 1] || null); else pintarPestanas();
  }

  function guardarSesion() { L.guardado.escribir('abiertos', orden); }

  // --- borradores y guardado (RF-IDE-036, RF-IDE-037) ----------------------------------
  function borradores() { return L.guardado.leer('borradores', {}); }
  function guardarBorrador(ruta) {
    var b = borradores(); b[ruta] = abiertos[ruta].texto; L.guardado.escribir('borradores', b);
  }
  function borrarBorrador(ruta) { var b = borradores(); delete b[ruta]; L.guardado.escribir('borradores', b); }

  function guardarArchivo(ruta) {
    var d = abiertos[ruta];
    if (!d) return Promise.resolve(true);
    if (!d.sucio) return Promise.resolve(true);
    return L.api('PUT', '/api/archivo', { ruta: ruta, texto: d.texto }, 'Guardando ' + ruta).then(function (r) {
      if (!r.ok) { L.estado('No se pudo guardar ' + ruta + ': ' + (r.datos.error || r.estado), 'error'); return false; }
      d.original = d.texto; d.sucio = false; borrarBorrador(ruta); pintarPestanas();
      L.estado('Guardado ' + ruta, 'ok');
      return true;
    });
  }
  L.guardarTodo = function () {
    return Promise.all(orden.map(guardarArchivo)).then(function (r) { return r.every(Boolean); });
  };

  // --- abrir -----------------------------------------------------------------------------
  function abrir(ruta, linea) {
    var hecho = function () {
      activar(ruta);
      if (linea) irALinea(linea);
    };
    if (abiertos[ruta]) { hecho(); return Promise.resolve(); }
    return L.api('GET', '/api/archivo?ruta=' + encodeURIComponent(ruta)).then(function (r) {
      if (!r.ok) { L.estado(ruta + ': ' + (r.datos.error || r.estado), 'error'); return; }
      var texto = r.datos.texto, sucio = false;
      var b = borradores()[ruta];
      if (b != null && b !== texto) { texto = b; sucio = true; avisoRecuperado(ruta, r.datos.texto); }
      abiertos[ruta] = { texto: texto, original: r.datos.texto, sucio: sucio };
      orden.push(ruta); guardarSesion();
      hecho();
    });
  }

  function avisoRecuperado(ruta, deDisco) {
    var barra = L.el('div', { class: 'aviso-recuperado', role: 'alert' }, [
      L.el('span', { texto: 'Se recuperó trabajo no guardado de ' + ruta + '. ' }),
      L.el('button', { type: 'button', texto: 'Conservar', onclick: function () { barra.remove(); } }),
      L.el('button', { type: 'button', texto: 'Descartar', onclick: function () {
        var d = abiertos[ruta]; d.texto = deDisco; d.sucio = false; borrarBorrador(ruta);
        if (actual === ruta) { ui.area.value = deDisco; pintarTextoResaltado(); } pintarPestanas(); barra.remove();
      } })
    ]);
    ui.avisos.append(barra);
  }

  function irALinea(n) {
    var lineas = ui.area.value.split('\n'), pos = 0;
    for (var i = 0; i < n - 1 && i < lineas.length; i++) pos += lineas[i].length + 1;
    ui.area.focus();
    ui.area.setSelectionRange(pos, pos + (lineas[n - 1] || '').length);
    var alto = parseFloat(getComputedStyle(ui.area).lineHeight) || 20;
    ui.area.scrollTop = Math.max(0, (n - 3) * alto);
    sincronizarDesplazamiento();
  }

  // --- edicion --------------------------------------------------------------------------
  function alEditar() {
    var d = abiertos[actual];
    if (!d) return;
    d.texto = ui.area.value;
    d.sucio = d.texto !== d.original;
    pintarTextoResaltado();
    sincronizarDesplazamiento();
    pintarPestanas();
    L.emit('texto-editado', { ruta: actual, texto: d.texto, origen: 'editor' });
    clearTimeout(autoPendiente);
    autoPendiente = setTimeout(function () { if (d.sucio) guardarBorrador(actual); else borrarBorrador(actual); }, 400);
    sugerir();
    programarCodificacion();
  }

  // --- autocompletado (RF-IDE-012) ------------------------------------------------------
  var sug = { abierta: false, items: [], sel: 0, desde: 0 };

  function etiquetasDelTexto(texto) {
    var r = [], re = /^[ \t]*([A-Za-z_]\w*):/gm, m;
    while ((m = re.exec(texto))) r.push(m[1]);
    if (lenguaje() === 'sgt') { re = /\bfuncion\s+([A-Za-z_]\w*)/g; while ((m = re.exec(texto))) r.push(m[1]); }
    simbolos.forEach(function (s) { r.push(s.nombre); });
    return r.filter(function (v, i, a) { return a.indexOf(v) === i; });
  }

  function candidatos(prefijo, antesEnLinea) {
    var p = prefijo.toUpperCase(), res = [];
    function agregar(lista, tipo) {
      lista.forEach(function (x) { if (x.toUpperCase().indexOf(p) === 0 && x.toUpperCase() !== p) res.push({ texto: x, tipo: tipo }); });
    }
    if (lenguaje() === 'sgt') {
      agregar(['const', 'var', 'funcion', 'estructura', 'si', 'sino', 'mientras', 'bucle', 'romper', 'continuar', 'devolver', 'segun', 'otro', 'como', 'verdadero', 'falso', 'importar', 'asm', 'u16', 'i16', 'bool'], 'palabra');
      agregar(etiquetasDelTexto(abiertos[actual].texto), 'símbolo');
      return res;
    }
    if (prefijo[0] === '.') { agregar(L.lenguaje.listas.directivas, 'directiva'); return res; }
    var sinEtiqueta = antesEnLinea.replace(/^\s*[A-Za-z_]\w*:/, '');
    var alInicio = /^\s*[A-Za-z_.]*$/.test(sinEtiqueta);
    if (alInicio) agregar(L.lenguaje.listas.mnemonicos, 'instrucción');
    else agregar(L.lenguaje.listas.registros, 'registro');
    agregar(etiquetasDelTexto(abiertos[actual].texto), 'etiqueta');
    return res;
  }

  function sugerir() {
    if (!actual || lenguaje() === 'texto') return cerrarSugerencias();
    var pos = ui.area.selectionStart;
    var antes = ui.area.value.slice(0, pos);
    var linea = antes.slice(antes.lastIndexOf('\n') + 1);
    var m = /[A-Za-z_.@][\w]*$/.exec(linea);
    if (!m || ui.area.selectionEnd !== pos) return cerrarSugerencias();
    var lista = candidatos(m[0], linea.slice(0, linea.length - m[0].length) + '');
    if (!lista.length) return cerrarSugerencias();
    sug.abierta = true; sug.items = lista.slice(0, 12); sug.sel = 0; sug.desde = pos - m[0].length;
    pintarSugerencias();
  }

  function pintarSugerencias() {
    ui.sug.textContent = '';
    sug.items.forEach(function (it, i) {
      ui.sug.append(L.el('li', {
        role: 'option', id: 'sug-' + i, 'aria-selected': i === sug.sel ? 'true' : 'false',
        onmousedown: function (ev) { ev.preventDefault(); sug.sel = i; aceptarSugerencia(); }
      }, [L.el('span', { texto: it.texto }), L.el('span', { class: 'suave', texto: ' ' + it.tipo })]));
    });
    ui.sug.hidden = false;
    ui.area.setAttribute('aria-expanded', 'true');
    ui.area.setAttribute('aria-activedescendant', 'sug-' + sug.sel);
    // Posicion bajo el cursor (fuente monoespaciada).
    var antes = ui.area.value.slice(0, ui.area.selectionStart), lineas = antes.split('\n');
    var cs = getComputedStyle(ui.area), ancho = parseFloat(cs.fontSize) * 0.6, alto = parseFloat(cs.lineHeight) || 20;
    ui.sug.style.left = (parseFloat(cs.paddingLeft) + lineas[lineas.length - 1].length * ancho - ui.area.scrollLeft) + 'px';
    ui.sug.style.top = (parseFloat(cs.paddingTop) + lineas.length * alto - ui.area.scrollTop) + 'px';
  }

  function cerrarSugerencias() {
    sug.abierta = false;
    if (ui.sug) { ui.sug.hidden = true; }
    if (ui.area) { ui.area.setAttribute('aria-expanded', 'false'); ui.area.removeAttribute('aria-activedescendant'); }
  }

  function aceptarSugerencia() {
    if (!sug.abierta) return;
    var it = sug.items[sug.sel], pos = ui.area.selectionStart, v = ui.area.value;
    ui.area.value = v.slice(0, sug.desde) + it.texto + v.slice(pos);
    var np = sug.desde + it.texto.length;
    ui.area.setSelectionRange(np, np);
    cerrarSugerencias();
    alEditar();
    cerrarSugerencias();
  }

  // --- codificacion bajo el cursor (RF-IDE-013) -----------------------------------------
  var temporizadorCodificacion = null;
  function lineaActual() {
    return ui.area.value.slice(0, ui.area.selectionStart).split('\n').length;
  }
  function programarCodificacion() {
    clearTimeout(temporizadorCodificacion);
    temporizadorCodificacion = setTimeout(mostrarCodificacion, 350);
  }
  function mostrarCodificacion() {
    var pie = document.getElementById('pie-codificacion');
    if (!pie) return;
    pie.textContent = '';
    if (!actual || lenguaje() !== 'asm' || !L.proyecto.maquina) return;
    var n = lineaActual(), texto = ui.area.value.split('\n')[n - 1] || '';
    if (!texto.replace(/;.*/, '').replace(/^\s*\w+:/, '').trim() || /^\s*[.\w]+\s*:?\s*$/.test(texto) && /:\s*$/.test(texto)) return;
    var ruta = actual;
    guardarArchivo(ruta).then(function (ok) {
      if (!ok || ruta !== actual) return;
      return L.api('POST', '/api/codificacion', { maquina: L.proyecto.maquina, fuente: ruta, linea: n }).then(function (r) {
        if (!r.ok || ruta !== actual || n !== lineaActual()) return;
        var p = r.datos.palabras || [];
        if (!p.length) return;
        pie.textContent = 'L' + n + ' · ' + p.map(function (w) { return '0x' + L.hex(w.direccion, 4) + ': ' + w.binario.replace(/(.{4})(?=.)/g, '$1 ') + ' (0x' + w.hex + ')'; }).join(' | ');
        pie.setAttribute('title', 'Codificación de la instrucción bajo el cursor');
      });
    });
  }

  // --- errores en linea (RF-IDE-014) -----------------------------------------------------
  L.compilar = function () {
    if (!actual) { L.estado('Abre un archivo para compilar', 'aviso'); return Promise.resolve(false); }
    if (!L.proyecto.maquina) { L.estado('Elige una máquina', 'aviso'); return Promise.resolve(false); }
    var fuente = actual;
    return L.guardarTodo().then(function (ok) {
      if (!ok) return false;
      var ruta = /\.sgt$/.test(fuente) ? '/api/compilar' : '/api/ensamblar';
      L.estado('Compilando ' + fuente + '…');
      return L.api('POST', ruta, { maquina: L.proyecto.maquina, fuente: fuente }, 'Compilando ' + fuente).then(function (r) {
        diagnosticos = {};
        if (r.ok) {
          simbolos = r.datos.simbolos || [];
          L.estado('Compilado: ' + r.datos.palabras + ' palabras' + ((r.datos.avisos || []).length ? ', ' + r.datos.avisos.length + ' avisos' : ''), 'ok');
          L.emit('problemas', []);
        } else {
          var ds = r.datos.diagnosticos || [];
          ds.forEach(function (d) {
            diagnosticos[d.archivo] = diagnosticos[d.archivo] || {};
            diagnosticos[d.archivo][d.linea] = d.mensaje + (d.sugerencia ? ' (' + d.sugerencia + ')' : '');
          });
          L.estado(ds.length ? ds.length + ' error(es); primero en línea ' + ds[0].linea : (r.datos.error || 'Error'), 'error');
          L.emit('problemas', ds);
        }
        pintarTextoResaltado();
        return r.ok;
      });
    });
  };

  // --- ir a la definicion (RF-IDE-015) --------------------------------------------------
  function palabraBajoElCursor() {
    var v = ui.area.value, p = ui.area.selectionStart;
    var a = p, b = p;
    while (a > 0 && /[\w]/.test(v[a - 1])) a--;
    while (b < v.length && /[\w]/.test(v[b])) b++;
    return v.slice(a, b);
  }
  L.irADefinicion = function (nombre) {
    nombre = nombre || (actual && palabraBajoElCursor());
    if (!nombre) return;
    var s = simbolos.filter(function (x) { return x.nombre === nombre; })[0];
    if (s) { abrir(s.archivo, s.linea); L.estado('Definición de ' + nombre + ' en ' + s.archivo + ':' + s.linea); return; }
    // Sin ensamblado reciente: se busca en el texto abierto.
    var re = lenguaje() === 'sgt' ? new RegExp('\\b(?:funcion|const|var|estructura)\\s+' + nombre + '\\b') : new RegExp('^[ \\t]*' + nombre + '\\s*:|^[ \\t]*' + nombre + '\\s*=', 'm');
    var m = re.exec(ui.area.value);
    if (m) { irALinea(ui.area.value.slice(0, m.index).split('\n').length); L.estado('Definición de ' + nombre); return; }
    L.api('POST', '/api/buscar', { texto: nombre, mayusculas: true }).then(function (r) {
      var def = (r.datos.resultados || []).filter(function (x) { return new RegExp('^\\s*' + nombre + '\\s*[:=]|\\b(?:funcion|const|var)\\s+' + nombre + '\\b').test(x.texto); })[0];
      if (def) abrir(def.ruta, def.linea); else L.estado('No encuentro la definición de ' + nombre, 'aviso');
    });
  };

  // --- construccion del panel -----------------------------------------------------------
  L.paneles.registrar({
    id: 'editor', titulo: 'Editor', zona: 'centro',
    construir: function (cuerpo) {
      ui.avisos = L.el('div', { class: 'avisos' });
      ui.pestanas = L.el('div', { role: 'tablist', 'aria-label': 'Archivos abiertos', class: 'pestanas' });
      ui.resalte = L.el('pre', { class: 'resalte', 'aria-hidden': 'true' });
      ui.capa = L.el('div', { class: 'capa' }, [ui.resalte]);
      ui.numeros = L.el('div', { class: 'numeros', 'aria-hidden': 'true' });
      ui.area = L.el('textarea', {
        class: 'area', spellcheck: 'false', wrap: 'off', autocapitalize: 'off', autocomplete: 'off', role: 'combobox',
        'aria-autocomplete': 'list', 'aria-expanded': 'false', 'aria-controls': 'sugerencias', 'aria-label': 'Editor', disabled: true
      });
      ui.sug = L.el('ul', { class: 'lista sugerencias', id: 'sugerencias', role: 'listbox', 'aria-label': 'Sugerencias', hidden: true });
      var marco = L.el('div', { class: 'marco-editor' }, [L.el('div', { class: 'canal' }, [ui.numeros]), L.el('div', { class: 'texto' }, [ui.capa, ui.area, ui.sug])]);
      cuerpo.append(ui.avisos, ui.pestanas, marco);
      ui.area.addEventListener('input', alEditar);
      ui.area.addEventListener('scroll', sincronizarDesplazamiento);
      ui.area.addEventListener('keyup', function (ev) { if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].indexOf(ev.key) >= 0) { if (!sug.abierta) programarCodificacion(); } });
      ui.area.addEventListener('click', function (ev) {
        cerrarSugerencias(); programarCodificacion();
        if (ev.ctrlKey || ev.metaKey) L.irADefinicion();
      });
      ui.area.addEventListener('blur', function () { setTimeout(cerrarSugerencias, 100); });
      ui.area.addEventListener('keydown', function (ev) {
        if (sug.abierta) {
          if (ev.key === 'ArrowDown') { ev.preventDefault(); sug.sel = (sug.sel + 1) % sug.items.length; pintarSugerencias(); return; }
          if (ev.key === 'ArrowUp') { ev.preventDefault(); sug.sel = (sug.sel + sug.items.length - 1) % sug.items.length; pintarSugerencias(); return; }
          if (ev.key === 'Enter' || ev.key === 'Tab') { ev.preventDefault(); aceptarSugerencia(); return; }
          if (ev.key === 'Escape') { ev.preventDefault(); cerrarSugerencias(); return; }
        }
        if (ev.key === 'Tab' && !ev.ctrlKey && !ev.altKey && !ev.shiftKey && !sug.abierta) {
          // Tabulador: ocho espacios para columnar el ensamblador; Escape y luego Tab sale del editor.
          ev.preventDefault();
          var p = ui.area.selectionStart, v = ui.area.value;
          ui.area.value = v.slice(0, p) + '        ' + v.slice(ui.area.selectionEnd);
          ui.area.setSelectionRange(p + 8, p + 8);
          alEditar();
        }
      });
    }
  });

  L.on('estado', function (e) {
    var antes = lineaEnEjecucion();
    ejecucion = e && e.ubicacion ? e.ubicacion : null;
    if (antes === lineaEnEjecucion()) return;
    pintarTextoResaltado();
    var n = lineaEnEjecucion();
    if (n && ui.area) { // que la linea quede a la vista
      var alto = ui.area.clientHeight, y = (n - 1) * 1.55 * 0.96 * parseFloat(getComputedStyle(document.documentElement).fontSize);
      if (y < ui.area.scrollTop || y > ui.area.scrollTop + alto - 40) { ui.area.scrollTop = Math.max(0, y - alto / 3); sincronizarDesplazamiento(); }
    }
  });
  L.on('abrir-archivo', function (d) { abrir(d.ruta, d.linea); });
  L.on('maquina', function (m) { L.cargarLenguaje(m).then(function () { pintarTextoResaltado(); }); });
  L.on('lenguaje', function () { pintarTextoResaltado(); });
  L.on('proyecto', function () {
    if (!L.proyecto.maquina) return;
    L.cargarLenguaje(L.proyecto.maquina).then(function () {
      var previos = L.guardado.leer('abiertos', []).filter(function (r) {
        return L.proyecto.archivos.some(function (a) { return a.ruta === r; });
      });
      var ultimo = L.guardado.leer('actual', null);
      if (!orden.length) {
        previos.reduce(function (p, r) { return p.then(function () { return abrir(r); }); }, Promise.resolve()).then(function () {
          if (ultimo && abiertos[ultimo]) activar(ultimo);
          document.body.setAttribute('data-editor-listo', '1');
        });
      }
    });
  });

  L.atajos.registrar('guardar', function () { if (actual) guardarArchivo(actual); });
  L.atajos.registrar('compilar', function () { L.compilar(); });
  L.atajos.registrar('ir-definicion', function () { L.irADefinicion(); });

  // Cambia el texto de un archivo abierto desde fuera del cuadro de texto (el diagrama de arquitectura).
  function ponerTexto(ruta, texto) {
    var d = abiertos[ruta];
    if (!d) return false;
    d.texto = texto; d.sucio = d.texto !== d.original;
    if (ruta === actual) { ui.area.value = texto; pintarTextoResaltado(); sincronizarDesplazamiento(); }
    pintarPestanas(); guardarBorrador(ruta);
    return true;
  }

  L.editor = {
    ponerTexto: ponerTexto,
    abrir: abrir, activo: function () { return actual; }, texto: function () { return actual ? abiertos[actual].texto : ''; },
    diagnosticos: function () { return diagnosticos; }, irALinea: irALinea, guardar: function () { return actual ? guardarArchivo(actual) : Promise.resolve(true); },
    recargar: function (ruta) {
      // Tras un reemplazo en disco: relee los archivos abiertos que no tengan cambios propios.
      return Promise.all(orden.map(function (r) {
        if (abiertos[r].sucio) return null;
        return L.api('GET', '/api/archivo?ruta=' + encodeURIComponent(r)).then(function (x) {
          if (x.ok) { abiertos[r].texto = abiertos[r].original = x.datos.texto; if (r === actual) { ui.area.value = x.datos.texto; pintarTextoResaltado(); } }
        });
      }));
    }
  };
})();
