//! Interprete de referencia: ejecuta el programa tipado sin pasar por la maquina destino.
//!
//! Sirve de oraculo. Un programa compilado y ejecutado en el emulador debe dar las mismas
//! salidas que aqui (RF-SGT-020, RF-SGT-021). Comparte con el compilador la aritmetica
//! ([`crate::valores`]) y el reparto de memoria ([`crate::layout`]).

use std::collections::{BTreeMap, HashMap, VecDeque};

use thiserror::Error;

use crate::ast::OpBinario;
use crate::destino::Destino;
use crate::hir::*;
use crate::layout::{disponer, palabras, Disposicion};
use crate::tipos::{envolver, Tipo};
use crate::valores;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErrorEjecucion {
    #[error("{0}")]
    Disposicion(String),
    #[error("division entre cero (linea {0})")]
    DivisionPorCero(u32),
    #[error("se supero el limite de {0} pasos")]
    LimitePasos(u64),
    #[error("el interprete no puede ejecutar {0}")]
    NoInterpretable(&'static str),
    #[error("falta la funcion `principal`")]
    SinPrincipal,
    #[error("indice {indice} fuera de un arreglo de {largo} elementos (linea {linea})")]
    FueraDeArreglo {
        indice: i128,
        largo: u64,
        linea: u32,
    },
}

/// Lo que el programa hizo hacia fuera.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resultado {
    /// Escrituras a puertos, en orden: (puerto, valor).
    pub salidas: Vec<(u64, u64)>,
    /// Pasos de sentencia ejecutados.
    pub pasos: u64,
    /// Valor final de cada global escalar, por nombre.
    pub globales: BTreeMap<String, i128>,
}

enum Flujo {
    Normal,
    Romper,
    Continuar,
    Devolver(Option<i128>),
}

struct Maq<'a> {
    m: &'a Modulo,
    d: &'a Destino,
    disp: Disposicion,
    memoria: HashMap<u64, u64>,
    registros: HashMap<String, i128>,
    entradas: HashMap<u64, VecDeque<u64>>,
    salidas: Vec<(u64, u64)>,
    pasos: u64,
    limite: u64,
}

pub fn ejecutar(
    m: &Modulo,
    d: &Destino,
    entradas: HashMap<u64, VecDeque<u64>>,
    limite_pasos: u64,
) -> Result<Resultado, ErrorEjecucion> {
    let disp = disponer(m, d).map_err(|e| ErrorEjecucion::Disposicion(e.to_string()))?;
    let principal = m
        .funciones
        .iter()
        .position(|f| f.nombre == crate::analisis::PRINCIPAL)
        .ok_or(ErrorEjecucion::SinPrincipal)?;
    let mut x = Maq {
        m,
        d,
        disp,
        memoria: HashMap::new(),
        registros: HashMap::new(),
        entradas,
        salidas: Vec::new(),
        pasos: 0,
        limite: limite_pasos,
    };
    // Valores iniciales de las globales, como hace el codigo de arranque.
    for (i, g) in m.globales.iter().enumerate() {
        if let Some(Expr {
            clase: ClaseExpr::Const(v),
            ..
        }) = &g.inicial
        {
            let dir = x.disp.globales[i];
            x.escribir(dir, &g.tipo, *v);
        }
    }
    x.llamar(principal, &[])?;
    let mut globales = BTreeMap::new();
    for (i, g) in m.globales.iter().enumerate() {
        if g.tipo.es_escalar() {
            let dir = x.disp.globales[i];
            globales.insert(g.nombre.clone(), x.leer(dir, &g.tipo));
        }
    }
    Ok(Resultado {
        salidas: x.salidas,
        pasos: x.pasos,
        globales,
    })
}

fn bits_y_signo(t: &Tipo, d: &Destino) -> (u32, bool) {
    match t {
        Tipo::Entero { bits, signo } => (*bits, *signo),
        Tipo::Puntero(_) => (d.direccion, false),
        _ => (1, false),
    }
}

impl Maq<'_> {
    fn leer_palabra(&self, a: u64) -> u64 {
        self.memoria.get(&a).copied().unwrap_or(0)
    }

    fn leer(&self, dir: u64, t: &Tipo) -> i128 {
        let (bits, signo) = bits_y_signo(t, self.d);
        let n = palabras(self.m, self.d, t);
        let mut v: i128 = 0;
        for k in 0..n {
            let idx = if self.d.little { k } else { n - 1 - k };
            let w = i128::from(self.leer_palabra(dir + k));
            v |= w << (self.d.palabra as u64 * idx);
        }
        if matches!(t, Tipo::Bool) {
            return i128::from(v != 0);
        }
        envolver(v, bits, signo)
    }

    fn escribir(&mut self, dir: u64, t: &Tipo, v: i128) {
        let n = palabras(self.m, self.d, t);
        let mascara = (1i128 << self.d.palabra) - 1;
        for k in 0..n {
            let idx = if self.d.little { k } else { n - 1 - k };
            let w = (v >> (self.d.palabra as u64 * idx)) & mascara;
            self.memoria.insert(dir + k, w as u64);
        }
    }

    fn paso(&mut self) -> Result<(), ErrorEjecucion> {
        self.pasos += 1;
        if self.pasos > self.limite {
            return Err(ErrorEjecucion::LimitePasos(self.limite));
        }
        Ok(())
    }

    fn llamar(&mut self, f: IdFuncion, args: &[i128]) -> Result<Option<i128>, ErrorEjecucion> {
        let fun = &self.m.funciones[f];
        for (p, v) in fun.parametros.iter().zip(args) {
            let dir = self.disp.locales[f][*p];
            let t = fun.locales[*p].tipo.clone();
            self.escribir(dir, &t, *v);
        }
        match self.bloque(f, &fun.cuerpo)? {
            Flujo::Devolver(v) => Ok(v),
            _ => Ok(None),
        }
    }

    fn bloque(&mut self, f: IdFuncion, b: &[Sent]) -> Result<Flujo, ErrorEjecucion> {
        for s in b {
            match self.sent(f, s)? {
                Flujo::Normal => {}
                otro => return Ok(otro),
            }
        }
        Ok(Flujo::Normal)
    }

    fn sent(&mut self, f: IdFuncion, s: &Sent) -> Result<Flujo, ErrorEjecucion> {
        self.paso()?;
        match &s.clase {
            ClaseSent::Asignar(sitio, e) => {
                let v = self.expr(f, e)?;
                self.guardar(f, sitio, v)?;
                Ok(Flujo::Normal)
            }
            ClaseSent::Si(c, a, b) => {
                if self.expr(f, c)? != 0 {
                    self.bloque(f, a)
                } else {
                    self.bloque(f, b)
                }
            }
            ClaseSent::Mientras(c, cuerpo) => {
                while self.expr(f, c)? != 0 {
                    match self.bloque(f, cuerpo)? {
                        Flujo::Romper => break,
                        Flujo::Devolver(v) => return Ok(Flujo::Devolver(v)),
                        _ => {}
                    }
                    self.paso()?;
                }
                Ok(Flujo::Normal)
            }
            ClaseSent::Bucle(cuerpo) => loop {
                match self.bloque(f, cuerpo)? {
                    Flujo::Romper => return Ok(Flujo::Normal),
                    Flujo::Devolver(v) => return Ok(Flujo::Devolver(v)),
                    _ => {}
                }
                self.paso()?;
            },
            ClaseSent::Romper => Ok(Flujo::Romper),
            ClaseSent::Continuar => Ok(Flujo::Continuar),
            ClaseSent::Devolver(v) => {
                let v = v.as_ref().map(|e| self.expr(f, e)).transpose()?;
                Ok(Flujo::Devolver(v))
            }
            ClaseSent::Segun {
                valor,
                brazos,
                otro,
            } => {
                let v = self.expr(f, valor)?;
                match brazos.iter().find(|(x, _)| *x == v) {
                    Some((_, b)) => self.bloque(f, b),
                    None => self.bloque(f, otro),
                }
            }
            ClaseSent::Asm(..) => Err(ErrorEjecucion::NoInterpretable("ensamblador en linea")),
            ClaseSent::Salida(p, v) => {
                let (p, v) = (self.expr(f, p)?, self.expr(f, v)?);
                self.salidas.push((p as u64, v as u64));
                Ok(Flujo::Normal)
            }
            ClaseSent::Expr(e) => {
                self.expr(f, e)?;
                Ok(Flujo::Normal)
            }
        }
    }

    fn direccion(&mut self, f: IdFuncion, s: &Sitio) -> Result<u64, ErrorEjecucion> {
        Ok(match &s.clase {
            ClaseSitio::Variable(Ref::Global(g)) => self.disp.globales[*g],
            ClaseSitio::Variable(Ref::Local(l)) => self.disp.locales[f][*l],
            ClaseSitio::Indice(b, i, tam) => {
                let base = self.direccion(f, b)?;
                let idx = self.expr(f, i)?;
                if let Tipo::Arreglo(_, n) = b.tipo {
                    if idx < 0 || idx as u64 >= n {
                        return Err(ErrorEjecucion::FueraDeArreglo {
                            indice: idx,
                            largo: n,
                            linea: i.lugar.linea,
                        });
                    }
                }
                base.wrapping_add((idx as u64).wrapping_mul(*tam))
            }
            ClaseSitio::Campo(b, desp) => self.direccion(f, b)? + desp,
            ClaseSitio::Deref(p) => self.expr(f, p)? as u64,
            ClaseSitio::Registro(_) => {
                return Err(ErrorEjecucion::NoInterpretable(
                    "la direccion de un registro",
                ))
            }
        })
    }

    fn guardar(&mut self, f: IdFuncion, s: &Sitio, v: i128) -> Result<(), ErrorEjecucion> {
        if let ClaseSitio::Registro(r) = &s.clase {
            let (bits, signo) = bits_y_signo(&s.tipo, self.d);
            self.registros.insert(r.clone(), envolver(v, bits, signo));
            return Ok(());
        }
        let dir = self.direccion(f, s)?;
        self.escribir(dir, &s.tipo, v);
        Ok(())
    }

    fn expr(&mut self, f: IdFuncion, e: &Expr) -> Result<i128, ErrorEjecucion> {
        Ok(match &e.clase {
            ClaseExpr::Const(v) => *v,
            ClaseExpr::Leer(s) => {
                if let ClaseSitio::Registro(r) = &s.clase {
                    return Ok(self.registros.get(r).copied().unwrap_or(0));
                }
                let dir = self.direccion(f, s)?;
                self.leer(dir, &s.tipo)
            }
            ClaseExpr::Direccion(s) => i128::from(self.direccion(f, s)?),
            ClaseExpr::Binario(op, a, b) => {
                let x = self.expr(f, a)?;
                // En `&&` y `||` el segundo operando solo se evalua si hace falta.
                match op {
                    OpBinario::YLogico if x == 0 => return Ok(0),
                    OpBinario::OLogico if x != 0 => return Ok(1),
                    _ => {}
                }
                let y = self.expr(f, b)?;
                let (bits, signo) = bits_y_signo(&a.tipo, self.d);
                let y_efectivo = if matches!(op, OpBinario::Suma | OpBinario::Resta)
                    && matches!(a.tipo, Tipo::Puntero(_))
                {
                    envolver(y, self.d.direccion, false)
                } else {
                    y
                };
                valores::binario(*op, x, y_efectivo, bits, signo)
                    .map_err(|_| ErrorEjecucion::DivisionPorCero(e.lugar.linea))?
            }
            ClaseExpr::Unario(op, a) => {
                let x = self.expr(f, a)?;
                let (bits, signo) = bits_y_signo(&a.tipo, self.d);
                valores::unario(*op, x, bits, signo)
            }
            ClaseExpr::Cast(a) => {
                let x = self.expr(f, a)?;
                let (bits, signo) = bits_y_signo(&e.tipo, self.d);
                envolver(x, bits, signo)
            }
            ClaseExpr::Llamada(id, args) => {
                let mut vs = Vec::new();
                for a in args {
                    vs.push(self.expr(f, a)?);
                }
                self.llamar(*id, &vs)?.unwrap_or(0)
            }
            ClaseExpr::Entrada(p) => {
                let p = self.expr(f, p)? as u64;
                i128::from(
                    self.entradas
                        .get_mut(&p)
                        .and_then(|q| q.pop_front())
                        .unwrap_or(0),
                )
            }
            ClaseExpr::Bandera(_) => {
                return Err(ErrorEjecucion::NoInterpretable(
                    "las banderas de la maquina",
                ))
            }
        })
    }
}
