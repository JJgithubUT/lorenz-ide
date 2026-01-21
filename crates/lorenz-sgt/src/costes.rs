//! Estimacion del coste en ciclos de cada funcion (RF-SGT-030, RF-SGT-031).
//!
//! El coste sale del ensamblador generado y de los ciclos que declara cada instruccion en la
//! descripcion de la maquina; no hay una tabla propia del compilador. Es una cota del peor caso
//! para una pasada por el codigo de la funcion:
//! - cada instruccion cuenta una vez, y un salto cuenta su coste mas alto;
//! - cada llamada suma el coste total de la funcion llamada;
//! - un bucle cuenta una sola iteracion: `bucles` avisa de que el coste real puede ser mayor.
//!
//! Como en SGT no hay recursion (D-059), el coste total de una funcion siempre se puede calcular.

use std::collections::HashMap;

use lorenz_arch::modelo::Maquina;

use crate::diagnostico::Diagnostico;
use crate::emisor::Linea;
use crate::hir::{ClaseExpr, ClaseSent, ClaseSitio, Expr, Funcion, Modulo, Sent, Sitio};
use crate::podar::Sustitucion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coste {
    pub nombre: String,
    /// Nombre escrito en el fuente.
    pub base: String,
    /// Ciclos de las instrucciones de la propia funcion.
    pub propios: u64,
    /// Ciclos propios mas los de todo lo que llama.
    pub total: u64,
    pub instrucciones: u64,
    /// La funcion (o algo que llama) tiene bucles: `total` cuenta una iteracion de cada uno.
    pub bucles: bool,
    pub presupuesto: Option<u64>,
}

/// Calcula el coste de cada funcion del modulo, en el orden del modulo.
pub fn estimar(
    m: &Modulo,
    maq: &Maquina,
    lineas: &[Linea],
    sustituye: Sustitucion<'_>,
) -> Vec<Coste> {
    let ciclos: HashMap<String, u64> = maq
        .instrucciones
        .iter()
        .map(|i| {
            (
                i.mnemonico.to_ascii_uppercase(),
                u64::from(i.ciclos.peor_caso()),
            )
        })
        .collect();
    // Las lineas de cada funcion, desde su etiqueta hasta la de la siguiente.
    let etiquetas: HashMap<String, usize> = m
        .funciones
        .iter()
        .enumerate()
        .map(|(i, f)| (format!("f_{}:", f.nombre), i))
        .collect();
    let mut propios = vec![0u64; m.funciones.len()];
    let mut instrucciones = vec![0u64; m.funciones.len()];
    let mut actual: Option<usize> = None;
    for l in lineas {
        let t = l.texto.trim();
        if let Some(&i) = etiquetas.get(t) {
            actual = Some(i);
            continue;
        }
        let Some(i) = actual else { continue };
        if t.is_empty() || t.ends_with(':') || t.starts_with('.') || t.starts_with(';') {
            continue;
        }
        let mnemonico = t.split_whitespace().next().unwrap_or("");
        if let Some(c) = ciclos.get(&mnemonico.to_ascii_uppercase()) {
            propios[i] += c;
            instrucciones[i] += 1;
        }
    }
    let mut memo: HashMap<usize, (u64, bool)> = HashMap::new();
    (0..m.funciones.len())
        .map(|i| {
            let (total, bucles) = total(m, i, &propios, sustituye, &mut memo, &mut Vec::new());
            Coste {
                nombre: m.funciones[i].nombre.clone(),
                base: m.funciones[i].base.clone(),
                propios: propios[i],
                total,
                instrucciones: instrucciones[i],
                bucles,
                presupuesto: m.funciones[i].presupuesto,
            }
        })
        .collect()
}

fn total(
    m: &Modulo,
    f: usize,
    propios: &[u64],
    sustituye: Sustitucion<'_>,
    memo: &mut HashMap<usize, (u64, bool)>,
    pila: &mut Vec<usize>,
) -> (u64, bool) {
    if let Some(&r) = memo.get(&f) {
        return r;
    }
    if pila.contains(&f) {
        return (0, true);
    }
    pila.push(f);
    let fun = &m.funciones[f];
    let mut llamadas = Vec::new();
    let mut bucles = false;
    recorrer(m, fun, sustituye, &mut llamadas, &mut bucles);
    let mut suma = propios[f];
    for g in llamadas {
        let (c, b) = total(m, g, propios, sustituye, memo, pila);
        suma += c;
        bucles |= b;
    }
    pila.pop();
    memo.insert(f, (suma, bucles));
    (suma, bucles)
}

/// Lo que hace falta para seguir las llamadas implicitas de los operadores sin instruccion.
struct Ctx<'a> {
    m: &'a Modulo,
    raiz: usize,
    sustituye: Sustitucion<'a>,
}

fn recorrer(
    m: &Modulo,
    f: &Funcion,
    sustituye: Sustitucion<'_>,
    llamadas: &mut Vec<usize>,
    bucles: &mut bool,
) {
    let cx = Ctx {
        m,
        raiz: f.raiz,
        sustituye,
    };
    for s in &f.cuerpo {
        sentencia(&cx, s, llamadas, bucles);
    }
}

fn sentencia(cx: &Ctx, s: &Sent, ll: &mut Vec<usize>, b: &mut bool) {
    match &s.clase {
        ClaseSent::Asignar(si, v) => {
            sitio(cx, si, ll);
            expr(cx, v, ll);
        }
        ClaseSent::Si(c, x, y) => {
            expr(cx, c, ll);
            x.iter().chain(y).for_each(|s| sentencia(cx, s, ll, b));
        }
        ClaseSent::Mientras(c, cuerpo) => {
            *b = true;
            expr(cx, c, ll);
            cuerpo.iter().for_each(|s| sentencia(cx, s, ll, b));
        }
        ClaseSent::Bucle(cuerpo) => {
            *b = true;
            cuerpo.iter().for_each(|s| sentencia(cx, s, ll, b));
        }
        ClaseSent::Devolver(Some(e)) | ClaseSent::Expr(e) => expr(cx, e, ll),
        ClaseSent::Segun {
            valor,
            brazos,
            otro,
        } => {
            expr(cx, valor, ll);
            for (_, c) in brazos {
                c.iter().for_each(|s| sentencia(cx, s, ll, b));
            }
            otro.iter().for_each(|s| sentencia(cx, s, ll, b));
        }
        ClaseSent::Salida(p, v) => {
            expr(cx, p, ll);
            expr(cx, v, ll);
        }
        ClaseSent::Romper
        | ClaseSent::Continuar
        | ClaseSent::Devolver(None)
        | ClaseSent::Asm(..) => {}
    }
}

fn sitio(cx: &Ctx, s: &Sitio, ll: &mut Vec<usize>) {
    match &s.clase {
        ClaseSitio::Variable(_) | ClaseSitio::Registro(_) => {}
        ClaseSitio::Indice(b, i, _) => {
            sitio(cx, b, ll);
            expr(cx, i, ll);
        }
        ClaseSitio::Campo(b, _) => sitio(cx, b, ll),
        ClaseSitio::Deref(p) => expr(cx, p, ll),
    }
}

fn expr(cx: &Ctx, e: &Expr, ll: &mut Vec<usize>) {
    match &e.clase {
        ClaseExpr::Const(_) | ClaseExpr::Bandera(_) => {}
        ClaseExpr::Leer(s) | ClaseExpr::Direccion(s) => sitio(cx, s, ll),
        ClaseExpr::Binario(op, a, b) => {
            expr(cx, a, ll);
            expr(cx, b, ll);
            if let Some(nombre) = (cx.sustituye)(*op, &a.tipo) {
                let id =
                    cx.m.funciones
                        .iter()
                        .position(|f| f.base == nombre && f.raiz == cx.raiz);
                ll.extend(id);
            }
        }
        ClaseExpr::Unario(_, a) | ClaseExpr::Cast(a) | ClaseExpr::Entrada(a) => expr(cx, a, ll),
        ClaseExpr::Llamada(id, args) => {
            args.iter().for_each(|a| expr(cx, a, ll));
            ll.push(*id);
        }
    }
}

/// Los diagnosticos de las funciones que pasan de su presupuesto.
pub fn comprobar_presupuestos(m: &Modulo, costes: &[Coste]) -> Vec<Diagnostico> {
    costes
        .iter()
        .zip(&m.funciones)
        .filter_map(|(c, f)| {
            let p = c.presupuesto?;
            (c.total > p).then(|| {
                let nota = if c.bucles {
                    " (contando una sola iteracion de cada bucle: el coste real puede ser mayor)"
                } else {
                    ""
                };
                Diagnostico::nuevo(
                    &m.archivo,
                    f.lugar,
                    format!(
                        "la funcion `{}` cuesta hasta {} ciclos en el peor caso y su presupuesto es de {p}{nota}",
                        f.base, c.total
                    ),
                )
                .con_sugerencia("sube el presupuesto, simplifica la funcion o usa un nivel de optimizacion mayor (-O2, -O3)")
            })
        })
        .collect()
}
