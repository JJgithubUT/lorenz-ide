// Cargador minimo del modulo `lorenz_wasm.wasm` (RF-ECO-029). Funciona en navegador y en Node.
export async function cargarLorenz(origen) {
  const bytes = typeof origen === 'string' ? await (await fetch(origen)).arrayBuffer() : origen;
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const x = instance.exports;
  const cod = new TextEncoder(), dec = new TextDecoder();
  const meter = (texto) => {
    const b = cod.encode(texto);
    const p = x.lorenz_reservar(b.length);
    new Uint8Array(x.memory.buffer, p, b.length).set(b);
    return [p, b.length];
  };
  return {
    // Devuelve { ok, texto }: el resumen de la ejecucion o el motivo del error.
    ejecutar(maquinaToml, fuente, limite = 100000) {
      const [mp, mn] = meter(maquinaToml);
      const [fp, fn] = meter(fuente);
      const r = x.lorenz_ejecutar(mp, mn, fp, fn, BigInt(limite));
      const n = new DataView(x.memory.buffer).getUint32(r, true);
      const texto = dec.decode(new Uint8Array(x.memory.buffer, r + 4, n));
      x.lorenz_liberar(r, 4 + n);
      x.lorenz_liberar(mp, mn);
      x.lorenz_liberar(fp, fn);
      return texto.startsWith('ok\n') ? { ok: true, texto: texto.slice(3) } : { ok: false, texto: texto.replace(/^error\n/, '') };
    },
  };
}
