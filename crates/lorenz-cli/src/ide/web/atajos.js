// Atajos de teclado reasignables (RF-IDE-019, RF-IDE-020).
'use strict';
(function () {
  var L = window.L;
  var predeterminados = {
    compilar: 'Ctrl+B', ejecutar: 'F5', pausar: 'F6', paso: 'F10', 'paso-atras': 'Shift+F10',
    reiniciar: 'Ctrl+Shift+F5', ruptura: 'F9', 'ir-definicion': 'F12', buscar: 'Ctrl+Shift+F',
    guardar: 'Ctrl+S', 'siguiente-panel': 'F8', 'panel-anterior': 'Shift+F8', 'continuar-hasta-ruptura': 'Ctrl+F5'
  };
  var nombres = {
    compilar: 'Compilar', ejecutar: 'Ejecutar / continuar', pausar: 'Pausar', paso: 'Avanzar un paso',
    'paso-atras': 'Retroceder un paso', reiniciar: 'Reiniciar', ruptura: 'Alternar punto de ruptura',
    'ir-definicion': 'Ir a la definición', buscar: 'Buscar en el proyecto', guardar: 'Guardar',
    'siguiente-panel': 'Siguiente panel', 'panel-anterior': 'Panel anterior', 'continuar-hasta-ruptura': 'Continuar hasta la ruptura'
  };
  var acciones = {};
  var propios = L.guardado.leer('atajos', {});

  function combinacion(ev) {
    var partes = [];
    if (ev.ctrlKey || ev.metaKey) partes.push('Ctrl');
    if (ev.altKey) partes.push('Alt');
    if (ev.shiftKey) partes.push('Shift');
    var k = ev.key;
    if (['Control', 'Shift', 'Alt', 'Meta'].indexOf(k) >= 0) return null;
    if (k.length === 1) k = k.toUpperCase();
    if (k === ' ') k = 'Espacio';
    partes.push(k);
    return partes.join('+');
  }

  L.atajos = {
    nombres: nombres,
    combinacion: combinacion,
    de: function (id) { return id in propios ? propios[id] : predeterminados[id]; },
    fijar: function (id, comb) {
      // Un atajo solo puede servir a una accion: se quita de la que lo tuviera.
      Object.keys(predeterminados).forEach(function (o) {
        if (o !== id && L.atajos.de(o) === comb) propios[o] = '';
      });
      propios[id] = comb;
      L.guardado.escribir('atajos', propios);
      L.emit('atajos');
    },
    restaurar: function () { propios = {}; L.guardado.escribir('atajos', propios); L.emit('atajos'); },
    ids: function () { return Object.keys(predeterminados); },
    registrar: function (id, f) { acciones[id] = f; },
    lanzar: function (id) { if (acciones[id]) acciones[id](); }
  };

  document.addEventListener('keydown', function (ev) {
    if (L.capturandoAtajo) return;
    var c = combinacion(ev);
    if (!c) return;
    var ids = L.atajos.ids();
    for (var i = 0; i < ids.length; i++) {
      if (L.atajos.de(ids[i]) === c && acciones[ids[i]]) {
        ev.preventDefault();
        acciones[ids[i]]();
        return;
      }
    }
  });
})();
