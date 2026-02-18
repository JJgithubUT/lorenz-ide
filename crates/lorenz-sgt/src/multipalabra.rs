//! Enteros de varias palabras (RF-SGT-001, RF-SGT-002).
//!
//! Una maquina de 8 bits tiene `u16` y `u32`; una de 16 bits tiene `u32` y `u64`. Ningun
//! generador sabe operar con ellos, asi que esta pasada, que va justo despues del analisis, los
//! baja a operaciones de una palabra sobre la misma representacion intermedia:
//!
//! - cada valor ancho es una lista de palabras, de la menos a la mas significativa; en memoria
//!   cada palabra va en el desplazamiento que dicta el orden de bytes de la maquina;
//! - suma y resta propagan el acarreo palabra a palabra, comparar mira desde la palabra alta,
//!   los desplazamientos por una constante mueven palabras y bits;
//! - multiplicar, dividir y los desplazamientos variables llaman a rutinas de la biblioteca
//!   escritas en SGT (`__mul32`...), que a su vez se bajan con esta misma pasada;
//! - un parametro ancho llega partido en palabras y se copia a su variable al entrar; un
//!   resultado ancho vuelve con su primera palabra como valor de retorno y las demas en las
//!   globales `__rv1`, `__rv2`... (copiadas por raiz como el resto de la biblioteca).
//!
//! Los punteros de varias palabras y los enteros anchos en `segun` o como puerto quedan fuera.

use std::collections::HashMap;

use crate::ast::{OpBinario as B, OpUnario as U};
use crate::destino::Destino;
use crate::diagnostico::{Diagnostico, Lugar};
use crate::hir::*;
use crate::tipos::Tipo;

struct Bajador {
    w: u32,
    little: bool,
    archivo: String,
    ids: HashMap<String, usize>,
    rv: Vec<IdGlobal>,
    diag: Vec<Diagnostico>,
    // Estado de la funcion que se baja.
    locales: Vec<Variable>,
    temporales: Vec<(Tipo, IdLocal, bool)>,
    lugar: Lugar,
    ret_palabras: usize,
}

/// Anchos de entero mayores que la palabra que la maquina soporta.
pub fn anchos_anchos(d: &Destino) -> Vec<u32> {
    d.anchos_de_entero()
        .into_iter()
        .filter(|b| *b > d.palabra)
        .collect()
}

pub fn bajar(m: &mut Modulo, d: &Destino) -> Result<(), Vec<Diagnostico>> {
    let w = d.palabra;
    let maximo = anchos_anchos(d).last().map_or(1, |b| (b / w) as usize);
    let mut b = Bajador {
        w,
        little: d.little,
        archivo: m.archivo.clone(),
        ids: HashMap::new(),
        rv: Vec::new(),
        diag: Vec::new(),
        locales: Vec::new(),
        temporales: Vec::new(),
        lugar: Lugar::nuevo(1, 1),
        ret_palabras: 1,
    };
    for (i, f) in m.funciones.iter().enumerate() {
        b.ids.insert(f.nombre.clone(), i);
    }
    for k in 1..maximo {
        b.rv.push(m.globales.len());
        m.globales.push(Variable {
            nombre: format!("__rv{k}"),
            tipo: Tipo::u(w),
            lugar: Lugar::nuevo(1, 1),
            direccion: None,
            inicial: None,
            datos: Vec::new(),
        });
    }
    for fi in 0..m.funciones.len() {
        let mut f = m.funciones[fi].clone();
        b.funcion(&mut f);
        m.funciones[fi] = f;
    }
    if b.diag.is_empty() {
        Ok(())
    } else {
        Err(b.diag)
    }
}

impl Bajador {
    fn tw(&self) -> Tipo {
        Tipo::u(self.w)
    }

    /// Palabras de un entero ancho; `None` si cabe en una.
    fn multi(&self, t: &Tipo) -> Option<usize> {
        match t {
            Tipo::Entero { bits, .. } if *bits > self.w => Some((*bits / self.w) as usize),
            _ => None,
        }
    }

    fn error(&mut self, msg: impl Into<String>) {
        let d = Diagnostico::nuevo(&self.archivo, self.lugar, msg.into());
        self.diag.push(d);
    }

    fn mascara(&self) -> i128 {
        (1i128 << self.w) - 1
    }

    fn cw(&self, v: i128) -> Expr {
        Expr {
            lugar: self.lugar,
            tipo: self.tw(),
            clase: ClaseExpr::Const(v & self.mascara()),
        }
    }

    fn cbool(&self, v: bool) -> Expr {
        Expr {
            lugar: self.lugar,
            tipo: Tipo::Bool,
            clase: ClaseExpr::Const(i128::from(v)),
        }
    }

    fn bin(&self, op: B, a: Expr, b: Expr) -> Expr {
        let tipo = if op.es_comparacion() || matches!(op, B::YLogico | B::OLogico) {
            Tipo::Bool
        } else {
            a.tipo.clone()
        };
        Expr {
            lugar: self.lugar,
            tipo,
            clase: ClaseExpr::Binario(op, Box::new(a), Box::new(b)),
        }
    }

    fn cast(&self, e: Expr, t: Tipo) -> Expr {
        if e.tipo == t {
            return e;
        }
        Expr {
            lugar: self.lugar,
            tipo: t,
            clase: ClaseExpr::Cast(Box::new(e)),
        }
    }

    fn no(&self, e: Expr) -> Expr {
        Expr {
            lugar: self.lugar,
            tipo: Tipo::Bool,
            clase: ClaseExpr::Unario(U::No, Box::new(e)),
        }
    }

    fn leer_local(&self, id: IdLocal) -> Expr {
        let tipo = self.locales[id].tipo.clone();
        self.leer(Sitio {
            tipo,
            clase: ClaseSitio::Variable(Ref::Local(id)),
        })
    }

    fn leer(&self, s: Sitio) -> Expr {
        Expr {
            lugar: self.lugar,
            tipo: s.tipo.clone(),
            clase: ClaseExpr::Leer(Box::new(s)),
        }
    }

    fn sent(&self, clase: ClaseSent) -> Sent {
        Sent {
            lugar: self.lugar,
            clase,
        }
    }

    /// Desplazamiento en memoria de la palabra de significancia `k` de un valor de `n` palabras.
    fn desp(&self, k: usize, n: usize) -> u64 {
        (if self.little { k } else { n - 1 - k }) as u64
    }

    /// El sitio de una palabra de un valor ancho.
    fn sitio_palabra(&self, s: &Sitio, off: u64) -> Sitio {
        let tipo = self.tw();
        match &s.clase {
            ClaseSitio::Campo(b, o) => Sitio {
                tipo,
                clase: ClaseSitio::Campo(b.clone(), o + off),
            },
            _ => Sitio {
                tipo,
                clase: ClaseSitio::Campo(Box::new(s.clone()), off),
            },
        }
    }

    // ------------------------------------------------------------------
    // Temporales
    // ------------------------------------------------------------------

    fn liberar(&mut self) {
        for t in &mut self.temporales {
            t.2 = false;
        }
    }

    fn temp(&mut self, tipo: &Tipo) -> IdLocal {
        if let Some(t) = self
            .temporales
            .iter_mut()
            .find(|(t, _, usado)| !*usado && t == tipo)
        {
            t.2 = true;
            return t.1;
        }
        let id = self.locales.len();
        self.locales.push(Variable {
            nombre: format!("__t{id}"),
            tipo: tipo.clone(),
            lugar: self.lugar,
            direccion: None,
            inicial: None,
            datos: Vec::new(),
        });
        self.temporales.push((tipo.clone(), id, true));
        id
    }

    fn asignar_local(&self, id: IdLocal, v: Expr) -> Sent {
        let tipo = self.locales[id].tipo.clone();
        self.sent(ClaseSent::Asignar(
            Sitio {
                tipo,
                clase: ClaseSitio::Variable(Ref::Local(id)),
            },
            v,
        ))
    }

    /// Guarda el valor en un temporal nuevo y devuelve su lectura.
    fn mat(&mut self, e: Expr, pre: &mut Vec<Sent>) -> Expr {
        let t = self.temp(&e.tipo.clone());
        pre.push(self.asignar_local(t, e));
        self.leer_local(t)
    }

    fn es_simple(e: &Expr) -> bool {
        match &e.clase {
            ClaseExpr::Const(_) => true,
            ClaseExpr::Leer(s) => {
                let mut s = &**s;
                loop {
                    match &s.clase {
                        ClaseSitio::Variable(_) => return true,
                        ClaseSitio::Campo(b, _) => s = b,
                        _ => return false,
                    }
                }
            }
            _ => false,
        }
    }

    fn simple(&mut self, e: Expr, pre: &mut Vec<Sent>) -> Expr {
        if Self::es_simple(&e) {
            e
        } else {
            self.mat(e, pre)
        }
    }

    fn puro_expr(e: &Expr) -> bool {
        match &e.clase {
            ClaseExpr::Const(_) | ClaseExpr::Bandera(_) => true,
            ClaseExpr::Leer(s) | ClaseExpr::Direccion(s) => Self::puro_sitio(s),
            ClaseExpr::Binario(_, a, b) => Self::puro_expr(a) && Self::puro_expr(b),
            ClaseExpr::Unario(_, a) | ClaseExpr::Cast(a) => Self::puro_expr(a),
            ClaseExpr::Llamada(..) | ClaseExpr::Entrada(_) => false,
        }
    }

    fn puro_sitio(s: &Sitio) -> bool {
        match &s.clase {
            ClaseSitio::Variable(_) | ClaseSitio::Registro(_) => true,
            ClaseSitio::Indice(b, i, _) => Self::puro_sitio(b) && Self::puro_expr(i),
            ClaseSitio::Campo(b, _) => Self::puro_sitio(b),
            ClaseSitio::Deref(p) => Self::puro_expr(p),
        }
    }

    // ------------------------------------------------------------------
    // Funciones y sentencias
    // ------------------------------------------------------------------

    fn funcion(&mut self, f: &mut Funcion) {
        self.locales = std::mem::take(&mut f.locales);
        self.temporales.clear();
        self.lugar = f.lugar;
        self.ret_palabras = self.multi(&f.retorno).unwrap_or(1);
        let mut parametros = Vec::new();
        let mut prologo = Vec::new();
        for &p in &f.parametros {
            let tipo = self.locales[p].tipo.clone();
            if let Some(n) = self.multi(&tipo) {
                let base = Sitio {
                    tipo: tipo.clone(),
                    clase: ClaseSitio::Variable(Ref::Local(p)),
                };
                for k in 0..n {
                    let id = self.locales.len();
                    self.locales.push(Variable {
                        nombre: format!("{}__{k}", self.locales[p].nombre),
                        tipo: self.tw(),
                        lugar: self.locales[p].lugar,
                        direccion: None,
                        inicial: None,
                        datos: Vec::new(),
                    });
                    parametros.push(id);
                    let destino = self.sitio_palabra(&base, self.desp(k, n));
                    let v = self.leer_local(id);
                    prologo.push(self.sent(ClaseSent::Asignar(destino, v)));
                }
            } else {
                parametros.push(p);
            }
        }
        let mut cuerpo = prologo;
        let original = std::mem::take(&mut f.cuerpo);
        cuerpo.extend(self.bloque(&original));
        if self.ret_palabras > 1 {
            f.retorno = self.tw();
        }
        f.parametros = parametros;
        f.cuerpo = cuerpo;
        f.locales = std::mem::take(&mut self.locales);
    }

    fn bloque(&mut self, b: &[Sent]) -> Vec<Sent> {
        let mut out = Vec::new();
        for s in b {
            self.sent_baja(s, &mut out);
        }
        out
    }

    fn sent_baja(&mut self, s: &Sent, out: &mut Vec<Sent>) {
        self.lugar = s.lugar;
        self.liberar();
        match &s.clase {
            ClaseSent::Asignar(sitio, v) => {
                if let Some(n) = self.multi(&sitio.tipo) {
                    let ws = self.pal(v, out);
                    let s2 = self.sitio(sitio, out, true);
                    for (k, w) in ws.into_iter().enumerate() {
                        let dest = self.sitio_palabra(&s2, self.desp(k, n));
                        out.push(self.sent(ClaseSent::Asignar(dest, w)));
                    }
                } else {
                    let s2 = self.sitio(sitio, out, false);
                    let v2 = self.una(v, out);
                    out.push(self.sent(ClaseSent::Asignar(s2, v2)));
                }
            }
            ClaseSent::Si(c, a, b) => {
                let c2 = self.expr(c, out);
                let a2 = self.bloque(a);
                let b2 = self.bloque(b);
                out.push(self.sent(ClaseSent::Si(c2, a2, b2)));
            }
            ClaseSent::Mientras(c, cuerpo) => {
                let mut pre = Vec::new();
                let c2 = self.expr(c, &mut pre);
                let cuerpo2 = self.bloque(cuerpo);
                if pre.is_empty() {
                    out.push(self.sent(ClaseSent::Mientras(c2, cuerpo2)));
                } else {
                    let mut b = pre;
                    let salir = self.no(c2);
                    b.push(self.sent(ClaseSent::Si(
                        salir,
                        vec![self.sent(ClaseSent::Romper)],
                        Vec::new(),
                    )));
                    b.extend(cuerpo2);
                    out.push(self.sent(ClaseSent::Bucle(b)));
                }
            }
            ClaseSent::Bucle(b) => {
                let b2 = self.bloque(b);
                out.push(self.sent(ClaseSent::Bucle(b2)));
            }
            ClaseSent::Devolver(Some(v)) => {
                if let Some(n) = self.multi(&v.tipo) {
                    let mut ws = self.pal(v, out);
                    for k in 1..n {
                        let w = ws[k].clone();
                        let g = self.rv[k - 1];
                        let dest = Sitio {
                            tipo: self.tw(),
                            clase: ClaseSitio::Variable(Ref::Global(g)),
                        };
                        out.push(self.sent(ClaseSent::Asignar(dest, w)));
                    }
                    let w0 = ws.swap_remove(0);
                    out.push(self.sent(ClaseSent::Devolver(Some(w0))));
                } else {
                    let v2 = self.expr(v, out);
                    out.push(self.sent(ClaseSent::Devolver(Some(v2))));
                }
            }
            ClaseSent::Segun {
                valor,
                brazos,
                otro,
            } => {
                if self.multi(&valor.tipo).is_some() {
                    self.error("`segun` no admite un entero de varias palabras");
                    return;
                }
                let v2 = self.expr(valor, out);
                let brazos2 = brazos
                    .iter()
                    .map(|(c, b)| (*c, self.bloque(b)))
                    .collect::<Vec<_>>();
                let otro2 = self.bloque(otro);
                out.push(self.sent(ClaseSent::Segun {
                    valor: v2,
                    brazos: brazos2,
                    otro: otro2,
                }));
            }
            ClaseSent::Salida(p, v) => {
                if self.multi(&v.tipo).is_some() {
                    self.error("`salida` solo escribe valores de una palabra");
                    return;
                }
                let p2 = self.una(p, out);
                let v2 = self.expr(v, out);
                out.push(self.sent(ClaseSent::Salida(p2, v2)));
            }
            ClaseSent::Expr(e) => {
                if self.multi(&e.tipo).is_some() {
                    if let ClaseExpr::Llamada(id, args) = &e.clase {
                        let args2 = self.argumentos(*id, args, out);
                        let call = Expr {
                            lugar: e.lugar,
                            tipo: self.tw(),
                            clase: ClaseExpr::Llamada(*id, args2),
                        };
                        out.push(self.sent(ClaseSent::Expr(call)));
                    } else {
                        // Sin efectos: basta con evaluar lo que tenga dentro.
                        let _ = self.pal(e, out);
                    }
                } else {
                    let e2 = self.expr(e, out);
                    out.push(self.sent(ClaseSent::Expr(e2)));
                }
            }
            ClaseSent::Romper
            | ClaseSent::Continuar
            | ClaseSent::Devolver(None)
            | ClaseSent::Asm(..) => out.push(s.clone()),
        }
    }

    // ------------------------------------------------------------------
    // Sitios
    // ------------------------------------------------------------------

    /// Baja las expresiones de un sitio. Si se va a leer o escribir por palabras (`por_palabras`)
    /// y alguna parte tiene efectos, se guarda antes en un temporal para no repetirlos.
    fn sitio(&mut self, s: &Sitio, pre: &mut Vec<Sent>, por_palabras: bool) -> Sitio {
        let por_palabras = por_palabras || self.multi(&s.tipo).is_some();
        let clase = match &s.clase {
            ClaseSitio::Variable(_) | ClaseSitio::Registro(_) => return s.clone(),
            ClaseSitio::Indice(b, i, t) => {
                let b2 = self.sitio(b, pre, por_palabras);
                let mut i2 = self.una(i, pre);
                if por_palabras && !Self::puro_expr(&i2) {
                    i2 = self.mat(i2, pre);
                }
                ClaseSitio::Indice(Box::new(b2), Box::new(i2), *t)
            }
            ClaseSitio::Campo(b, o) => {
                ClaseSitio::Campo(Box::new(self.sitio(b, pre, por_palabras)), *o)
            }
            ClaseSitio::Deref(p) => {
                let mut p2 = self.una(p, pre);
                if por_palabras && !Self::puro_expr(&p2) {
                    p2 = self.mat(p2, pre);
                }
                ClaseSitio::Deref(Box::new(p2))
            }
        };
        Sitio {
            tipo: s.tipo.clone(),
            clase,
        }
    }

    // ------------------------------------------------------------------
    // Expresiones de una palabra
    // ------------------------------------------------------------------

    /// Una expresion que debe caber en una palabra; si es ancha se queda con la palabra baja.
    fn una(&mut self, e: &Expr, pre: &mut Vec<Sent>) -> Expr {
        if self.multi(&e.tipo).is_some() {
            let mut ws = self.pal(e, pre);
            ws.swap_remove(0)
        } else {
            self.expr(e, pre)
        }
    }

    fn expr(&mut self, e: &Expr, pre: &mut Vec<Sent>) -> Expr {
        let clase = match &e.clase {
            ClaseExpr::Const(_) | ClaseExpr::Bandera(_) => return e.clone(),
            ClaseExpr::Leer(s) => ClaseExpr::Leer(Box::new(self.sitio(s, pre, false))),
            ClaseExpr::Direccion(s) => ClaseExpr::Direccion(Box::new(self.sitio(s, pre, false))),
            ClaseExpr::Binario(op, a, b) => return self.binario(e, *op, a, b, pre),
            ClaseExpr::Unario(op, a) => {
                let a2 = self.expr(a, pre);
                ClaseExpr::Unario(*op, Box::new(a2))
            }
            ClaseExpr::Cast(a) => {
                if self.multi(&a.tipo).is_some() {
                    let mut ws = self.pal(a, pre);
                    let w0 = ws.swap_remove(0);
                    ClaseExpr::Cast(Box::new(w0))
                } else {
                    let a2 = self.expr(a, pre);
                    ClaseExpr::Cast(Box::new(a2))
                }
            }
            ClaseExpr::Llamada(id, args) => {
                let args2 = self.argumentos(*id, args, pre);
                ClaseExpr::Llamada(*id, args2)
            }
            ClaseExpr::Entrada(p) => {
                let p2 = self.una(p, pre);
                ClaseExpr::Entrada(Box::new(p2))
            }
        };
        Expr {
            lugar: e.lugar,
            tipo: e.tipo.clone(),
            clase,
        }
    }

    fn binario(&mut self, e: &Expr, op: B, a: &Expr, b: &Expr, pre: &mut Vec<Sent>) -> Expr {
        if self.multi(&a.tipo).is_some() {
            // Solo las comparaciones de enteros anchos devuelven una palabra.
            let (wa, wb) = self.pal2(a, b, pre);
            let signo = matches!(a.tipo, Tipo::Entero { signo: true, .. });
            return self.comparar(op, wa, wb, signo, pre);
        }
        let a2 = self.expr(a, pre);
        if matches!(op, B::YLogico | B::OLogico) {
            let mut pre_b = Vec::new();
            let b2 = self.expr(b, &mut pre_b);
            if pre_b.is_empty() {
                return self.bin(op, a2, b2);
            }
            // El segundo operando tiene preparativos: solo se hacen si hace falta.
            let t = self.temp(&Tipo::Bool);
            pre.push(self.asignar_local(t, a2));
            let r = pre_b;
            let mut cuerpo = r;
            cuerpo.push(self.asignar_local(t, b2));
            let cond = self.leer_local(t);
            let cond = if op == B::YLogico {
                cond
            } else {
                self.no(cond)
            };
            pre.push(self.sent(ClaseSent::Si(cond, cuerpo, Vec::new())));
            return self.leer_local(t);
        }
        let mut pre_b = Vec::new();
        let b2 = self.una(b, &mut pre_b);
        let a2 = if pre_b.is_empty() {
            a2
        } else {
            let a2 = if matches!(a2.clase, ClaseExpr::Const(_)) {
                a2
            } else {
                self.mat(a2, pre)
            };
            pre.append(&mut pre_b);
            a2
        };
        Expr {
            lugar: e.lugar,
            tipo: e.tipo.clone(),
            clase: ClaseExpr::Binario(op, Box::new(a2), Box::new(b2)),
        }
    }

    /// Argumentos de una llamada, con los anchos partidos en palabras.
    fn argumentos(&mut self, _id: IdFuncion, args: &[Expr], pre: &mut Vec<Sent>) -> Vec<Expr> {
        let mut todos: Vec<Vec<Expr>> = Vec::new();
        for a in args {
            let mut buf = Vec::new();
            let ws = if self.multi(&a.tipo).is_some() {
                self.pal(a, &mut buf)
            } else {
                vec![self.expr(a, &mut buf)]
            };
            if !buf.is_empty() {
                for previos in todos.iter_mut() {
                    for w in previos.iter_mut() {
                        if !matches!(w.clase, ClaseExpr::Const(_)) {
                            let v = std::mem::replace(w, self.cw(0));
                            *w = self.mat(v, pre);
                        }
                    }
                }
                pre.append(&mut buf);
            }
            todos.push(ws);
        }
        todos.into_iter().flatten().collect()
    }

    // ------------------------------------------------------------------
    // Expresiones anchas
    // ------------------------------------------------------------------

    /// Baja dos operandos anchos respetando el orden de evaluacion.
    fn pal2(&mut self, a: &Expr, b: &Expr, pre: &mut Vec<Sent>) -> (Vec<Expr>, Vec<Expr>) {
        let mut wa = self.pal(a, pre);
        let mut buf = Vec::new();
        let wb = self.pal(b, &mut buf);
        if !buf.is_empty() {
            for w in wa.iter_mut() {
                if !matches!(w.clase, ClaseExpr::Const(_)) {
                    let v = std::mem::replace(w, self.cw(0));
                    *w = self.mat(v, pre);
                }
            }
            pre.append(&mut buf);
        }
        (wa, wb)
    }

    fn consts(&self, v: i128, n: usize) -> Vec<Expr> {
        (0..n).map(|k| self.cw(v >> (k as u32 * self.w))).collect()
    }

    /// Un entero ancho como palabras, de la menos a la mas significativa, todas de tipo `u{W}`.
    fn pal(&mut self, e: &Expr, pre: &mut Vec<Sent>) -> Vec<Expr> {
        let Some(n) = self.multi(&e.tipo) else {
            // Un valor de una palabra pedido como lista.
            let v = self.expr(e, pre);
            return vec![self.cast(v, self.tw())];
        };
        match &e.clase {
            ClaseExpr::Const(v) => self.consts(*v, n),
            ClaseExpr::Leer(s) => {
                let s2 = self.sitio(s, pre, true);
                (0..n)
                    .map(|k| {
                        let p = self.sitio_palabra(&s2, self.desp(k, n));
                        self.leer(p)
                    })
                    .collect()
            }
            ClaseExpr::Cast(a) => self.cast_ancho(a, n, &e.tipo, pre),
            ClaseExpr::Unario(U::Negar, a) => {
                let wa = self.pal(a, pre);
                let ceros = self.consts(0, n);
                self.sumar(ceros, wa, true, pre)
            }
            ClaseExpr::Unario(U::Complemento, a) => {
                let wa = self.pal(a, pre);
                wa.into_iter()
                    .map(|w| Expr {
                        lugar: self.lugar,
                        tipo: self.tw(),
                        clase: ClaseExpr::Unario(U::Complemento, Box::new(w)),
                    })
                    .collect()
            }
            ClaseExpr::Binario(op, a, b) => match op {
                B::Suma | B::Resta => {
                    let (wa, wb) = self.pal2(a, b, pre);
                    self.sumar(wa, wb, *op == B::Resta, pre)
                }
                B::Y | B::O | B::Xor => {
                    let (wa, wb) = self.pal2(a, b, pre);
                    wa.into_iter()
                        .zip(wb)
                        .map(|(x, y)| self.bin(*op, x, y))
                        .collect()
                }
                B::DespIzq | B::DespDer => {
                    if let ClaseExpr::Const(c) = &b.clase {
                        let c = (*c).max(0) as u128;
                        let signo = matches!(e.tipo, Tipo::Entero { signo: true, .. });
                        let wa = self.pal(a, pre);
                        return self.desplazar(wa, c, *op == B::DespIzq, signo, pre);
                    }
                    self.rutina(e, *op, a, b, pre)
                }
                B::Mul | B::Div | B::Mod | B::SumaSat | B::RestaSat => {
                    self.rutina(e, *op, a, b, pre)
                }
                _ => {
                    self.error("operacion no disponible para enteros de varias palabras");
                    self.consts(0, n)
                }
            },
            ClaseExpr::Llamada(id, args) => {
                let args2 = self.argumentos(*id, args, pre);
                let call = Expr {
                    lugar: e.lugar,
                    tipo: self.tw(),
                    clase: ClaseExpr::Llamada(*id, args2),
                };
                let t0 = self.temp(&self.tw());
                pre.push(self.asignar_local(t0, call));
                let mut ws = vec![self.leer_local(t0)];
                for k in 1..n {
                    let g = self.rv[k - 1];
                    let lectura = self.leer(Sitio {
                        tipo: self.tw(),
                        clase: ClaseSitio::Variable(Ref::Global(g)),
                    });
                    ws.push(self.mat(lectura, pre));
                }
                ws
            }
            ClaseExpr::Entrada(_) | ClaseExpr::Bandera(_) | ClaseExpr::Direccion(_) => {
                self.error("esta expresion no puede ser un entero de varias palabras");
                self.consts(0, n)
            }
            ClaseExpr::Unario(U::No, _) => {
                self.error("esta expresion no puede ser un entero de varias palabras");
                self.consts(0, n)
            }
        }
    }

    /// Conversion hacia un entero de `n` palabras.
    fn cast_ancho(&mut self, a: &Expr, n: usize, destino: &Tipo, pre: &mut Vec<Sent>) -> Vec<Expr> {
        let signo_origen = matches!(a.tipo, Tipo::Entero { signo: true, .. });
        let mut ws = if self.multi(&a.tipo).is_some() {
            self.pal(a, pre)
        } else {
            let v = self.expr(a, pre);
            vec![self.cast(v, self.tw())]
        };
        let _ = destino;
        ws.truncate(n);
        if ws.len() < n {
            let relleno = if signo_origen {
                let alta = ws.last().cloned().unwrap();
                let alta = self.simple(alta, pre);
                let f = self.temp(&self.tw());
                pre.push(self.asignar_local(f, self.cw(0)));
                let top = self.cw(1i128 << (self.w - 1));
                let cond = self.bin(B::MayorIgual, alta, top);
                let todo = self.cw(self.mascara());
                let poner = self.asignar_local(f, todo);
                pre.push(self.sent(ClaseSent::Si(cond, vec![poner], Vec::new())));
                self.leer_local(f)
            } else {
                self.cw(0)
            };
            while ws.len() < n {
                ws.push(relleno.clone());
            }
        }
        ws
    }

    /// Suma o resta con acarreo (o prestamo) entre palabras.
    fn sumar(&mut self, a: Vec<Expr>, b: Vec<Expr>, resta: bool, pre: &mut Vec<Sent>) -> Vec<Expr> {
        let n = a.len();
        let a: Vec<Expr> = a.into_iter().map(|x| self.simple(x, pre)).collect();
        let b: Vec<Expr> = b.into_iter().map(|x| self.simple(x, pre)).collect();
        let (op, op_cmp) = if resta {
            (B::Resta, B::Menor)
        } else {
            (B::Suma, B::Menor)
        };
        let mut res = Vec::new();
        let mut acarreo: Option<IdLocal> = None;
        for k in 0..n {
            let s = self.temp(&self.tw());
            pre.push(self.asignar_local(s, self.bin(op, a[k].clone(), b[k].clone())));
            let ultimo = k == n - 1;
            let mut sale: Option<IdLocal> = None;
            if !ultimo {
                let c = self.temp(&Tipo::Bool);
                // Suma: hay acarreo si el resultado quedo por debajo de un sumando.
                // Resta: hay prestamo si el minuendo es menor que el sustraendo.
                let primero = if resta {
                    self.bin(op_cmp, a[k].clone(), b[k].clone())
                } else {
                    self.bin(op_cmp, self.leer_local(s), a[k].clone())
                };
                pre.push(self.asignar_local(c, primero));
                sale = Some(c);
            }
            if let Some(entra) = acarreo {
                let entra_w = self.cast(self.leer_local(entra), self.tw());
                let doble = |t: &mut Self, c: IdLocal, pre: &mut Vec<Sent>| {
                    // Tambien hay acarreo/prestamo si al aplicar el de entrada se da la vuelta.
                    let cero = t.bin(B::Igual, t.leer_local(s), t.cw(0));
                    let y = t.bin(B::YLogico, t.leer_local(entra), cero);
                    let total = t.bin(B::OLogico, t.leer_local(c), y);
                    pre.push(t.asignar_local(c, total));
                };
                if resta {
                    // `s` aun es el valor antes de restar el prestamo.
                    if let Some(c) = sale {
                        doble(self, c, pre);
                    }
                    pre.push(self.asignar_local(s, self.bin(op, self.leer_local(s), entra_w)));
                } else {
                    pre.push(self.asignar_local(s, self.bin(op, self.leer_local(s), entra_w)));
                    if let Some(c) = sale {
                        doble(self, c, pre);
                    }
                }
            }
            res.push(self.leer_local(s));
            acarreo = sale;
        }
        res
    }

    /// Desplazamiento por una constante de bits.
    fn desplazar(
        &mut self,
        a: Vec<Expr>,
        c: u128,
        izquierda: bool,
        signo: bool,
        pre: &mut Vec<Sent>,
    ) -> Vec<Expr> {
        let n = a.len();
        let w = self.w as u128;
        if c >= w * n as u128 {
            // Todo se sale: queda cero, o el signo repetido.
            if !izquierda && signo {
                let alta = a[n - 1].clone();
                let r = self.relleno(alta, pre);
                return vec![r; n];
            }
            return self.consts(0, n);
        }
        let q = (c / w) as usize;
        let r = (c % w) as u32;
        let a: Vec<Expr> = a.into_iter().map(|x| self.simple(x, pre)).collect();
        let relleno = if !izquierda && signo {
            self.relleno(a[n - 1].clone(), pre)
        } else {
            self.cw(0)
        };
        let por = |s: &Self, e: Expr, op: B, bits: u32| s.bin(op, e, s.cw(i128::from(bits)));
        let mut res = Vec::new();
        for k in 0..n {
            let (principal, vecino): (Option<Expr>, Option<Expr>) = if izquierda {
                // La palabra k sale de a[k-q] desplazada y de los bits altos de a[k-q-1].
                let p = k.checked_sub(q).map(|j| a[j].clone());
                let v = k.checked_sub(q + 1).map(|j| a[j].clone());
                (p, v)
            } else {
                let j = k + q;
                let p = if j < n { Some(a[j].clone()) } else { None };
                let v = if j + 1 < n {
                    Some(a[j + 1].clone())
                } else {
                    None
                };
                (p, v)
            };
            let mut parte = None;
            if r == 0 {
                parte = Some(match principal {
                    Some(p) => p,
                    None => {
                        if izquierda {
                            self.cw(0)
                        } else {
                            relleno.clone()
                        }
                    }
                });
            } else if izquierda {
                if let Some(p) = principal {
                    parte = Some(por(self, p, B::DespIzq, r));
                }
                if let Some(v) = vecino {
                    let alto = por(self, v, B::DespDer, self.w - r);
                    parte = Some(match parte {
                        Some(x) => self.bin(B::O, x, alto),
                        None => alto,
                    });
                }
                if parte.is_none() {
                    parte = Some(self.cw(0));
                }
            } else {
                let j = k + q;
                // La palabra mas alta de un valor con signo se corre aritmeticamente.
                let baja = match principal {
                    Some(p) if signo && j == n - 1 => {
                        let con_signo = Tipo::Entero {
                            bits: self.w,
                            signo: true,
                        };
                        let pi = self.cast(p, con_signo.clone());
                        let sh = por(self, pi, B::DespDer, r);
                        self.cast(sh, self.tw())
                    }
                    Some(p) => por(self, p, B::DespDer, r),
                    None => {
                        if j == n && signo {
                            // Todo el valor son bits de signo.
                            relleno.clone()
                        } else {
                            self.cw(0)
                        }
                    }
                };
                let alta_fuente = match vecino {
                    Some(v) => Some(v),
                    None if signo && j < n => Some(relleno.clone()),
                    None => None,
                };
                parte = Some(match alta_fuente {
                    Some(v) => {
                        let alto = por(self, v, B::DespIzq, self.w - r);
                        self.bin(B::O, baja, alto)
                    }
                    None => baja,
                });
            }
            res.push(parte.unwrap());
        }
        // Los resultados dependen de varias palabras de origen: se guardan antes de escribirlos.
        res.into_iter()
            .map(|x| {
                if matches!(x.clase, ClaseExpr::Const(_)) {
                    x
                } else {
                    self.mat(x, pre)
                }
            })
            .collect()
    }

    /// Palabra con todos los bits iguales al signo de `alta`.
    fn relleno(&mut self, alta: Expr, pre: &mut Vec<Sent>) -> Expr {
        let alta = self.simple(alta, pre);
        let f = self.temp(&self.tw());
        pre.push(self.asignar_local(f, self.cw(0)));
        let top = self.cw(1i128 << (self.w - 1));
        let cond = self.bin(B::MayorIgual, alta, top);
        let poner = self.asignar_local(f, self.cw(self.mascara()));
        pre.push(self.sent(ClaseSent::Si(cond, vec![poner], Vec::new())));
        self.leer_local(f)
    }

    /// Operaciones que se resuelven llamando a una rutina de la biblioteca.
    fn rutina(&mut self, e: &Expr, op: B, a: &Expr, b: &Expr, pre: &mut Vec<Sent>) -> Vec<Expr> {
        let Tipo::Entero { bits, signo } = e.tipo else {
            return Vec::new();
        };
        let n = (bits / self.w) as usize;
        let sin = Tipo::Entero { bits, signo: false };
        let con = Tipo::Entero { bits, signo: true };
        let (nombre, tipo_a, tipo_r) = match (op, signo) {
            (B::Mul, _) => ("__mul", sin.clone(), sin.clone()),
            (B::Div, false) => ("__divu", sin.clone(), sin.clone()),
            (B::Mod, false) => ("__modu", sin.clone(), sin.clone()),
            (B::Div, true) => ("__divs", con.clone(), con.clone()),
            (B::Mod, true) => ("__mods", con.clone(), con.clone()),
            (B::DespIzq, _) => ("__shl", sin.clone(), sin.clone()),
            (B::DespDer, false) => ("__shr", sin.clone(), sin.clone()),
            (B::DespDer, true) => ("__sar", con.clone(), con.clone()),
            _ => {
                self.error(format!(
                    "`{}` no esta disponible para enteros de {bits} bits",
                    op.simbolo()
                ));
                return self.consts(0, n);
            }
        };
        let Some(&id) = self.ids.get(&format!("{nombre}{bits}")) else {
            self.error(format!("falta la rutina `{nombre}{bits}` de la biblioteca"));
            return self.consts(0, n);
        };
        let ca = self.cast(a.clone(), tipo_a.clone());
        let segundo = if matches!(op, B::DespIzq | B::DespDer) {
            sin.clone()
        } else {
            tipo_a
        };
        let cb = self.cast(b.clone(), segundo);
        let call = Expr {
            lugar: e.lugar,
            tipo: tipo_r,
            clase: ClaseExpr::Llamada(id, vec![ca, cb]),
        };
        self.pal(&call, pre)
    }

    /// Comparacion de dos enteros anchos, ya como palabras.
    fn comparar(
        &mut self,
        op: B,
        wa: Vec<Expr>,
        wb: Vec<Expr>,
        signo: bool,
        pre: &mut Vec<Sent>,
    ) -> Expr {
        let n = wa.len();
        let wa: Vec<Expr> = wa.into_iter().map(|x| self.simple(x, pre)).collect();
        let wb: Vec<Expr> = wb.into_iter().map(|x| self.simple(x, pre)).collect();
        match op {
            B::Igual | B::Distinto => {
                let (junta, cmp) = if op == B::Igual {
                    (B::YLogico, B::Igual)
                } else {
                    (B::OLogico, B::Distinto)
                };
                let mut r: Option<Expr> = None;
                for k in 0..n {
                    let c = self.bin(cmp, wa[k].clone(), wb[k].clone());
                    r = Some(match r {
                        None => c,
                        Some(p) => self.bin(junta, p, c),
                    });
                }
                r.unwrap_or_else(|| self.cbool(true))
            }
            _ => {
                let (x, y, estricto) = match op {
                    B::Menor => (&wa, &wb, true),
                    B::MenorIgual => (&wa, &wb, false),
                    B::Mayor => (&wb, &wa, true),
                    _ => (&wb, &wa, false),
                };
                let con_signo = Tipo::Entero {
                    bits: self.w,
                    signo: true,
                };
                let mut r: Option<Expr> = None;
                for k in 0..n {
                    let cmp_op = if k == 0 && !estricto {
                        B::MenorIgual
                    } else {
                        B::Menor
                    };
                    let lt = if k == n - 1 && signo {
                        let xs = self.cast(x[k].clone(), con_signo.clone());
                        let ys = self.cast(y[k].clone(), con_signo.clone());
                        self.bin(cmp_op, xs, ys)
                    } else {
                        self.bin(cmp_op, x[k].clone(), y[k].clone())
                    };
                    r = Some(match r {
                        None => lt,
                        Some(prev) => {
                            let iguales = self.bin(B::Igual, x[k].clone(), y[k].clone());
                            let y_bajo = self.bin(B::YLogico, iguales, prev);
                            self.bin(B::OLogico, lt, y_bajo)
                        }
                    });
                }
                r.unwrap()
            }
        }
    }
}
