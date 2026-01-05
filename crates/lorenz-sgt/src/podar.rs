//! Quita del programa lo que no se usa de la biblioteca de apoyo.
//!
//! Las funciones del usuario se conservan siempre; las de la biblioteca (`__...`) solo si algun
//! camino las llama, directamente o porque la maquina no tiene la instruccion de un operador y el
//! compilador lo sustituye por una llamada (RF-SGT-041).

use crate::ast::OpBinario;
use crate::hir::*;
use crate::tipos::Tipo;

/// Decide si un operador sobre un tipo se resuelve llamando a una funcion de la biblioteca.
pub type Sustitucion<'a> = &'a dyn Fn(OpBinario, &Tipo) -> Option<&'static str>;

pub fn podar(mut m: Modulo, sustituye: Sustitucion<'_>) -> Modulo {
    let n = m.funciones.len();
    let mut alcanzada = vec![false; n];
    let mut pendientes: Vec<usize> = (0..n)
        .filter(|&i| {
            !m.funciones[i].nombre.starts_with("__") || m.funciones[i].interrupcion.is_some()
        })
        .collect();
    for &i in &pendientes {
        alcanzada[i] = true;
    }
    let mut globales_usadas = vec![false; m.globales.len()];
    for (i, g) in m.globales.iter().enumerate() {
        if !g.nombre.starts_with("__") {
            globales_usadas[i] = true;
        }
    }
    while let Some(f) = pendientes.pop() {
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
                    // Los operandos de comparacion llevan su tipo en el primer operando.
                }
                _ => {}
            },
            &mut |r| {
                if let Ref::Global(g) = r {
                    globales.push(*g);
                }
            },
        );
        // Una sustitucion puede nombrar una funcion de la biblioteca que a su vez usa globales.
        for g in globales {
            globales_usadas[g] = true;
        }
        for id in llamadas {
            if !alcanzada[id] {
                alcanzada[id] = true;
                pendientes.push(id);
            }
        }
    }
    // Renumerar.
    let mapa_f: Vec<Option<usize>> = {
        let mut k = 0;
        alcanzada
            .iter()
            .map(|&a| {
                if a {
                    k += 1;
                    Some(k - 1)
                } else {
                    None
                }
            })
            .collect()
    };
    let mapa_g: Vec<Option<usize>> = {
        let mut k = 0;
        globales_usadas
            .iter()
            .map(|&a| {
                if a {
                    k += 1;
                    Some(k - 1)
                } else {
                    None
                }
            })
            .collect()
    };
    let mut funciones = Vec::new();
    for (i, mut f) in m.funciones.drain(..).enumerate() {
        if mapa_f[i].is_none() {
            continue;
        }
        f.recorrer_mut(
            &mut |e| {
                if let ClaseExpr::Llamada(id, _) = &mut e.clase {
                    *id = mapa_f[*id].expect("llamada a una funcion podada");
                }
            },
            &mut |r| {
                if let Ref::Global(g) = r {
                    *g = mapa_g[*g].expect("referencia a una global podada");
                }
            },
        );
        funciones.push(f);
    }
    let globales = m
        .globales
        .drain(..)
        .enumerate()
        .filter(|(i, _)| mapa_g[*i].is_some())
        .map(|(_, g)| g)
        .collect();
    m.funciones = funciones;
    m.globales = globales;
    m
}
