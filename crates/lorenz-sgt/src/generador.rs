//! Generacion de ensamblador desde el programa tipado.
//!
//! Convencion de llamada (RF-SGT-027), la misma en toda maquina:
//! - Los argumentos se guardan en las posiciones fijas de los parametros de la funcion llamada.
//! - La direccion de retorno se guarda en una posicion fija por funcion.
//! - El valor devuelto vuelve en el primer registro auxiliar.
//! - Quien llama guarda los registros temporales vivos antes de la llamada y los repone despues.
//!
//! No hay pila ni marcos porque SGT no admite recursion (decision D-059); por eso tampoco hace
//! falta que la maquina tenga un puntero de pila (RF-SGT-026): el prologo y el epilogo son la
//! entrada de la funcion y el salto a la direccion de retorno guardada.

use std::collections::HashMap;

use lorenz_arch::modelo::Maquina;
use lorenz_arch::semantica::ast::{OpBinario as OpM, OpUnario as UnM};

use crate::analisis::PRINCIPAL;
use crate::ast::{OpBinario, OpUnario};
use crate::destino::Destino;
use crate::diagnostico::{Diagnostico, Lugar};
use crate::emisor::{Emisor, Falta, Linea, Sitio, Valor};
use crate::hir::*;
use crate::layout::{disponer, palabras, Disposicion};
use crate::tipos::Tipo;

pub struct Generado {
    pub lineas: Vec<Linea>,
    pub disposicion: Disposicion,
}

struct Gen<'a> {
    m: &'a Modulo,
    d: &'a Destino,
    em: Emisor<'a>,
    disp: Disposicion,
    diag: Vec<Diagnostico>,
    /// Registros temporales y los que estan libres ahora.
    temporales: Vec<String>,
    libres: Vec<String>,
    funcion: usize,
    lugar: Lugar,
    /// (continuar, romper) de los bucles abiertos.
    bucles: Vec<(String, String)>,
    ra: Vec<u64>,
    derrames: HashMap<(usize, String), u64>,
    fin_funcion: String,
}

type G<T> = Result<T, ()>;

fn op_maquina(op: OpBinario) -> Option<OpM> {
    Some(match op {
        OpBinario::Suma => OpM::Suma,
        OpBinario::Resta => OpM::Resta,
        OpBinario::Mul => OpM::Producto,
        OpBinario::Div => OpM::Division,
        OpBinario::Mod => OpM::Modulo,
        OpBinario::Y => OpM::Y,
        OpBinario::O => OpM::O,
        OpBinario::Xor => OpM::Xor,
        OpBinario::DespIzq => OpM::DesplazaIzq,
        OpBinario::DespDer => OpM::DesplazaDer,
        OpBinario::Igual => OpM::Igual,
        OpBinario::Distinto => OpM::Distinto,
        OpBinario::Menor => OpM::Menor,
        OpBinario::MenorIgual => OpM::MenorIgual,
        OpBinario::Mayor => OpM::Mayor,
        OpBinario::MayorIgual => OpM::MayorIgual,
        OpBinario::SumaSat | OpBinario::RestaSat | OpBinario::YLogico | OpBinario::OLogico => {
            return None
        }
    })
}

fn es_orden(op: OpBinario) -> bool {
    matches!(
        op,
        OpBinario::Menor | OpBinario::MenorIgual | OpBinario::Mayor | OpBinario::MayorIgual
    )
}

fn es_comparacion(op: OpBinario) -> bool {
    op.es_comparacion()
}

pub fn generar(
    m: &Modulo,
    d: &Destino,
    maq: &Maquina,
    saltos_largos: bool,
) -> Result<Generado, Vec<Diagnostico>> {
    let disp = disponer(m, d).map_err(|e| vec![e])?;
    let mut em = Emisor::nuevo(maq);
    em.saltos_largos = saltos_largos;
    let lugar0 = Lugar::nuevo(1, 1);
    if let Err(f) = em.preparar_registros() {
        return Err(vec![falta_a_diag(m, d, lugar0, &f)]);
    }
    // Registro base de los datos: solo si hay acceso con registro mas desplazamiento.
    let region = d
        .regiones
        .iter()
        .find(|r| r.nombre == disp.region)
        .map(|r| r.desde)
        .unwrap_or(0);
    let hay_base_mas_desp = em.perfil.instrucciones.iter().any(|i| {
        matches!(
            &i.forma,
            crate::seleccion::Forma::Leer {
                dir: crate::seleccion::Dir::Indirecta { desp: Some(_), .. },
                ..
            }
        )
    });
    let mut pool = em.pool.clone();
    if hay_base_mas_desp && pool.len() >= 6 {
        let gp = pool.pop().unwrap();
        em.gp = Some((gp, region));
    }
    let temporales: Vec<String> = pool[2..].to_vec();
    let mut g = Gen {
        m,
        d,
        em,
        disp,
        diag: Vec::new(),
        libres: temporales.clone(),
        temporales,
        funcion: 0,
        lugar: lugar0,
        bucles: Vec::new(),
        ra: Vec::new(),
        derrames: HashMap::new(),
        fin_funcion: String::new(),
    };
    g.programa();
    if g.diag.is_empty() {
        Ok(Generado {
            lineas: g.em.lineas,
            disposicion: g.disp,
        })
    } else {
        g.diag.sort_by_key(|d| d.lugar);
        g.diag.dedup();
        Err(g.diag)
    }
}

fn falta_a_diag(m: &Modulo, d: &Destino, lugar: Lugar, f: &Falta) -> Diagnostico {
    Diagnostico::nuevo(
        &m.archivo,
        lugar,
        format!(
            "`{}` no puede ejecutar esto: le falta {}",
            d.nombre, f.operacion
        ),
    )
}

impl<'a> Gen<'a> {
    fn err(&mut self, f: Falta) {
        let lugar = self.lugar;
        self.diag.push(falta_a_diag(self.m, self.d, lugar, &f));
    }

    fn error(&mut self, mensaje: impl Into<String>) {
        let d = Diagnostico::nuevo(&self.m.archivo, self.lugar, mensaje);
        self.diag.push(d);
    }

    fn r<T>(&mut self, r: Result<T, Falta>) -> G<T> {
        r.map_err(|f| self.err(f))
    }

    // ------------------------------------------------------------------
    // Registros temporales
    // ------------------------------------------------------------------

    fn tmp(&mut self) -> G<String> {
        if self.libres.is_empty() {
            let n = self.temporales.len();
            self.error(format!(
                "la expresion necesita mas de {n} registros temporales y `{}` solo deja {n} (los demas los usa el compilador); simplificala con variables intermedias",
                self.d.nombre
            ));
            return Err(());
        }
        Ok(self.libres.remove(0))
    }

    fn liberar(&mut self, r: &str) {
        if self.temporales.iter().any(|t| t == r) && !self.libres.iter().any(|l| l == r) {
            self.libres.insert(0, r.to_string());
        }
    }

    fn vivos(&self) -> Vec<String> {
        self.temporales
            .iter()
            .filter(|t| !self.libres.contains(t))
            .cloned()
            .collect()
    }

    fn una_palabra(&mut self, t: &Tipo) -> G<()> {
        if palabras(self.m, self.d, t) > 1 {
            self.error(format!(
                "el tipo {t} ocupa {} palabras de {} bits en `{}` y esta version solo opera con valores de una palabra",
                palabras(self.m, self.d, t),
                self.d.palabra,
                self.d.nombre
            ));
            return Err(());
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Programa y funciones
    // ------------------------------------------------------------------

    fn programa(&mut self) {
        // Posiciones de direccion de retorno, una por funcion.
        for _ in &self.m.funciones {
            let a = self.disp.libre;
            self.disp.libre += 1;
            self.ra.push(a);
        }
        let arranque = self.d.arranque;
        self.em.directiva(format!("    .org {arranque:#x}"));
        self.em.poner_etiqueta("_inicio");
        self.inicio();
        let Some(principal) = self.m.funciones.iter().position(|f| f.nombre == PRINCIPAL) else {
            return;
        };
        // `principal` va primero, pegada al codigo de arranque.
        let mut orden: Vec<usize> = vec![principal];
        orden.extend((0..self.m.funciones.len()).filter(|i| *i != principal));
        for f in orden {
            self.funcion_(f);
        }
        if self.disp.libre > self.disp.limite + 1 {
            self.lugar = Lugar::nuevo(1, 1);
            self.error("los datos del programa no caben en la memoria de la maquina");
        }
    }

    fn inicio(&mut self) {
        self.lugar = Lugar::nuevo(1, 1);
        self.em.linea_fuente = None;
        let gp = self.em.gp.clone();
        if let Some((gp, origen)) = gp {
            let r = self.em.cargar(&gp, &Valor::Num(i128::from(origen)));
            let _ = self.r(r);
        }
        // Las globales sin valor inicial empiezan en cero.
        let autos: Vec<(u64, u64)> = self
            .m
            .globales
            .iter()
            .enumerate()
            .filter(|(_, g)| g.direccion.is_none())
            .map(|(i, g)| {
                (
                    self.disp.globales[i],
                    palabras(self.m, self.d, &g.tipo).max(1),
                )
            })
            .collect();
        if let (Some(desde), Some(hasta)) = (
            autos.iter().map(|(a, _)| *a).min(),
            autos.iter().map(|(a, n)| a + n).max(),
        ) {
            let r = self.limpiar(desde, hasta);
            let _ = self.r(r);
        }
        for (i, g) in self.m.globales.iter().enumerate() {
            if let Some(Expr {
                clase: ClaseExpr::Const(v),
                ..
            }) = &g.inicial
            {
                let dir = self.disp.globales[i];
                if self.una_palabra(&g.tipo).is_err() {
                    continue;
                }
                let t = match self.tmp() {
                    Ok(t) => t,
                    Err(()) => continue,
                };
                let r = self.em.cargar(&t, &Valor::Num(*v));
                let _ = self.r(r);
                let r = self.em.escribir(&Sitio::Fija(dir), &t);
                let _ = self.r(r);
                self.liberar(&t);
            }
        }
    }

    fn limpiar(&mut self, desde: u64, hasta: u64) -> Result<(), Falta> {
        if hasta - desde <= 3 {
            let z = self.libres[0].clone();
            self.em.cargar(&z, &Valor::Num(0))?;
            for a in desde..hasta {
                self.em.escribir(&Sitio::Fija(a), &z)?;
            }
            return Ok(());
        }
        let (p, fin, z) = (
            self.libres[0].clone(),
            self.libres[1].clone(),
            self.libres[2].clone(),
        );
        self.em.cargar(&p, &Valor::Num(i128::from(desde)))?;
        self.em.cargar(&fin, &Valor::Num(i128::from(hasta)))?;
        self.em.cargar(&z, &Valor::Num(0))?;
        let bucle = self.em.etiqueta_nueva("z");
        self.em.poner_etiqueta(&bucle);
        self.em.escribir(&Sitio::EnReg(p.clone()), &z)?;
        self.em.sumar_const(&p, 1)?;
        self.em.saltar_si(OpM::Distinto, &p, &fin, true, &bucle)
    }

    fn nombre_funcion(&self, f: usize) -> String {
        format!("f_{}", self.m.funciones[f].nombre)
    }

    fn funcion_(&mut self, f: usize) {
        self.funcion = f;
        let fun = &self.m.funciones[f];
        self.lugar = fun.lugar;
        self.em.linea_fuente = Some(fun.lugar.linea);
        self.libres = self.temporales.clone();
        self.bucles.clear();
        if fun.interrupcion.is_some() {
            self.error("los manejadores de interrupcion todavia no se compilan");
            return;
        }
        let nombre = self.nombre_funcion(f);
        self.fin_funcion = format!("{nombre}__fin");
        self.em.poner_etiqueta(&nombre);
        let cuerpo = &fun.cuerpo;
        self.bloque(cuerpo);
        self.em.linea_fuente = Some(fun.lugar.linea);
        let fin = self.fin_funcion.clone();
        self.em.poner_etiqueta(&fin);
        if fun.nombre == PRINCIPAL {
            let r = self.em.detener();
            let _ = self.r(r);
        } else {
            let s1 = self.em.s1();
            let ra = self.ra[f];
            let r = self.em.leer(&s1, &Sitio::Fija(ra));
            let _ = self.r(r);
            let r = self.em.saltar_reg(&s1);
            let _ = self.r(r);
        }
    }

    // ------------------------------------------------------------------
    // Sentencias
    // ------------------------------------------------------------------

    fn bloque(&mut self, b: &[Sent]) {
        for s in b {
            self.lugar = s.lugar;
            self.em.linea_fuente = Some(s.lugar.linea);
            self.libres = self.temporales.clone();
            let _ = self.sentencia(s);
        }
    }

    fn sentencia(&mut self, s: &Sent) -> G<()> {
        match &s.clase {
            ClaseSent::Asignar(sitio, e) => {
                self.una_palabra(&sitio.tipo)?;
                if let ClaseSitio::Registro(r) = &sitio.clase {
                    let v = self.expr(e)?.ok_or(())?;
                    let r2 = self.em.mover(r, &v);
                    return self.r(r2);
                }
                let v = self.expr(e)?.ok_or(())?;
                let lugar = self.sitio(sitio)?;
                let r = self.em.escribir(&lugar, &v);
                self.r(r)?;
                Ok(())
            }
            ClaseSent::Si(c, a, b) => {
                let sino = self.em.etiqueta_nueva("s");
                let fin = self.em.etiqueta_nueva("f");
                self.rama(c, &sino, false)?;
                self.bloque(a);
                if !b.is_empty() {
                    let r = self.em.saltar(&fin);
                    self.r(r)?;
                }
                self.em.poner_etiqueta(&sino);
                if !b.is_empty() {
                    self.bloque(b);
                    self.em.poner_etiqueta(&fin);
                }
                Ok(())
            }
            ClaseSent::Mientras(c, cuerpo) => {
                let (ini, fin) = (self.em.etiqueta_nueva("m"), self.em.etiqueta_nueva("e"));
                self.em.poner_etiqueta(&ini);
                self.rama(c, &fin, false)?;
                self.bucles.push((ini.clone(), fin.clone()));
                self.bloque(cuerpo);
                self.bucles.pop();
                self.em.linea_fuente = Some(s.lugar.linea);
                let r = self.em.saltar(&ini);
                self.r(r)?;
                self.em.poner_etiqueta(&fin);
                Ok(())
            }
            ClaseSent::Bucle(cuerpo) => {
                let (ini, fin) = (self.em.etiqueta_nueva("b"), self.em.etiqueta_nueva("e"));
                self.em.poner_etiqueta(&ini);
                self.bucles.push((ini.clone(), fin.clone()));
                self.bloque(cuerpo);
                self.bucles.pop();
                let r = self.em.saltar(&ini);
                self.r(r)?;
                self.em.poner_etiqueta(&fin);
                Ok(())
            }
            ClaseSent::Romper => {
                let (_, fin) = self.bucles.last().cloned().ok_or(())?;
                let r = self.em.saltar(&fin);
                self.r(r)
            }
            ClaseSent::Continuar => {
                let (ini, _) = self.bucles.last().cloned().ok_or(())?;
                let r = self.em.saltar(&ini);
                self.r(r)
            }
            ClaseSent::Devolver(v) => {
                if let Some(e) = v {
                    let t = self.expr(e)?.ok_or(())?;
                    let s0 = self.em.s0();
                    let r = self.em.mover(&s0, &t);
                    self.r(r)?;
                }
                let fin = self.fin_funcion.clone();
                let r = self.em.saltar(&fin);
                self.r(r)
            }
            ClaseSent::Segun {
                valor,
                brazos,
                otro,
            } => {
                self.una_palabra(&valor.tipo)?;
                let fin = self.em.etiqueta_nueva("g");
                let v = self.expr(valor)?.ok_or(())?;
                let mut etiquetas = Vec::new();
                for (k, _) in brazos {
                    let et = self.em.etiqueta_nueva("a");
                    let tk = self.tmp()?;
                    let r = self.em.cargar(&tk, &Valor::Num(*k));
                    self.r(r)?;
                    let r = self.em.saltar_si(OpM::Igual, &v, &tk, true, &et);
                    self.r(r)?;
                    self.liberar(&tk);
                    etiquetas.push(et);
                }
                self.liberar(&v);
                // Sin coincidencia: el cuerpo de `otro` y a la salida.
                self.bloque(otro);
                self.lugar = s.lugar;
                let r = self.em.saltar(&fin);
                self.r(r)?;
                for (et, (_, cuerpo)) in etiquetas.iter().zip(brazos) {
                    self.em.poner_etiqueta(et);
                    self.bloque(cuerpo);
                    self.lugar = s.lugar;
                    let r = self.em.saltar(&fin);
                    self.r(r)?;
                }
                self.em.poner_etiqueta(&fin);
                Ok(())
            }
            ClaseSent::Asm(..) => {
                self.error("el ensamblador en linea todavia no se compila");
                Err(())
            }
            ClaseSent::Salida(p, v) => self.salida(p, v),
            ClaseSent::Expr(e) => {
                if let Some(t) = self.expr(e)? {
                    self.liberar(&t);
                }
                Ok(())
            }
        }
    }

    fn salida(&mut self, _p: &Expr, _v: &Expr) -> G<()> {
        self.error("las instrucciones de salida por puerto todavia no se compilan");
        Err(())
    }

    // ------------------------------------------------------------------
    // Lugares
    // ------------------------------------------------------------------

    fn sitio(&mut self, s: &crate::hir::Sitio) -> G<Sitio> {
        match &s.clase {
            ClaseSitio::Variable(Ref::Global(g)) => Ok(Sitio::Fija(self.disp.globales[*g])),
            ClaseSitio::Variable(Ref::Local(l)) => {
                Ok(Sitio::Fija(self.disp.locales[self.funcion][*l]))
            }
            ClaseSitio::Campo(b, desp) => match self.sitio(b)? {
                Sitio::Fija(a) => Ok(Sitio::Fija(a + desp)),
                Sitio::EnReg(r) => {
                    if *desp != 0 {
                        let x = self.em.sumar_const(&r, *desp as i128);
                        self.r(x)?;
                    }
                    Ok(Sitio::EnReg(r))
                }
            },
            ClaseSitio::Indice(b, i, tam) => {
                let base = self.sitio(b)?;
                if let ClaseExpr::Const(c) = i.clase {
                    let desp = c as u64 * tam;
                    return match base {
                        Sitio::Fija(a) => Ok(Sitio::Fija(a + desp)),
                        Sitio::EnReg(r) => {
                            if desp != 0 {
                                let x = self.em.sumar_const(&r, desp as i128);
                                self.r(x)?;
                            }
                            Ok(Sitio::EnReg(r))
                        }
                    };
                }
                let idx = self.expr(i)?.ok_or(())?;
                let x = self.multiplicar_const(&idx, *tam);
                self.r(x)?;
                match base {
                    Sitio::Fija(a) => {
                        let x = self.em.sumar_const(&idx, a as i128);
                        self.r(x)?;
                    }
                    Sitio::EnReg(r) => {
                        let x = self.em.alu_rr(OpM::Suma, &idx, &idx, &r);
                        self.r(x)?;
                        self.liberar(&r);
                    }
                }
                Ok(Sitio::EnReg(idx))
            }
            ClaseSitio::Deref(p) => {
                let r = self.expr(p)?.ok_or(())?;
                Ok(Sitio::EnReg(r))
            }
            ClaseSitio::Registro(_) => {
                self.error("leer o escribir registros de la maquina todavia no se compila");
                Err(())
            }
        }
    }

    /// `r = r * n` sin instruccion de multiplicar: suma de desplazamientos.
    fn multiplicar_const(&mut self, r: &str, n: u64) -> Result<(), Falta> {
        match n {
            0 => self.em.cargar(r, &Valor::Num(0)),
            1 => Ok(()),
            _ if n.is_power_of_two() => {
                for _ in 0..n.trailing_zeros() {
                    self.em.duplicar(r)?;
                }
                Ok(())
            }
            _ => {
                let acc = self.em.s0();
                let mut hay = false;
                let mut resto = n;
                while resto > 0 {
                    if resto & 1 == 1 {
                        if hay {
                            self.em.alu_rr(OpM::Suma, &acc, &acc, r)?;
                        } else {
                            self.em.mover(&acc, r)?;
                            hay = true;
                        }
                    }
                    resto >>= 1;
                    if resto > 0 {
                        self.em.duplicar(r)?;
                    }
                }
                self.em.mover(r, &acc)
            }
        }
    }

    // ------------------------------------------------------------------
    // Expresiones
    // ------------------------------------------------------------------

    /// Evalua `e` y devuelve el registro temporal con el valor (o `None` si no tiene valor).
    fn expr(&mut self, e: &Expr) -> G<Option<String>> {
        if e.tipo != Tipo::Vacio {
            self.una_palabra(&e.tipo)?;
        }
        match &e.clase {
            ClaseExpr::Const(v) => {
                let t = self.tmp()?;
                let r = self.em.cargar(&t, &Valor::Num(*v));
                self.r(r)?;
                Ok(Some(t))
            }
            ClaseExpr::Leer(s) => {
                if let ClaseSitio::Registro(nombre) = &s.clase {
                    let t = self.tmp()?;
                    let r = self.em.mover(&t, nombre);
                    self.r(r)?;
                    return Ok(Some(t));
                }
                let lugar = self.sitio(s)?;
                let t = match &lugar {
                    Sitio::EnReg(r) => r.clone(),
                    Sitio::Fija(_) => self.tmp()?,
                };
                let r = self.em.leer(&t, &lugar);
                self.r(r)?;
                Ok(Some(t))
            }
            ClaseExpr::Direccion(s) => match self.sitio(s)? {
                Sitio::EnReg(r) => Ok(Some(r)),
                Sitio::Fija(a) => {
                    let t = self.tmp()?;
                    let r = self.em.cargar(&t, &Valor::Num(i128::from(a)));
                    self.r(r)?;
                    Ok(Some(t))
                }
            },
            ClaseExpr::Unario(op, a) => self.unario(*op, a),
            ClaseExpr::Binario(op, a, b) => self.binario(e, *op, a, b),
            ClaseExpr::Cast(a) => {
                let a = self.expr(a)?.ok_or(())?;
                Ok(Some(a))
            }
            ClaseExpr::Llamada(id, args) => self.llamada(*id, args),
            ClaseExpr::Entrada(_) => {
                self.error("las instrucciones de entrada por puerto todavia no se compilan");
                Err(())
            }
            ClaseExpr::Bandera(n) => {
                self.error(format!("leer la bandera `{n}` todavia no se compila"));
                Err(())
            }
        }
    }

    fn unario(&mut self, op: OpUnario, a: &Expr) -> G<Option<String>> {
        match op {
            OpUnario::Negar => {
                let t = self.expr(a)?.ok_or(())?;
                let r = self.em.unaria(UnM::Menos, &t, &t);
                self.r(r)?;
                Ok(Some(t))
            }
            OpUnario::Complemento => {
                let t = self.expr(a)?.ok_or(())?;
                let r = self.em.unaria(UnM::NegacionBits, &t, &t);
                self.r(r)?;
                Ok(Some(t))
            }
            OpUnario::No => self.bool_por_rama_de(&Expr {
                lugar: a.lugar,
                tipo: Tipo::Bool,
                clase: ClaseExpr::Unario(OpUnario::No, Box::new(a.clone())),
            }),
        }
    }

    fn binario(&mut self, e: &Expr, op: OpBinario, a: &Expr, b: &Expr) -> G<Option<String>> {
        if matches!(op, OpBinario::YLogico | OpBinario::OLogico) || es_comparacion(op) {
            // Atajo: `<` y `>` sin signo con una instruccion que ya da el booleano.
            let sin_signo = !matches!(a.tipo, Tipo::Entero { signo: true, .. });
            if sin_signo
                && matches!(op, OpBinario::Menor | OpBinario::Mayor)
                && self.em.tiene_alu_rr(OpM::Menor)
            {
                let ta = self.expr(a)?.ok_or(())?;
                let tb = self.expr(b)?.ok_or(())?;
                let (x, y) = if op == OpBinario::Menor {
                    (&ta, &tb)
                } else {
                    (&tb, &ta)
                };
                let r = self.em.alu_rr(OpM::Menor, &ta, x, y);
                self.r(r)?;
                self.liberar(&tb);
                return Ok(Some(ta));
            }
            return self.bool_por_rama_de(e);
        }
        let ta = self.expr(a)?.ok_or(())?;
        if let ClaseExpr::Const(c) = b.clase {
            if let Some(opm) = op_maquina(op) {
                if matches!(
                    op,
                    OpBinario::Suma
                        | OpBinario::Resta
                        | OpBinario::Y
                        | OpBinario::O
                        | OpBinario::Xor
                ) {
                    let r = self.em.alu_const(opm, &ta, &ta, c);
                    self.r(r)?;
                    return Ok(Some(ta));
                }
            }
        }
        let tb = self.expr(b)?.ok_or(())?;
        let Some(opm) = op_maquina(op) else {
            self.error(format!(
                "el operador `{}` todavia no se compila",
                op.simbolo()
            ));
            return Err(());
        };
        let r = self.em.alu_rr(opm, &ta, &ta, &tb);
        self.r(r)?;
        self.liberar(&tb);
        Ok(Some(ta))
    }

    /// Materializa una condicion como 0 o 1 usando saltos.
    fn bool_por_rama_de(&mut self, e: &Expr) -> G<Option<String>> {
        let res = self.tmp()?;
        let (falso, fin) = (self.em.etiqueta_nueva("v"), self.em.etiqueta_nueva("w"));
        self.rama(e, &falso, false)?;
        let r = self.em.cargar(&res, &Valor::Num(1));
        self.r(r)?;
        let r = self.em.saltar(&fin);
        self.r(r)?;
        self.em.poner_etiqueta(&falso);
        let r = self.em.cargar(&res, &Valor::Num(0));
        self.r(r)?;
        self.em.poner_etiqueta(&fin);
        Ok(Some(res))
    }

    /// Salta a `destino` si la condicion `e` vale `cuando`.
    fn rama(&mut self, e: &Expr, destino: &str, cuando: bool) -> G<()> {
        match &e.clase {
            ClaseExpr::Const(v) => {
                if (*v != 0) == cuando {
                    let r = self.em.saltar(destino);
                    return self.r(r);
                }
                Ok(())
            }
            ClaseExpr::Unario(OpUnario::No, x) => self.rama(x, destino, !cuando),
            ClaseExpr::Binario(OpBinario::YLogico, a, b) => {
                if !cuando {
                    self.rama(a, destino, false)?;
                    self.rama(b, destino, false)
                } else {
                    let salta = self.em.etiqueta_nueva("y");
                    self.rama(a, &salta, false)?;
                    self.rama(b, destino, true)?;
                    self.em.poner_etiqueta(&salta);
                    Ok(())
                }
            }
            ClaseExpr::Binario(OpBinario::OLogico, a, b) => {
                if cuando {
                    self.rama(a, destino, true)?;
                    self.rama(b, destino, true)
                } else {
                    let salta = self.em.etiqueta_nueva("o");
                    self.rama(a, &salta, true)?;
                    self.rama(b, destino, false)?;
                    self.em.poner_etiqueta(&salta);
                    Ok(())
                }
            }
            ClaseExpr::Binario(op, a, b) if es_comparacion(*op) => {
                self.una_palabra(&a.tipo)?;
                let con_signo = matches!(a.tipo, Tipo::Entero { signo: true, .. });
                let ta = self.expr(a)?.ok_or(())?;
                let tb = self.expr(b)?.ok_or(())?;
                if con_signo && es_orden(*op) {
                    // El orden de la maquina es sin signo: se invierte el bit de signo de ambos.
                    let signo = 1i128 << (self.d.palabra - 1);
                    for t in [&ta, &tb] {
                        let r = self.em.alu_const(OpM::Xor, t, t, signo);
                        self.r(r)?;
                    }
                }
                let opm = op_maquina(*op).unwrap();
                let r = self.em.saltar_si(opm, &ta, &tb, cuando, destino);
                self.r(r)?;
                self.liberar(&ta);
                self.liberar(&tb);
                Ok(())
            }
            _ => {
                let t = self.expr(e)?.ok_or(())?;
                let r = self.em.saltar_si_cero(&t, !cuando, destino);
                self.r(r)?;
                self.liberar(&t);
                Ok(())
            }
        }
    }

    // ------------------------------------------------------------------
    // Llamadas
    // ------------------------------------------------------------------

    fn derrame(&mut self, reg: &str) -> u64 {
        let clave = (self.funcion, reg.to_string());
        if let Some(a) = self.derrames.get(&clave) {
            return *a;
        }
        let a = self.disp.libre;
        self.disp.libre += 1;
        self.derrames.insert(clave, a);
        a
    }

    fn llamada(&mut self, id: usize, args: &[Expr]) -> G<Option<String>> {
        let fun = &self.m.funciones[id];
        // 1. Todos los argumentos, antes de tocar las posiciones de los parametros: un argumento
        //    puede ser otra llamada a la misma funcion.
        let mut valores = Vec::new();
        for a in args {
            valores.push(self.expr(a)?.ok_or(())?);
        }
        for ((p, v), a) in fun.parametros.iter().zip(&valores).zip(args) {
            let _ = a;
            let dir = self.disp.locales[id][*p];
            let r = self.em.escribir(&Sitio::Fija(dir), v);
            self.r(r)?;
        }
        for v in &valores {
            self.liberar(v);
        }
        // 2. El registro del resultado y los vivos.
        let devuelve = fun.retorno != Tipo::Vacio;
        let destino = if devuelve { Some(self.tmp()?) } else { None };
        let vivos: Vec<String> = self
            .vivos()
            .into_iter()
            .filter(|v| Some(v) != destino.as_ref())
            .collect();
        for v in &vivos {
            let a = self.derrame(v);
            let r = self.em.escribir(&Sitio::Fija(a), v);
            self.r(r)?;
        }
        // 3. Direccion de retorno y salto.
        let ret = self.em.etiqueta_nueva("r");
        let s0 = self.em.s0();
        let r = self.em.cargar(&s0, &Valor::Sim(ret.clone()));
        self.r(r)?;
        let ra = self.ra[id];
        let r = self.em.escribir(&Sitio::Fija(ra), &s0);
        self.r(r)?;
        let nombre = self.nombre_funcion(id);
        let r = self.em.saltar(&nombre);
        self.r(r)?;
        self.em.poner_etiqueta(&ret);
        // 4. Resultado y reposicion.
        if let Some(d) = &destino {
            let r = self.em.mover(d, &s0);
            self.r(r)?;
        }
        for v in &vivos {
            let a = self.derrame(v);
            let r = self.em.leer(v, &Sitio::Fija(a));
            self.r(r)?;
        }
        Ok(destino)
    }
}
