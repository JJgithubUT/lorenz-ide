// Depurador grafico: sesion, ejecucion, registros como lamparas, desensamblado con puntos de
// ruptura y memoria (RF-IDE-019, RF-IDE-021, RF-IDE-033, RF-DEP-044).
'use strict';
(function () {
  var L = window.L;
  var sesion = L.sesion = { id: null, estado: null, previo: null, corriendo: false, rupturas: [], programa: null, maquina: null, lento: 0 };
  var botones = {};

  function mensajeDeParada(p) {
    switch (p.tipo) {
      case 'detenida': return 'Máquina detenida';
      case 'ruptura': return 'Punto de ruptura en 0x' + L.hex(p.direccion, 4);
      case 'observacion': return 'Punto de observación';
      case 'llegada': return 'Llegó a 0x' + L.hex(p.direccion, 4);
      case 'retorno': return 'Retorno de subrutina';
      case 'limite': return 'Límite de instrucciones';
      case 'fallo': return 'Fallo: ' + p.mensaje + ' (instrucción en 0x' + L.hex(p.direccion, 4) + ')';
      case 'condicion': return 'Condición inválida: ' + p.mensaje;
      default: return '';
    }
  }

  function aplicar(estado) {
    sesion.previo = sesion.estado;
    sesion.estado = estado;
    L.emit('estado', estado);
    actualizarBotones();
  }

  function actualizarBotones() {
    var abierta = sesion.id != null, c = sesion.corriendo;
    if (botones.ejecutar) {
      botones.ejecutar.disabled = c;
      botones.pausar.disabled = !c;
      botones.paso.disabled = c;
      botones.atras.disabled = c || !abierta;
      botones.ruptura.disabled = !abierta;
    }
  }

  function cerrarSesion() {
    if (sesion.id == null) return Promise.resolve();
    var id = sesion.id;
    sesion.id = null; sesion.estado = null; sesion.rupturas = [];
    return L.api('DELETE', '/api/sesiones/' + id);
  }

  // Abre una sesion nueva con la maquina y el programa elegidos; guarda y ensambla antes.
  function abrirSesion() {
    var programa = L.proyecto.programa, maquina = L.proyecto.maquina;
    if (!programa || !maquina) { L.estado('Elige una máquina y un programa', 'aviso'); return Promise.resolve(false); }
    return L.guardarTodo().then(function () {
      return cerrarSesion();
    }).then(function () {
      return L.api('POST', '/api/sesiones', { maquina: maquina, programa: programa }, 'Cargando ' + programa);
    }).then(function (r) {
      if (!r.ok) {
        L.estado('No se pudo abrir la sesión: ' + (r.datos.error || r.estado), 'error');
        L.emit('problemas', erroresDeTexto(r.datos.error || '', programa));
        return false;
      }
      L.emit('problemas', []);
      sesion.id = r.datos.id; sesion.programa = programa; sesion.maquina = maquina;
      return L.api('POST', '/api/sesiones/' + sesion.id + '/grabar', { activa: true }).then(function (g) {
        aplicar(g.datos);
        // Las rupturas del programa sobreviven a reiniciar y a recargar la pagina.
        var previas = L.guardado.leer('rupturas', {})[programa] || [];
        return previas.reduce(function (p, r) {
          return p.then(function () { return L.api('POST', '/api/sesiones/' + sesion.id + '/rupturas', { direccion: r.direccion, condicion: r.condicion || undefined }); });
        }, Promise.resolve()).then(refrescarRupturas).then(function () { L.estado('Sesión lista: ' + programa, 'ok'); return true; });
      });
    });
  }

  function erroresDeTexto(texto, programa) {
    return texto.split('\n').map(function (l) {
      var m = /^(.*?):(\d+):(?:\d+:)?\s*(.*)$/.exec(l);
      return m ? { archivo: m[1].replace(/^.*\//, function (x) { return x; }), linea: +m[2], mensaje: m[3] } : null;
    }).filter(Boolean);
  }

  function asegurar() {
    if (sesion.id != null && sesion.programa === L.proyecto.programa && sesion.maquina === L.proyecto.maquina) return Promise.resolve(true);
    return abrirSesion();
  }

  function refrescarRupturas() {
    if (sesion.id == null) return Promise.resolve();
    return L.api('GET', '/api/sesiones/' + sesion.id + '/rupturas').then(function (r) {
      sesion.rupturas = r.datos.rupturas || [];
      var todas = L.guardado.leer('rupturas', {});
      todas[sesion.programa] = sesion.rupturas.map(function (x) { return { direccion: x.direccion, condicion: x.condicion }; });
      L.guardado.escribir('rupturas', todas);
      L.emit('rupturas', sesion.rupturas);
    });
  }

  // --- acciones ------------------------------------------------------------------------
  var A = L.depurador = {};

  A.paso = function (n) {
    if (sesion.corriendo) return Promise.resolve();
    return asegurar().then(function (ok) {
      if (!ok) return;
      return L.api('POST', '/api/sesiones/' + sesion.id + '/paso', { n: n || 1 }).then(function (r) {
        if (!r.ok) { L.estado(r.datos.error || 'Error', 'error'); return; }
        aplicar(r.datos.estado);
        var p = r.datos.parada;
        L.estado(p.tipo === 'paso' ? 'Paso: PC = 0x' + L.hex(r.datos.estado.pc, 4) : mensajeDeParada(p), p.tipo === 'fallo' ? 'error' : '');
      });
    });
  };

  A.atras = function () {
    if (sesion.corriendo || sesion.id == null) return Promise.resolve();
    return L.api('POST', '/api/sesiones/' + sesion.id + '/atras', { n: 1 }).then(function (r) {
      if (!r.ok) { L.estado(r.datos.error || 'No se puede retroceder más', 'aviso'); return; }
      aplicar(r.datos);
      L.estado('Un paso atrás: PC = 0x' + L.hex(r.datos.pc, 4));
    });
  };

  A.ejecutar = function () {
    if (sesion.corriendo) return;
    asegurar().then(function (ok) {
      if (!ok) return;
      sesion.corriendo = true; actualizarBotones(); L.estado('Ejecutando…');
      (function ciclo() {
        if (!sesion.corriendo) return;
        var lento = sesion.lento;
        var peticion = { limite: lento ? 1 : 4000 };  // lenta: de una en una, y el depurador sigue respetando rupturas
        L.api('POST', '/api/sesiones/' + sesion.id + '/continuar', peticion).then(function (r) {
          if (!r.ok) { sesion.corriendo = false; actualizarBotones(); L.estado(r.datos.error || 'Error', 'error'); return; }
          aplicar(r.datos.estado);
          var t = r.datos.parada.tipo;
          if (sesion.corriendo && (t === 'limite' || t === 'paso') && !r.datos.estado.detenida) {
            setTimeout(ciclo, lento || 0);
          } else if (sesion.corriendo) {
            sesion.corriendo = false; actualizarBotones();
            L.estado(mensajeDeParada(r.datos.parada), t === 'fallo' ? 'error' : t === 'detenida' ? 'ok' : '');
          }
        });
      })();
    });
  };

  A.pausar = function () {
    if (!sesion.corriendo) return;
    sesion.corriendo = false; actualizarBotones();
    L.estado('Pausado' + (sesion.estado ? ' en 0x' + L.hex(sesion.estado.pc, 4) : ''));
  };

  A.reiniciar = function () {
    sesion.corriendo = false;
    return abrirSesion();
  };

  A.alternarRuptura = function (direccion) {
    return asegurar().then(function (ok) {
      if (!ok) return;
      if (direccion == null) direccion = sesion.estado.pc;
      var existente = sesion.rupturas.filter(function (r) { return r.direccion === direccion; })[0];
      var p = existente ? L.api('DELETE', '/api/sesiones/' + sesion.id + '/rupturas/' + existente.id) : L.api('POST', '/api/sesiones/' + sesion.id + '/rupturas', { direccion: direccion });
      return p.then(function (r) {
        if (!r.ok) { L.estado(r.datos.error || 'Error', 'error'); return; }
        L.estado((existente ? 'Ruptura quitada de 0x' : 'Ruptura puesta en 0x') + L.hex(direccion, 4));
        return refrescarRupturas();
      });
    });
  };

  A.editar = function (accion, cuerpo) {
    if (sesion.id == null) return Promise.resolve();
    return L.api('POST', '/api/sesiones/' + sesion.id + '/' + accion, cuerpo).then(function (r) {
      if (!r.ok) { L.estado(r.datos.error || 'Error', 'error'); return; }
      aplicar(r.datos);
    });
  };

  A.abrir = function () { return asegurar(); };
  A.aplicarEstado = aplicar;
  A.refrescarRupturas = refrescarRupturas;

  A.pedir = function (ruta) { return sesion.id == null ? Promise.resolve({ ok: false, datos: {} }) : L.api('GET', '/api/sesiones/' + sesion.id + '/' + ruta); };

  L.atajos.registrar('ejecutar', A.ejecutar);
  L.atajos.registrar('continuar-hasta-ruptura', A.ejecutar);
  L.atajos.registrar('pausar', A.pausar);
  L.atajos.registrar('paso', function () { A.paso(1); });
  L.atajos.registrar('paso-atras', A.atras);
  L.atajos.registrar('reiniciar', A.reiniciar);
  L.atajos.registrar('ruptura', function () { A.alternarRuptura(null); });

  L.on('programa', function () { if (sesion.id != null && !sesion.corriendo) { cerrarSesion().then(function () { actualizarBotones(); L.emit('estado', null); }); } });
  L.on('maquina', function () { if (sesion.id != null && !sesion.corriendo) { cerrarSesion().then(function () { actualizarBotones(); L.emit('estado', null); }); } });

  // --- barra de acciones ----------------------------------------------------------------
  (L.modulos = L.modulos || []).push(function () {
    var barra = document.getElementById('acciones');
    function boton(id, texto, accion, atajo) {
      var b = L.el('button', { type: 'button', id: 'accion-' + id, texto: texto, onclick: function () { L.atajos.lanzar(accion); } });
      function titulo() { b.title = L.atajos.nombres[accion] + (L.atajos.de(accion) ? ' (' + L.atajos.de(accion) + ')' : ''); b.setAttribute('aria-keyshortcuts', L.atajos.de(accion) || ''); }
      titulo(); L.on('atajos', titulo);
      barra.append(b);
      return b;
    }
    boton('compilar', 'Compilar', 'compilar');
    botones.ejecutar = boton('ejecutar', '▶ Ejecutar', 'ejecutar');
    botones.pausar = boton('pausar', '⏸ Pausar', 'pausar');
    botones.paso = boton('paso', '⏭ Paso', 'paso');
    botones.atras = boton('atras', '⏮ Atrás', 'paso-atras');
    boton('reiniciar', '↺ Reiniciar', 'reiniciar');
    botones.ruptura = boton('ruptura', '● Ruptura', 'ruptura');
    actualizarBotones();
  });

  // --- registros como lamparas (RF-IDE-021) -----------------------------------------------
  L.paneles.registrar({
    id: 'registros', titulo: 'Registros', zona: 'der',
    construir: function (cuerpo) {
      var modo = L.guardado.leer('registros-modo', 'lamparas');
      var filas = L.el('div', { id: 'registros-filas', role: 'table', 'aria-label': 'Registros' });
      var banderas = L.el('div', { id: 'banderas', class: 'mono', 'aria-label': 'Banderas' });
      var info = L.el('div', { id: 'contadores', class: 'suave mono' });
      var conmutar = L.el('button', {
        type: 'button', id: 'registros-modo', 'aria-pressed': modo === 'lamparas' ? 'true' : 'false',
        onclick: function () { modo = modo === 'lamparas' ? 'numeros' : 'lamparas'; L.guardado.escribir('registros-modo', modo); pintar(sesion.estado); }
      });
      function pintar(e) {
        conmutar.textContent = modo === 'lamparas' ? 'Ver como números' : 'Ver como lámparas';
        conmutar.setAttribute('aria-pressed', modo === 'lamparas' ? 'true' : 'false');
        filas.textContent = ''; banderas.textContent = ''; info.textContent = '';
        if (!e) { filas.append(L.el('p', { class: 'suave', texto: 'Sin sesión. Pulsa Paso o Ejecutar.' })); return; }
        var previos = {};
        if (sesion.previo) sesion.previo.registros.forEach(function (r) { previos[r.nombre] = r.valor; });
        e.registros.forEach(function (r) {
          var cambio = sesion.previo && previos[r.nombre] !== r.valor;
          var fila = L.el('div', { class: 'registro' + (cambio ? ' cambio' : ''), role: 'row', 'data-registro': r.nombre });
          fila.append(L.el('span', { class: 'nombre', role: 'rowheader', texto: r.nombre }));
          if (modo === 'lamparas' && r.bits) {
            var grupo = L.el('span', { class: 'lamparas', role: 'cell' });
            for (var b = r.bits - 1; b >= 0; b--) {
              (function (b) {
                var enc = ((r.valor >> b) & 1) === 1 && b < 31 || (b >= 31 && Math.floor(r.valor / Math.pow(2, b)) % 2 === 1);
                var l = L.el('button', {
                  type: 'button', class: 'lampara ' + (enc ? 'on' : 'off'), texto: enc ? '●' : '○', 'aria-pressed': enc ? 'true' : 'false',
                  'aria-label': r.nombre + ' bit ' + b + ' = ' + (enc ? 1 : 0), title: r.nombre + ' bit ' + b + ' = ' + (enc ? 1 : 0) + ' (clic para invertir)',
                  onclick: function () {
                    var mascara = Math.pow(2, b);
                    var nuevo = enc ? r.valor - mascara : r.valor + mascara;
                    A.editar('registro', { nombre: r.nombre, valor: nuevo });
                  }
                });
                grupo.append(l);
                if (b % 4 === 0 && b > 0) grupo.append(L.el('span', { class: 'sep', texto: ' ' }));
              })(b);
            }
            fila.append(grupo);
          }
          fila.append(L.el('span', { class: 'valor mono', role: 'cell', texto: '0x' + L.hex(r.valor, Math.max(2, Math.ceil((r.bits || 8) / 4))) + ' = ' + r.valor }));
          if (cambio) fila.append(L.el('span', { class: 'delta', role: 'cell', texto: ' Δ', title: 'Cambió en el último paso' }));
          filas.append(fila);
        });
        (e.banderas || []).forEach(function (f) {
          var antes = sesion.previo && sesion.previo.banderas.filter(function (x) { return x.nombre === f.nombre; })[0];
          banderas.append(L.el('span', { class: 'bandera ' + (f.valor ? 'on' : 'off'), title: 'Bandera ' + f.nombre, texto: f.nombre + (f.valor ? ' ✓1' : ' ✗0') + (antes && antes.valor !== f.valor ? 'Δ' : '') + '  ' }));
        });
        info.textContent = 'PC 0x' + L.hex(e.pc, 4) + (e.sp != null ? '  SP 0x' + L.hex(e.sp, 4) : '') + '  instr ' + e.instrucciones + '  ciclos ' + e.ciclos + (e.detenida ? '  [DETENIDA]' : '');
      }
      cuerpo.append(L.el('div', { class: 'fila' }, [conmutar]), filas, banderas, info);
      L.on('estado', pintar);
      pintar(null);
    }
  });

  // --- desensamblado con puntos de ruptura ---------------------------------------------------
  L.paneles.registrar({
    id: 'desensamblado', titulo: 'Desensamblado', zona: 'der',
    construir: function (cuerpo) {
      var lista = L.el('ul', { class: 'lista desens', id: 'desens-lista', role: 'listbox', 'aria-label': 'Instrucciones' });
      var ultima = null;
      function pintar() {
        var e = sesion.estado;
        if (!e) { lista.textContent = ''; ultima = null; return; }
        A.pedir('desensamblado?desde=' + e.arranque + '&n=200').then(function (r) {
          if (!r.ok) return;
          lista.textContent = '';
          var actual = null;
          (r.datos.lineas || []).forEach(function (l) {
            var tiene = sesion.rupturas.some(function (x) { return x.direccion === l.direccion; });
            var li = L.el('li', { role: 'option', 'aria-selected': l.actual ? 'true' : 'false', class: 'ins' + (l.actual ? ' actual' : ''), 'data-direccion': l.direccion, tabindex: '0' });
            var rup = L.el('button', {
              type: 'button', class: 'bp' + (tiene ? ' puesta' : ''), texto: tiene ? '●' : '○', 'aria-pressed': tiene ? 'true' : 'false',
              'aria-label': (tiene ? 'Quitar' : 'Poner') + ' ruptura en 0x' + L.hex(l.direccion, 4), tabindex: '-1',
              onclick: function (ev) { ev.stopPropagation(); A.alternarRuptura(l.direccion); }
            });
            li.append(rup, L.el('span', { class: 'flecha', 'aria-hidden': 'true', texto: l.actual ? '▶' : ' ' }),
              L.el('span', { class: 'dir mono', texto: ' ' + L.hex(l.direccion, 4) + ' ' }),
              L.el('span', { class: 'mono', texto: (l.etiqueta ? l.etiqueta + ': ' : '') + l.texto }));
            li.addEventListener('keydown', function (ev) { if (ev.key === ' ' || ev.key === 'Enter') { ev.preventDefault(); A.alternarRuptura(l.direccion); } });
            if (l.actual) actual = li;
            lista.append(li);
          });
          if (actual && actual.scrollIntoView) actual.scrollIntoView({ block: 'nearest' });
        });
      }
      cuerpo.append(lista);
      L.on('estado', pintar);
      L.on('rupturas', pintar);
    }
  });

  // --- memoria: editor hexadecimal en vivo (RF-DEP-037) -------------------------------------------
  L.paneles.registrar({
    id: 'memoria', titulo: 'Memoria (editor hexadecimal)', zona: 'der', colapsado: true,
    construir: function (cuerpo) {
      var desde = L.el('input', { type: 'text', id: 'memoria-desde', value: '0x0000', 'aria-label': 'Dirección inicial', size: '8' });
      var vista = L.el('div', { id: 'memoria-vista', class: 'mono hex', role: 'grid', 'aria-label': 'Memoria' });
      var previo = {};
      L.memoria = { irA: function (d) { desde.value = '0x' + L.hex(d, 4); pintar(); L.paneles.mostrar('memoria'); } };
      function dir() { var v = desde.value.trim(); var n = /^0x/i.test(v) ? parseInt(v, 16) : parseInt(v, 10); return isNaN(n) || n < 0 ? 0 : n; }
      function editar(celda, d, actual) {
        var i = L.el('input', { type: 'text', class: 'edicion', value: '0x' + L.hex(actual || 0, 2), 'aria-label': 'Nuevo valor de la celda 0x' + L.hex(d, 4), size: '6' });
        celda.replaceWith(i); i.focus(); i.select();
        var hecho = false;
        function fin(guardar) {
          if (hecho) return; hecho = true;
          if (guardar) {
            var t = i.value.trim(), v = /^0x/i.test(t) ? parseInt(t, 16) : parseInt(t, 10);
            if (isNaN(v) || v < 0) { L.estado('Valor inválido: ' + t, 'error'); pintar(); return; }
            A.editar('memoria', { direccion: d, valor: v });
          } else pintar();
        }
        i.addEventListener('keydown', function (ev) { if (ev.key === 'Enter') fin(true); else if (ev.key === 'Escape') fin(false); });
        i.addEventListener('blur', function () { fin(false); });
      }
      function pintar() {
        if (!sesion.estado) { vista.textContent = 'Sin sesión.'; return; }
        var d0 = dir(), ancho = 8, filas = 16;
        A.pedir('memoria?desde=' + d0 + '&n=' + (ancho * filas)).then(function (r) {
          if (!r.ok) { vista.textContent = r.datos.error || ''; return; }
          var c = r.datos.celdas, nuevo = {};
          var dig = Math.max(2, Math.ceil((sesion.estado.bits_palabra || 8) / 4));
          vista.textContent = '';
          for (var f = 0; f < filas; f++) {
            var fila = L.el('div', { class: 'fila-hex', role: 'row' }, [L.el('span', { class: 'dir-hex', role: 'rowheader', texto: L.hex(d0 + f * ancho, 4) + ':' })]);
            for (var i = 0; i < ancho; i++) {
              (function (d, v) {
                nuevo[d] = v;
                var cambio = previo[d] !== undefined && previo[d] !== v;
                var b = L.el('button', {
                  type: 'button', class: 'celda' + (cambio ? ' cambio' : ''), role: 'gridcell', 'data-dir': d,
                  title: 'Celda 0x' + L.hex(d, 4) + (cambio ? ' (cambió)' : '') + ' — clic para editar',
                  texto: (v == null ? '·'.repeat(dig) : L.hex(v, dig)) + (cambio ? '*' : '')
                });
                b.addEventListener('click', function () { if (v != null) editar(b, d, v); });
                fila.append(b);
              })(d0 + f * ancho + i, c[f * ancho + i]);
            }
            vista.append(fila);
          }
          previo = nuevo;
        });
      }
      desde.addEventListener('change', pintar);
      cuerpo.append(L.el('label', {}, ['Desde ', desde]), vista);
      L.on('estado', function () { if (!cuerpo.closest('.panel').classList.contains('colapsado')) pintar(); });
      L.on('disposicion', function () { if (!cuerpo.closest('.panel').classList.contains('colapsado')) pintar(); });
    }
  });

  // --- puntos de ruptura ------------------------------------------------------------------------
  L.paneles.registrar({
    id: 'rupturas', titulo: 'Puntos de ruptura', zona: 'der', colapsado: true,
    construir: function (cuerpo) {
      var lista = L.el('ul', { class: 'lista', id: 'rupturas-lista', 'aria-label': 'Puntos de ruptura' });
      cuerpo.append(lista);
      L.on('rupturas', function (rs) {
        lista.textContent = '';
        if (!rs.length) lista.append(L.el('li', { class: 'suave', texto: 'Ninguno.' }));
        rs.forEach(function (r) {
          lista.append(L.el('li', {}, [L.el('span', { class: 'mono', texto: '● 0x' + L.hex(r.direccion, 4) + ' (' + r.aciertos + ' aciertos) ' }),
            L.el('button', { type: 'button', texto: 'Quitar', 'aria-label': 'Quitar ruptura de 0x' + L.hex(r.direccion, 4), onclick: function () { A.alternarRuptura(r.direccion); } })]));
        });
      });
    }
  });
})();
