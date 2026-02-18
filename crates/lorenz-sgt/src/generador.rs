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

type Sustituye = Box<dyn Fn(OpBinario, &Tipo) -> Option<&'static str>>;

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
    sustituye: Sustituye,
    /// Se vuelve de las funciones por un numero de llamada y no por una direccion (la maquina no
    /// puede saltar a un registro, o la direccion no cabe en uno).
    por_indice: bool,
    /// Por funcion, las etiquetas de retorno de cada sitio desde el que se la llama.
    sitios: Vec<Vec<String>>,
}

type G<T> = Result<T, ()>;

/// Palabras que se ponen a cero una a una, con direccion absoluta, si la maquina no puede hacerlo
/// en un bucle.
const MAXIMO_SIN_BUCLE: u64 = 512;

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

/// Funcion de la biblioteca que sustituye a un operador que la maquina no tiene (RF-SGT-024).
///
/// Devuelve `None` cuando el operador se hace con una instruccion. Solo se contemplan enteros del
/// ancho de palabra.
pub fn sustitucion_para(maq: &Maquina) -> impl Fn(OpBinario, &Tipo) -> Option<&'static str> {
    let perfil = crate::seleccion::Perfil::de(maq);
    let palabra = maq.bits_palabra;
    move |op, t| {
        let Tipo::Entero { bits, signo } = t else {
            return None;
        };
        if *bits != palabra {
            return None;
        }
        let nativa = |o: OpM| crate::emisor::perfil_tiene_alu_rr(&perfil, o);
        Some(match (op, signo) {
            (OpBinario::Mul, _) => {
                if nativa(OpM::Producto) {
                    return None;
                }
                "__mul"
            }
            (OpBinario::Div, false) => {
                if nativa(OpM::Division) {
                    return None;
                }
                "__divu"
            }
            (OpBinario::Mod, false) => {
                if nativa(OpM::Modulo) {
                    return None;
                }
                "__modu"
            }
            (OpBinario::Div, true) => "__divs",
            (OpBinario::Mod, true) => "__mods",
            (OpBinario::DespIzq, _) => "__shl",
            (OpBinario::DespDer, false) => "__shr",
            (OpBinario::DespDer, true) => "__sar",
            (OpBinario::SumaSat, false) => "__sumasat_u",
            (OpBinario::RestaSat, false) => "__restasat_u",
            (OpBinario::SumaSat, true) => "__sumasat_s",
            (OpBinario::RestaSat, true) => "__restasat_s",
            _ => return None,
        })
    }
}

/// Reparte los registros del compilador: el de retorno, el auxiliar, los temporales y, si la
/// maquina direcciona con registro mas desplazamiento, el que apunta a los datos. Devuelve los
/// temporales.
pub fn repartir_registros(em: &mut Emisor, region: u64) -> Vec<String> {
    // Registro base de los datos: solo si hay acceso con registro mas desplazamiento.
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
    pool[2..].to_vec()
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
    let region = d
        .regiones
        .iter()
        .find(|r| r.nombre == disp.region)
        .map(|r| r.desde)
        .unwrap_or(0);
    let temporales = repartir_registros(&mut em, region);
    let g_em_salta_a_registro = em.salta_a_registro();
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
        sustituye: Box::new(sustitucion_para(maq)),
        por_indice: d.direccion > d.palabra || !g_em_salta_a_registro,
        sitios: vec![Vec::new(); m.funciones.len()],
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

    /// Un registro que el compilador usa para sus propios valores no se puede tocar a mano.
    fn registro_libre(&mut self, r: &str) -> G<()> {
        let propio =
            self.em.pool.iter().any(|p| p == r) || self.em.gp.as_ref().is_some_and(|(g, _)| g == r);
        if propio {
            self.error(format!(
                "el registro `{r}` lo usa el compilador para guardar valores temporales; en `{}` puedes usar los registros que no son de uso general o escribir ensamblador en linea",
                self.d.nombre
            ));
            return Err(());
        }
        Ok(())
    }

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
        let manejadores: Vec<(u32, usize)> = self
            .m
            .funciones
            .iter()
            .enumerate()
            .filter_map(|(i, f)| f.interrupcion.map(|l| (l, i)))
            .collect();
        self.em.directiva(format!("    .org {arranque:#x}"));
        self.em.poner_etiqueta("_inicio");
        if !manejadores.is_empty() {
            // El codigo corriente va despues de la tabla de vectores. Cada vector guarda la
            // direccion del manejador, que es lo que lee la maquina al interrumpir.
            let r = self.em.saltar("_arranque");
            let _ = self.r(r);
            let mut fin = arranque;
            for (linea, id) in &manejadores {
                if let Some(v) = self.d.vectores.get(linea) {
                    self.em.directiva(format!("    .org {v:#x}"));
                    let nombre = self.nombre_funcion(*id);
                    self.em.directiva(format!("    .dw {nombre}"));
                    fin = fin.max(*v);
                }
            }
            self.em.directiva(format!("    .org {:#x}", fin + 4));
            self.em.poner_etiqueta("_arranque");
        }
        self.inicio();
        if !manejadores.is_empty() && !self.d.interrupciones_al_arrancar {
            let r = self.em.interrupciones(true);
            let _ = self.r(r);
        }
        let Some(principal) = self.m.funciones.iter().position(|f| f.nombre == PRINCIPAL) else {
            return;
        };
        // `principal` va primero, pegada al codigo de arranque.
        let mut orden: Vec<usize> = vec![principal];
        orden.extend((0..self.m.funciones.len()).filter(|i| *i != principal));
        for f in orden {
            self.funcion_(f);
        }
        if self.por_indice {
            for f in 0..self.m.funciones.len() {
                let fun = &self.m.funciones[f];
                if fun.interrupcion.is_none() && fun.nombre != PRINCIPAL {
                    self.despacho(f);
                }
            }
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
            self.datos_iniciales(i, g);
            if let Some(Expr {
                clase: ClaseExpr::Const(v),
                ..
            }) = &g.inicial
            {
                let dir = self.disp.globales[i];
                let n = palabras(self.m, self.d, &g.tipo);
                if !matches!(g.tipo, Tipo::Entero { .. }) && self.una_palabra(&g.tipo).is_err() {
                    continue;
                }
                let t = match self.tmp() {
                    Ok(t) => t,
                    Err(()) => continue,
                };
                for (k, p) in self.palabras_de_valor(*v, n).into_iter().enumerate() {
                    let r = self.em.cargar(&t, &Valor::Num(p));
                    let _ = self.r(r);
                    let r = self.em.escribir(&Sitio::Fija(dir + k as u64), &t);
                    let _ = self.r(r);
                }
                self.liberar(&t);
            }
        }
    }

    /// Las `n` palabras de un valor, en el orden en que van en memoria.
    fn palabras_de_valor(&self, v: i128, n: u64) -> Vec<i128> {
        let w = self.d.palabra;
        let mascara = (1i128 << w) - 1;
        let mut ps: Vec<i128> = (0..n).map(|k| (v >> (k as u32 * w)) & mascara).collect();
        if !self.d.little {
            ps.reverse();
        }
        ps
    }

    /// Escribe el texto con que empieza un arreglo, un elemento por palabra.
    fn datos_iniciales(&mut self, global: usize, g: &crate::hir::Variable) {
        if g.datos.is_empty() {
            return;
        }
        let Tipo::Arreglo(elemento, _) = &g.tipo else {
            return;
        };
        if !matches!(**elemento, Tipo::Entero { .. }) && self.una_palabra(elemento).is_err() {
            return;
        }
        let n = palabras(self.m, self.d, elemento);
        let base = self.disp.globales[global];
        let Ok(t) = self.tmp() else { return };
        for (k, v) in g.datos.iter().enumerate() {
            for (j, p) in self.palabras_de_valor(*v, n).into_iter().enumerate() {
                let r = self.em.cargar(&t, &Valor::Num(p));
                let _ = self.r(r);
                let r = self
                    .em
                    .escribir(&Sitio::Fija(base + k as u64 * n + j as u64), &t);
                let _ = self.r(r);
            }
        }
        self.liberar(&t);
    }

    fn limpiar(&mut self, desde: u64, hasta: u64) -> Result<(), Falta> {
        // Un bucle con un puntero necesita escribir por registro y que la direccion quepa en uno;
        // si no, se escribe cada palabra con su direccion absoluta.
        let con_bucle = self.d.direccion <= self.d.palabra && self.em.escribe_por_registro();
        if hasta - desde <= 3 || !con_bucle {
            if hasta - desde > MAXIMO_SIN_BUCLE {
                return Err(Falta::nueva(format!(
                    "poner a cero {} palabras de datos al arrancar sin escribir por registro de direccion (el maximo es {MAXIMO_SIN_BUCLE})",
                    hasta - desde
                )));
            }
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
        let nombre = self.nombre_funcion(f);
        self.fin_funcion = format!("{nombre}__fin");
        self.em.poner_etiqueta(&nombre);
        let manejador = fun.interrupcion.is_some();
        if manejador {
            let r = self.guardar_registros(true);
            let _ = self.r(r);
        }
        let cuerpo = &fun.cuerpo;
        self.bloque(cuerpo);
        self.em.linea_fuente = Some(fun.lugar.linea);
        let fin = self.fin_funcion.clone();
        self.em.poner_etiqueta(&fin);
        if manejador {
            let r = self.guardar_registros(false);
            let _ = self.r(r);
            let r = self.em.retorno_de_interrupcion();
            let _ = self.r(r);
        } else if fun.nombre == PRINCIPAL {
            let r = self.em.detener();
            let _ = self.r(r);
        } else {
            if self.por_indice {
                let vuelta = format!("{nombre}__vuelta");
                let r = self.em.saltar(&vuelta);
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
    }

    /// Vuelta de una funcion por numero de llamada: lee el numero que dejo quien llamo y salta a
    /// la etiqueta de ese sitio. Se escribe al final, cuando ya se conocen todos los sitios.
    fn despacho(&mut self, f: usize) {
        let sitios = self.sitios[f].clone();
        let nombre = self.nombre_funcion(f);
        self.em.linea_fuente = Some(self.m.funciones[f].lugar.linea);
        self.em.poner_etiqueta(&format!("{nombre}__vuelta"));
        let Some((ultimo, resto)) = sitios.split_last() else {
            // Nadie la llama: no hay a donde volver.
            let r = self.em.detener();
            let _ = self.r(r);
            return;
        };
        if !resto.is_empty() {
            // El primer registro lleva el valor que devuelve la funcion y no se toca: el numero de
            // llamada va en el auxiliar y se compara con un temporal.
            let s1 = self.em.s1();
            let Some(t) = self.temporales.first().cloned() else {
                self.error("faltan registros temporales para volver de las funciones");
                return;
            };
            let ra = self.ra[f];
            let r = self.em.leer(&s1, &Sitio::Fija(ra));
            let _ = self.r(r);
            for (k, etiqueta) in resto.iter().enumerate() {
                let r = self.em.cargar(&t, &Valor::Num(k as i128 + 1));
                let _ = self.r(r);
                let r = self.em.saltar_si(OpM::Igual, &s1, &t, true, etiqueta);
                let _ = self.r(r);
            }
        }
        let r = self.em.saltar(ultimo);
        let _ = self.r(r);
    }

    /// Guarda (o repone) todos los registros del compilador: un manejador puede interrumpir al
    /// programa en cualquier punto.
    fn guardar_registros(&mut self, guardar: bool) -> Result<(), Falta> {
        let base = self
            .disp
            .salvaguarda
            .expect("hay manejadores y falta el area de registros");
        let mut regs = self.em.pool.clone();
        if let Some((gp, _)) = &self.em.gp {
            regs.push(gp.clone());
        }
        for (k, r) in regs.iter().enumerate() {
            let sitio = Sitio::Fija(base + k as u64);
            if guardar {
                self.em.escribir(&sitio, r)?;
            } else {
                self.em.leer(r, &sitio)?;
            }
        }
        Ok(())
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
                    self.registro_libre(r)?;
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
            ClaseSent::Asm(texto, refs) => self.asm(texto, refs),
            ClaseSent::Salida(p, v) => self.salida(p, v),
            ClaseSent::Expr(e) => {
                if let Some(t) = self.expr(e)? {
                    self.liberar(&t);
                }
                Ok(())
            }
        }
    }

    /// Ensamblador en linea: cada `{nombre}` se sustituye por la direccion de esa variable. El
    /// texto puede usar cualquier registro de la maquina salvo el contador de programa y el
    /// puntero de pila; el compilador repone su registro base de datos al terminar.
    fn asm(&mut self, texto: &str, refs: &[(String, Ref)]) -> G<()> {
        let mut t = texto.to_string();
        for (n, r) in refs {
            let dir = match r {
                Ref::Global(g) => self.disp.globales[*g],
                Ref::Local(l) => self.disp.locales[self.funcion][*l],
            };
            t = t.replacen(&format!("{{{n}}}"), &format!("{dir:#x}"), 1);
        }
        for l in t.lines().filter(|l| !l.trim().is_empty()) {
            self.em.linea_literal(l);
        }
        let gp = self.em.gp.clone();
        if let Some((gp, origen)) = gp {
            let r = self.em.cargar(&gp, &Valor::Num(i128::from(origen)));
            self.r(r)?;
        }
        Ok(())
    }

    fn salida(&mut self, p: &Expr, v: &Expr) -> G<()> {
        let ClaseExpr::Const(puerto) = p.clase else {
            self.error("el puerto debe ser una constante");
            return Err(());
        };
        let t = self.expr(v)?.ok_or(())?;
        let r = self.em.salida(puerto as u64, &t);
        self.r(r)
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
                    self.registro_libre(nombre)?;
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
            ClaseExpr::Entrada(p) => {
                let ClaseExpr::Const(puerto) = p.clase else {
                    self.error("el puerto debe ser una constante");
                    return Err(());
                };
                let t = self.tmp()?;
                let r = self.em.entrada(&t, puerto as u64);
                self.r(r)?;
                Ok(Some(t))
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
        if let Some(nombre) = (self.sustituye)(op, &a.tipo) {
            let raiz = self.m.funciones[self.funcion].raiz;
            let id = self
                .m
                .funciones
                .iter()
                .position(|f| f.base == nombre && f.raiz == raiz)
                .expect("falta la biblioteca de apoyo");
            return self.llamar_con(id, &[ta, tb]);
        }
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
        // Un argumento puede ser otra llamada a la misma funcion, que pisaria los parametros ya
        // escritos: hasta el ultimo argumento que contiene una llamada se calculan todos antes de
        // tocar las posiciones de los parametros. Los de despues no pueden pisar nada, asi que se
        // guardan al calcularlos y no ocupan registros mientras se calculan los demas (un entero
        // ancho pasa varias palabras).
        let retener = args
            .iter()
            .rposition(puede_llamar)
            .map_or(0, |ultimo| ultimo + 1);
        let mut valores = Vec::new();
        for a in &args[..retener] {
            valores.push(self.expr(a)?.ok_or(())?);
        }
        for (k, a) in args.iter().enumerate().skip(retener) {
            let v = self.expr(a)?.ok_or(())?;
            let p = self.m.funciones[id].parametros[k];
            let dir = self.disp.locales[id][p];
            let r = self.em.escribir(&Sitio::Fija(dir), &v);
            self.r(r)?;
            self.liberar(&v);
        }
        self.llamar_con(id, &valores)
    }

    /// Llama a la funcion `id` con los argumentos ya calculados en registros. Los registros de
    /// los argumentos se liberan aqui.
    fn llamar_con(&mut self, id: usize, valores: &[String]) -> G<Option<String>> {
        let fun = &self.m.funciones[id];
        if fun.nombre == "__detener" {
            let r = self.em.detener();
            self.r(r)?;
            return Ok(None);
        }
        for (p, v) in fun.parametros.iter().zip(valores) {
            let dir = self.disp.locales[id][*p];
            let r = self.em.escribir(&Sitio::Fija(dir), v);
            self.r(r)?;
        }
        // Los argumentos ya estan guardados: sus registros se pueden reusar.
        let propios: Vec<String> = valores.to_vec();
        for v in &propios {
            self.liberar(v);
        }
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
        let ret = self.em.etiqueta_nueva("r");
        let s0 = self.em.s0();
        if self.por_indice {
            self.sitios[id].push(ret.clone());
            let numero = self.sitios[id].len() as i128;
            if numero >= 1i128 << self.d.palabra {
                self.error(format!(
                    "`{}` se llama desde {numero} sitios y el numero de llamada no cabe en una palabra de {} bits",
                    self.m.funciones[id].base, self.d.palabra
                ));
                return Err(());
            }
            let r = self.em.cargar(&s0, &Valor::Num(numero));
            self.r(r)?;
        } else {
            let r = self.em.cargar(&s0, &Valor::Sim(ret.clone()));
            self.r(r)?;
        }
        let ra = self.ra[id];
        let r = self.em.escribir(&Sitio::Fija(ra), &s0);
        self.r(r)?;
        let nombre = self.nombre_funcion(id);
        let r = self.em.saltar(&nombre);
        self.r(r)?;
        self.em.poner_etiqueta(&ret);
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

/// Si evaluar la expresion puede ejecutar una funcion (una llamada, o un operador que la maquina
/// sustituye por una rutina de la biblioteca).
fn puede_llamar(e: &Expr) -> bool {
    match &e.clase {
        ClaseExpr::Llamada(..) => true,
        ClaseExpr::Binario(op, a, b) => {
            matches!(
                op,
                OpBinario::Mul
                    | OpBinario::Div
                    | OpBinario::Mod
                    | OpBinario::DespIzq
                    | OpBinario::DespDer
                    | OpBinario::SumaSat
                    | OpBinario::RestaSat
            ) || puede_llamar(a)
                || puede_llamar(b)
        }
        ClaseExpr::Unario(_, a) | ClaseExpr::Cast(a) | ClaseExpr::Entrada(a) => puede_llamar(a),
        ClaseExpr::Leer(s) | ClaseExpr::Direccion(s) => sitio_puede_llamar(s),
        ClaseExpr::Const(_) | ClaseExpr::Bandera(_) => false,
    }
}

fn sitio_puede_llamar(s: &crate::hir::Sitio) -> bool {
    use crate::hir::ClaseSitio;
    match &s.clase {
        ClaseSitio::Variable(_) | ClaseSitio::Registro(_) => false,
        ClaseSitio::Indice(b, i, _) => sitio_puede_llamar(b) || puede_llamar(i),
        ClaseSitio::Campo(b, _) => sitio_puede_llamar(b),
        ClaseSitio::Deref(p) => puede_llamar(p),
    }
}
