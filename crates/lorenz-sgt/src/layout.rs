//! Reparto estatico de la memoria de datos (decision D-059).
//!
//! Como SGT no admite recursion, cada variable local tiene una direccion fija y no hace falta
//! marco de pila. El compilador y el interprete de referencia usan este mismo reparto, de modo que
//! `&variable` vale lo mismo en los dos.

use crate::destino::Destino;
use crate::diagnostico::{Diagnostico, Lugar};
use crate::hir::Modulo;
use crate::tipos::Tipo;

/// Tamano en palabras de un tipo.
pub fn palabras(m: &Modulo, d: &Destino, t: &Tipo) -> u64 {
    match t {
        Tipo::Entero { bits, .. } => d.palabras_de(*bits),
        Tipo::Bool => 1,
        Tipo::Puntero(_) => d.palabras_de(d.direccion),
        Tipo::Arreglo(e, n) => palabras(m, d, e) * n,
        Tipo::Estructura(i) => m.estructuras[*i].tamano,
        Tipo::Vacio | Tipo::Error => 0,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disposicion {
    /// Region donde se reparten los datos.
    pub region: String,
    pub globales: Vec<u64>,
    /// Por funcion, la direccion de cada local.
    pub locales: Vec<Vec<u64>>,
    /// Inicio del area donde un manejador de interrupcion guarda los registros; ocupa una palabra
    /// por registro de la maquina y solo existe si hay manejadores.
    pub salvaguarda: Option<u64>,
    /// Primera direccion libre tras el ultimo dato repartido.
    pub libre: u64,
    /// Ultima direccion usable de la region.
    pub limite: u64,
}

pub fn disponer(m: &Modulo, d: &Destino) -> Result<Disposicion, Diagnostico> {
    let region = d
        .regiones
        .iter()
        .find(|r| r.lectura && r.escritura && !r.dispositivo)
        .ok_or_else(|| {
            Diagnostico::nuevo(
                &m.archivo,
                Lugar::nuevo(1, 1),
                format!(
                    "`{}` no tiene ninguna region de memoria de lectura y escritura para los datos",
                    d.nombre
                ),
            )
        })?;
    // Tramos ya ocupados por variables con direccion fija.
    let mut ocupados: Vec<(u64, u64)> = m
        .globales
        .iter()
        .filter_map(|g| {
            g.direccion
                .map(|a| (a, a + palabras(m, d, &g.tipo).max(1) - 1))
        })
        .collect();
    ocupados.sort();
    let mut cursor = region.desde;
    let mut salvaguarda = None;
    if m.funciones.iter().any(|f| f.interrupcion.is_some()) {
        salvaguarda = Some(cursor);
        cursor += d.registros.len() as u64;
    }
    let mut sitio = |tam: u64, lugar: Lugar, nombre: &str| -> Result<u64, Diagnostico> {
        let tam = tam.max(1);
        loop {
            if let Some(&(_, fin)) = ocupados
                .iter()
                .find(|(a, f)| cursor <= *f && cursor + tam > *a)
            {
                cursor = fin + 1;
                continue;
            }
            break;
        }
        if cursor + tam - 1 > region.hasta {
            return Err(Diagnostico::nuevo(
                &m.archivo,
                lugar,
                format!(
                    "no queda memoria para `{nombre}`: la region `{}` ({:#X}..{:#X}) esta llena",
                    region.nombre, region.desde, region.hasta
                ),
            ));
        }
        let a = cursor;
        cursor += tam;
        Ok(a)
    };
    let mut globales = Vec::new();
    for g in &m.globales {
        let a = match g.direccion {
            Some(a) => a,
            None => sitio(palabras(m, d, &g.tipo), g.lugar, &g.nombre)?,
        };
        globales.push(a);
    }
    let mut locales = Vec::new();
    for f in &m.funciones {
        let mut v = Vec::new();
        for l in &f.locales {
            v.push(sitio(
                palabras(m, d, &l.tipo),
                l.lugar,
                &format!("{}::{}", f.nombre, l.nombre),
            )?);
        }
        locales.push(v);
    }
    Ok(Disposicion {
        region: region.nombre.clone(),
        globales,
        locales,
        salvaguarda,
        libre: cursor,
        limite: region.hasta,
    })
}
