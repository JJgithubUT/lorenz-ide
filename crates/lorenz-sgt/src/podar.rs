//! Deja en el programa solo lo que se usa y separa el codigo por raiz de ejecucion.
//!
//! - Las funciones del usuario se conservan siempre; las de la biblioteca (`__...`) solo si algun
//!   camino las llama, directamente o porque la maquina no tiene la instruccion de un operador y el
//!   compilador lo sustituye por una llamada (RF-SGT-041).
//! - Como las variables locales son estaticas, una funcion no puede ejecutarse a la vez desde el
//!   programa principal y desde un manejador de interrupcion. Cada raiz (el programa y cada
//!   manejador) recibe su propia copia de las funciones que alcanza, y de las variables de la
//!   biblioteca que esas copias usan (decision D-063).

use std::collections::{BTreeMap, BTreeSet};

use crate::analisis::PRINCIPAL;
use crate::ast::OpBinario;
use crate::hir::*;
use crate::tipos::Tipo;

/// Decide si un operador sobre un tipo se resuelve llamando a una funcion de la biblioteca.
pub type Sustitucion<'a> = &'a dyn Fn(OpBinario, &Tipo) -> Option<&'static str>;

fn es_biblioteca(n: &str) -> bool {
    crate::biblioteca::es_reservado(n)
}

/// Funciones y variables que alcanza una funcion directamente.
fn aristas(m: &Modulo, f: usize, sustituye: Sustitucion<'_>) -> (Vec<usize>, Vec<usize>) {
    let mut llamadas = Vec::new();
    let mut globales = Vec::new();
    let mut fun = m.funciones[f].clone();
    fun.recorrer_mut(
        &mut |e| match &e.clase {
            ClaseExpr::Llamada(id, _) => llamadas.push(*id),
            ClaseExpr::Binario(op, a, _) => {
                if let Some(nombre) = sustituye(*op, &a.tipo) {
                    if let Some(id) = m.funcion_por_nombre(nombre) {
                        llamadas.push(id);
                    }
                }
            }
            _ => {}
        },
        &mut |r| {
            if let Ref::Global(g) = r {
                globales.push(*g);
            }
        },
    );
    (llamadas, globales)
}

pub fn podar(m: Modulo, sustituye: Sustitucion<'_>) -> Modulo {
    let n = m.funciones.len();
    // Raices: el programa y los manejadores, en orden.
    let mut raices: Vec<Vec<usize>> = vec![Vec::new()];
    let mut semillas_principal: Vec<usize> = Vec::new();
    for (i, f) in m.funciones.iter().enumerate() {
        if f.interrupcion.is_some() {
            raices.push(vec![i]);
        } else if !es_biblioteca(&f.nombre) || f.nombre == PRINCIPAL {
            semillas_principal.push(i);
        }
    }
    raices[0] = semillas_principal;
    // Alcance de cada raiz.
    let mut alcance: Vec<BTreeSet<usize>> = Vec::new();
    for semillas in &raices {
        let mut vistos: BTreeSet<usize> = semillas.iter().copied().collect();
        let mut pendientes: Vec<usize> = semillas.clone();
        while let Some(f) = pendientes.pop() {
            for id in aristas(&m, f, sustituye).0 {
                if vistos.insert(id) {
                    pendientes.push(id);
                }
            }
        }
        alcance.push(vistos);
    }
    let _ = n;
    // Copias: una por (raiz, funcion). Las globales de la biblioteca tambien se copian por raiz.
    let mut nuevo = Modulo {
        estructuras: m.estructuras.clone(),
        archivo: m.archivo.clone(),
        ..Default::default()
    };
    let mut id_global: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    let mut id_funcion: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    // Las globales del usuario son unicas y van primero.
    let mut globales_usuario: BTreeMap<usize, usize> = BTreeMap::new();
    for (i, g) in m.globales.iter().enumerate() {
        if !es_biblioteca(&g.nombre) {
            globales_usuario.insert(i, nuevo.globales.len());
            nuevo.globales.push(g.clone());
        }
    }
    for (r, conj) in alcance.iter().enumerate() {
        for &f in conj {
            id_funcion.insert((r, f), nuevo.funciones.len());
            let mut c = m.funciones[f].clone();
            c.raiz = r;
            if r > 0 && !m.funciones[f].interrupcion.is_some() {
                c.nombre = format!("{}__r{r}", c.base);
            }
            nuevo.funciones.push(c);
        }
    }
    // Reescribir llamadas y referencias de cada copia.
    for (&(r, _), &nid) in &id_funcion.clone() {
        let mut fun = nuevo.funciones[nid].clone();
        let mut necesarias: Vec<usize> = Vec::new();
        fun.recorrer_mut(
            &mut |e| {
                if let ClaseExpr::Llamada(id, _) = &mut e.clase {
                    *id = id_funcion[&(r, *id)];
                }
            },
            &mut |rf| {
                if let Ref::Global(g) = rf {
                    necesarias.push(*g);
                }
            },
        );
        // Las referencias a globales se resuelven despues de crear las copias de la biblioteca.
        for g in necesarias {
            if !globales_usuario.contains_key(&g) && !id_global.contains_key(&(r, g)) {
                let mut c = m.globales[g].clone();
                if r > 0 {
                    c.nombre = format!("{}__r{r}", c.nombre);
                }
                id_global.insert((r, g), nuevo.globales.len());
                nuevo.globales.push(c);
            }
        }
        fun.recorrer_mut(&mut |_| {}, &mut |rf| {
            if let Ref::Global(g) = rf {
                *g = globales_usuario
                    .get(g)
                    .copied()
                    .unwrap_or_else(|| id_global[&(r, *g)]);
            }
        });
        nuevo.funciones[nid] = fun;
    }
    nuevo
}
