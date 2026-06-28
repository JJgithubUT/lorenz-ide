// Resaltado de sintaxis de ensamblador (segun la arquitectura activa) y de SGT (RF-IDE-010, RF-IDE-011).
'use strict';
(function () {
  var L = window.L;
  var PALABRAS_SGT = ['const', 'var', 'funcion', 'estructura', 'si', 'sino', 'mientras', 'bucle', 'romper', 'continuar', 'devolver', 'segun', 'otro', 'como', 'verdadero', 'falso', 'importar', 'asm'];
  var esc = L.escapar;

  function conjunto(lista) { var s = {}; (lista || []).forEach(function (x) { s[String(x).toUpperCase()] = true; }); return s; }

  L.lenguaje = { mnemonicos: {}, registros: {}, directivas: {}, listas: { mnemonicos: [], registros: [], directivas: [] } };
  L.cargarLenguaje = function (maquina) {
    if (!maquina) return Promise.resolve();
    return L.api('GET', '/api/lenguaje?maquina=' + encodeURIComponent(maquina)).then(function (r) {
      if (!r.ok) { L.estado('Arquitectura: ' + (r.datos.error || r.estado), 'error'); return; }
      L.lenguaje = {
        mnemonicos: conjunto(r.datos.mnemonicos), registros: conjunto(r.datos.registros), directivas: conjunto(r.datos.directivas),
        listas: { mnemonicos: r.datos.mnemonicos, registros: r.datos.registros, directivas: r.datos.directivas },
        instrucciones: r.datos.instrucciones, bitsPalabra: r.datos.bits_palabra, nombre: r.datos.nombre
      };
      L.emit('lenguaje');
    });
  };

  function span(clase, texto) { return '<span class="t-' + clase + '">' + esc(texto) + '</span>'; }

  // Ensamblador: una linea de texto -> HTML.
  function lineaAsm(t) {
    var salida = '', i = 0, n = t.length, m;
    var re = /^(?:[ \t]+|;.*|"(?:[^"\\]|\\.)*"?|'(?:[^'\\]|\\.)*'?|\.[A-Za-z_]\w*|(?:0[xX][0-9a-fA-F_]+|0[bB][01_]+|\$[0-9a-fA-F]+|\d[\d_]*)\b|[A-Za-z_][\w]*:?|.)/;
    while (i < n) {
      m = re.exec(t.slice(i));
      var tok = m[0];
      if (/^[ \t]+$/.test(tok)) { salida += esc(tok); }
      else if (tok[0] === ';') salida += span('comentario', tok);
      else if (tok[0] === '"' || tok[0] === "'") salida += span('cadena', tok);
      else if (tok[0] === '.') salida += span(L.lenguaje.directivas[tok.toUpperCase()] ? 'directiva' : 'directiva', tok);
      else if (/^(0[xX]|0[bB]|\$|\d)/.test(tok)) salida += span('numero', tok);
      else if (/^[A-Za-z_]/.test(tok)) {
        if (tok[tok.length - 1] === ':') { salida += span('etiqueta', tok); }
        else if (L.lenguaje.registros[tok.toUpperCase()]) salida += span('registro', tok);
        else if (L.lenguaje.mnemonicos[tok.toUpperCase()]) salida += span('mnemonico', tok);
        else salida += span('simbolo', tok);
      } else salida += esc(tok);
      i += tok.length;
    }
    return salida;
  }

  // SGT: mantiene el estado de los comentarios de bloque entre lineas.
  function lineasSgt(lineas) {
    var enBloque = false;
    return lineas.map(function (t) {
      var s = '', i = 0;
      while (i < t.length) {
        if (enBloque) {
          var f = t.indexOf('*/', i);
          if (f < 0) { s += span('comentario', t.slice(i)); i = t.length; } else { s += span('comentario', t.slice(i, f + 2)); i = f + 2; enBloque = false; }
          continue;
        }
        var resto = t.slice(i), m;
        if (resto.indexOf('//') === 0) { s += span('comentario', resto); break; }
        if (resto.indexOf('/*') === 0) {
          var f2 = t.indexOf('*/', i + 2);
          if (f2 < 0) { s += span('comentario', resto); i = t.length; enBloque = true; } else { s += span('comentario', t.slice(i, f2 + 2)); i = f2 + 2; }
          continue;
        }
        if ((m = /^"(?:[^"\\]|\\.)*"?/.exec(resto))) { s += span('cadena', m[0]); i += m[0].length; continue; }
        if ((m = /^@\w+/.exec(resto))) { s += span('directiva', m[0]); i += m[0].length; continue; }
        if ((m = /^(?:0[xX][0-9a-fA-F_]+|0[bB][01_]+|\d[\d_]*)/.exec(resto))) { s += span('numero', m[0]); i += m[0].length; continue; }
        if ((m = /^[A-Za-z_]\w*/.exec(resto))) {
          var w = m[0];
          if (PALABRAS_SGT.indexOf(w) >= 0) s += span('mnemonico', w);
          else if (/^(?:[iu](?:8|16|32|64)|usize|bool)$/.test(w)) s += span('registro', w);
          else s += esc(w);
          i += w.length; continue;
        }
        s += esc(resto[0]); i += 1;
      }
      return s;
    });
  }

  L.lenguajeDe = function (ruta) { return /\.sgt$/.test(ruta) ? 'sgt' : /\.(s|asm|lasm)$/.test(ruta) ? 'asm' : 'texto'; };

  // Devuelve un arreglo de HTML, uno por linea.
  L.resaltar = function (texto, lenguaje) {
    var lineas = texto.split('\n');
    if (lenguaje === 'sgt') return lineasSgt(lineas);
    if (lenguaje === 'asm') return lineas.map(lineaAsm);
    return lineas.map(esc);
  };
})();
