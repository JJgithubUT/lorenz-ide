// node probar.mjs lorenz_wasm.wasm maquina.toml programa.s
import { readFileSync } from 'node:fs';
import { cargarLorenz } from './lorenz.js';
const [, , wasm, maq, prog] = process.argv;
const l = await cargarLorenz(readFileSync(wasm));
const bien = l.ejecutar(readFileSync(maq, 'utf8'), readFileSync(prog, 'utf8'));
const mal = l.ejecutar(readFileSync(maq, 'utf8'), '        NOEXISTE\n');
console.log(bien.ok ? 'ok: ejecuta' : 'FALLA: ' + bien.texto);
console.log(!mal.ok ? 'ok: error explicado' : 'FALLA: no detecto el error');
console.log(bien.texto.split('\n').slice(0, 6).join(' | '));
process.exit(bien.ok && !mal.ok ? 0 : 1);
