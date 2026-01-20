//! Optimizaciones sobre la representacion intermedia (RF-SGT-033, RF-SGT-034).
//!
//! Tres niveles, cada uno incluye al anterior:
//! - 0: ninguna; el programa sale tal como se escribio.
//! - 1: plegado de constantes, ramas con condicion conocida y codigo muerto tras `devolver`,
//!   `romper` o `continuar`.
//! - 2: simplificaciones algebraicas (`x + 0`, `x * 1`, `x * 0`...) y sentencias sin efecto.
//! - 3: reduccion de fuerza (`x * 2^k` a desplazamiento, division y resto sin signo entre
//!   potencias de dos).
//!
//! Ninguna transformacion cambia lo que el programa hace: las operaciones que pueden detener la
//! maquina (dividir entre cero) o tocar un dispositivo (llamadas, lecturas de puerto) nunca se
//! eliminan ni se reordenan.

use crate::ast::OpBinario;
use crate::destino::Destino;
use crate::hir::{ClaseExpr, ClaseSent, ClaseSitio, Expr, Modulo, Sent, Sitio};
use crate::tipos::{envolver, Tipo};
use crate::valores;

/// Nivel maximo de optimizacion.
pub const NIVEL_MAXIMO: u8 = 3;

pub fn optimizar(m: &mut Modulo, d: &Destino, nivel: u8) {
    if nivel == 0 {
        return;
    }
    let o = Opt { d, nivel };
    for g in &mut m.globales {
        if let Some(e) = &mut g.inicial {
            o.expr(e);
        }
    }
    for f in &mut m.funciones {
        let cuerpo = std::mem::take(&mut f.cuerpo);
        f.cuerpo = o.bloque(cuerpo);
    }
}

struct Opt<'a> {
    d: &'a Destino,
    nivel: u8,
}

fn bits_y_signo(t: &Tipo, d: &Destino) -> (u32, bool) {
    match t {
        Tipo::Entero { bits, signo } => (*bits, *signo),
        Tipo::Puntero(_) => (d.direccion, false),
        _ => (1, false),
    }
}

fn constante(e: &Expr) -> Option<i128> {
    match e.clase {
        ClaseExpr::Const(v) => Some(v),
        _ => None,
    }
}

/// Una expresion sin efectos: se puede quitar o no evaluar sin que el programa lo note.
fn pura(e: &Expr) -> bool {
    match &e.clase {
        ClaseExpr::Const(_) | ClaseExpr::Bandera(_) => true,
        ClaseExpr::Leer(s) | ClaseExpr::Direccion(s) => sitio_puro(s),
        ClaseExpr::Binario(op, a, b) => {
            let divide = matches!(op, OpBinario::Div | OpBinario::Mod);
            // Dividir entre algo que no es una constante distinta de cero puede detener la maquina.
            let seguro = !divide || constante(b).is_some_and(|v| v != 0);
            seguro && pura(a) && pura(b)
        }
        ClaseExpr::Unario(_, a) | ClaseExpr::Cast(a) => pura(a),
        ClaseExpr::Llamada(..) | ClaseExpr::Entrada(_) => false,
    }
}

fn sitio_puro(s: &Sitio) -> bool {
    match &s.clase {
        ClaseSitio::Variable(_) | ClaseSitio::Registro(_) => true,
        ClaseSitio::Indice(b, i, _) => sitio_puro(b) && pura(i),
        ClaseSitio::Campo(b, _) => sitio_puro(b),
        ClaseSitio::Deref(p) => pura(p),
    }
}

fn potencia_de_dos(v: i128) -> Option<u32> {
    (v > 1 && v & (v - 1) == 0).then(|| v.trailing_zeros())
}

impl Opt<'_> {
    // ------------------------------------------------------------------
    // Sentencias
    // ------------------------------------------------------------------

    fn bloque(&self, b: Vec<Sent>) -> Vec<Sent> {
        let mut salida = Vec::new();
        for s in b {
            let termina = matches!(
                s.clase,
                ClaseSent::Devolver(_) | ClaseSent::Romper | ClaseSent::Continuar
            );
            self.sentencia(s, &mut salida);
            // Lo que sigue a una salida incondicional nunca se ejecuta.
            if termina || salida.last().is_some_and(acaba) {
                break;
            }
        }
        salida
    }

    fn sentencia(&self, s: Sent, fuera: &mut Vec<Sent>) {
        let lugar = s.lugar;
        let nueva = |clase| Sent { lugar, clase };
        match s.clase {
            ClaseSent::Asignar(mut sitio, mut v) => {
                self.sitio(&mut sitio);
                self.expr(&mut v);
                fuera.push(nueva(ClaseSent::Asignar(sitio, v)));
            }
            ClaseSent::Si(mut c, a, b) => {
                self.expr(&mut c);
                match constante(&c) {
                    Some(v) => {
                        let elegida = if v != 0 { a } else { b };
                        fuera.extend(self.bloque(elegida));
                    }
                    None => {
                        let a = self.bloque(a);
                        let b = self.bloque(b);
                        if a.is_empty() && b.is_empty() && pura(&c) && self.nivel >= 2 {
                            return;
                        }
                        fuera.push(nueva(ClaseSent::Si(c, a, b)));
                    }
                }
            }
            ClaseSent::Mientras(mut c, cuerpo) => {
                self.expr(&mut c);
                match constante(&c) {
                    Some(0) => {}
                    Some(_) => fuera.push(nueva(ClaseSent::Bucle(self.bloque(cuerpo)))),
                    None => fuera.push(nueva(ClaseSent::Mientras(c, self.bloque(cuerpo)))),
                }
            }
            ClaseSent::Bucle(cuerpo) => fuera.push(nueva(ClaseSent::Bucle(self.bloque(cuerpo)))),
            ClaseSent::Devolver(v) => {
                let v = v.map(|mut e| {
                    self.expr(&mut e);
                    e
                });
                fuera.push(nueva(ClaseSent::Devolver(v)));
            }
            ClaseSent::Segun {
                mut valor,
                brazos,
                otro,
            } => {
                self.expr(&mut valor);
                if let Some(v) = constante(&valor) {
                    let elegido = brazos
                        .into_iter()
                        .find(|(k, _)| *k == v)
                        .map(|(_, b)| b)
                        .unwrap_or(otro);
                    fuera.extend(self.bloque(elegido));
                    return;
                }
                let brazos = brazos
                    .into_iter()
                    .map(|(k, b)| (k, self.bloque(b)))
                    .collect();
                let otro = self.bloque(otro);
                fuera.push(nueva(ClaseSent::Segun {
                    valor,
                    brazos,
                    otro,
                }));
            }
            ClaseSent::Salida(mut p, mut v) => {
                self.expr(&mut p);
                self.expr(&mut v);
                fuera.push(nueva(ClaseSent::Salida(p, v)));
            }
            ClaseSent::Expr(mut e) => {
                self.expr(&mut e);
                if self.nivel >= 2 && pura(&e) {
                    return;
                }
                fuera.push(nueva(ClaseSent::Expr(e)));
            }
            otra @ (ClaseSent::Romper | ClaseSent::Continuar | ClaseSent::Asm(..)) => {
                fuera.push(nueva(otra));
            }
        }
    }

    fn sitio(&self, s: &mut Sitio) {
        match &mut s.clase {
            ClaseSitio::Variable(_) | ClaseSitio::Registro(_) => {}
            ClaseSitio::Indice(b, i, _) => {
                self.sitio(b);
                self.expr(i);
            }
            ClaseSitio::Campo(b, _) => self.sitio(b),
            ClaseSitio::Deref(p) => self.expr(p),
        }
    }

    // ------------------------------------------------------------------
    // Expresiones
    // ------------------------------------------------------------------

    fn expr(&self, e: &mut Expr) {
        match &mut e.clase {
            ClaseExpr::Const(_) | ClaseExpr::Bandera(_) => return,
            ClaseExpr::Leer(s) | ClaseExpr::Direccion(s) => {
                self.sitio(s);
                return;
            }
            ClaseExpr::Binario(_, a, b) => {
                self.expr(a);
                self.expr(b);
            }
            ClaseExpr::Unario(_, a) | ClaseExpr::Cast(a) | ClaseExpr::Entrada(a) => self.expr(a),
            ClaseExpr::Llamada(_, args) => {
                args.iter_mut().for_each(|a| self.expr(a));
                return;
            }
        }
        if let Some(n) = self.simplificar(e) {
            *e = n;
        }
    }

    fn constante(&self, e: &Expr, v: i128) -> Expr {
        Expr {
            lugar: e.lugar,
            tipo: e.tipo.clone(),
            clase: ClaseExpr::Const(v),
        }
    }

    fn simplificar(&self, e: &Expr) -> Option<Expr> {
        match &e.clase {
            ClaseExpr::Unario(op, a) => {
                let x = constante(a)?;
                let (bits, signo) = bits_y_signo(&a.tipo, self.d);
                Some(self.constante(e, valores::unario(*op, x, bits, signo)))
            }
            ClaseExpr::Cast(a) => {
                let x = constante(a)?;
                let (bits, signo) = bits_y_signo(&e.tipo, self.d);
                Some(self.constante(e, envolver(x, bits, signo)))
            }
            ClaseExpr::Binario(op, a, b) => self.binario(e, *op, a, b),
            _ => None,
        }
    }

    fn binario(&self, e: &Expr, op: OpBinario, a: &Expr, b: &Expr) -> Option<Expr> {
        // Los operadores perezosos deciden con solo el primer operando.
        match (op, constante(a)) {
            (OpBinario::YLogico, Some(0)) => return Some(self.constante(e, 0)),
            (OpBinario::OLogico, Some(v)) if v != 0 => return Some(self.constante(e, 1)),
            (OpBinario::YLogico, Some(_)) | (OpBinario::OLogico, Some(0)) => {
                return Some(b.clone())
            }
            _ => {}
        }
        if let (Some(x), Some(y)) = (constante(a), constante(b)) {
            if !matches!(a.tipo, Tipo::Entero { .. } | Tipo::Bool) {
                return None;
            }
            let (bits, signo) = bits_y_signo(&a.tipo, self.d);
            return valores::binario(op, x, y, bits, signo)
                .ok()
                .map(|v| self.constante(e, v));
        }
        if self.nivel < 2 || !matches!(e.tipo, Tipo::Entero { .. }) || e.tipo != a.tipo {
            return None;
        }
        let (_, signo) = bits_y_signo(&e.tipo, self.d);
        let cb = constante(b);
        let ca = constante(a);
        use OpBinario::*;
        match (op, ca, cb) {
            (Suma | Resta | O | Xor | DespIzq | DespDer, _, Some(0)) => return Some(a.clone()),
            (Suma | O | Xor, Some(0), _) if e.tipo == b.tipo => return Some(b.clone()),
            (Mul | Div, _, Some(1)) => return Some(a.clone()),
            (Mul, Some(1), _) if e.tipo == b.tipo => return Some(b.clone()),
            (Mul | Y, _, Some(0)) if pura(a) => return Some(self.constante(e, 0)),
            (Mul | Y, Some(0), _) if pura(b) => return Some(self.constante(e, 0)),
            _ => {}
        }
        if self.nivel >= 3 {
            if let Some(k) = cb.and_then(potencia_de_dos) {
                let desplazar = |o| {
                    let mut n = e.clone();
                    let mut c = b.clone();
                    c.clase = ClaseExpr::Const(i128::from(k));
                    n.clase = ClaseExpr::Binario(o, Box::new(a.clone()), Box::new(c));
                    Some(n)
                };
                match op {
                    Mul => return desplazar(DespIzq),
                    Div if !signo => return desplazar(DespDer),
                    Mod if !signo => {
                        let mut n = e.clone();
                        let mut c = b.clone();
                        c.clase = ClaseExpr::Const((1i128 << k) - 1);
                        n.clase = ClaseExpr::Binario(Y, Box::new(a.clone()), Box::new(c));
                        return Some(n);
                    }
                    _ => {}
                }
            }
        }
        None
    }
}

/// Una sentencia tras la cual el flujo no continua.
fn acaba(s: &Sent) -> bool {
    matches!(
        s.clase,
        ClaseSent::Devolver(_) | ClaseSent::Romper | ClaseSent::Continuar
    )
}
