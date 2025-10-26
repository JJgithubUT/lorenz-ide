//! El enlazador: combina varios objetos en uno solo (RF-ENS-019, RF-ENS-020).
//!
//! Los objetos ya vienen ubicados (cada uno se ensamblo con sus `.org`), asi que enlazar no mueve
//! codigo: reune los simbolos de todos, comprueba que ningun par de objetos escriba la misma
//! direccion, y completa las instrucciones que usaban un `.externo`. Cada una de ellas se ensambla
//! de nuevo en su direccion con la tabla de simbolos global; como el tamano de una instruccion no
//! depende de sus valores, el resultado ocupa justo el hueco que dejo el ensamblador.
//!
//! Se reportan todos los problemas juntos: simbolos duplicados, simbolos sin resolver, solapes y
//! las instrucciones que, ya con el valor real, no se pueden codificar.

use std::collections::BTreeMap;
use std::fmt;

use crate::ensamblador::{Ensamblador, EntradaMapa, Objeto, Segmento, Simbolo};
use crate::error::ErrorEns;

/// Donde se definio algo: archivo y linea.
pub type Lugar = (String, usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorEnlace {
    /// Dos objetos definen el mismo simbolo (RF-ENS-020).
    Duplicado {
        nombre: String,
        primero: Lugar,
        segundo: Lugar,
    },
    /// Un objeto usa un simbolo externo que ningun objeto define (RF-ENS-020).
    NoResuelto { nombre: String, usado_en: Lugar },
    /// Dos objetos escriben la misma direccion.
    Solape {
        direccion: u64,
        primero: String,
        segundo: String,
    },
    /// Una instruccion pendiente no se puede codificar con el valor final.
    AlCompletar(ErrorEns),
}

impl fmt::Display for ErrorEnlace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorEnlace::Duplicado {
                nombre,
                primero,
                segundo,
            } => write!(
                f,
                "el simbolo `{nombre}` esta definido dos veces: en {}:{} y en {}:{}",
                primero.0, primero.1, segundo.0, segundo.1
            ),
            ErrorEnlace::NoResuelto { nombre, usado_en } => write!(
                f,
                "{}:{}: el simbolo externo `{nombre}` no lo define ningun objeto",
                usado_en.0, usado_en.1
            ),
            ErrorEnlace::Solape {
                direccion,
                primero,
                segundo,
            } => write!(
                f,
                "los objetos `{primero}` y `{segundo}` escriben la direccion {direccion:#x}"
            ),
            ErrorEnlace::AlCompletar(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ErrorEnlace {}

/// Combina `objetos` (nombre, objeto) en uno. El nombre solo sirve para los mensajes.
pub fn enlazar(
    ensamblador: &Ensamblador<'_>,
    objetos: &[(String, Objeto)],
) -> Result<Objeto, Vec<ErrorEnlace>> {
    let mut errores = Vec::new();

    // Simbolos de todos los objetos. Dos constantes iguales no chocan (es normal repetir una
    // `.equ` comun); una etiqueta repetida o una constante con otro valor si.
    let mut simbolos: BTreeMap<String, Simbolo> = BTreeMap::new();
    for (_, o) in objetos {
        for (nombre, s) in &o.simbolos {
            match simbolos.get(nombre) {
                None => {
                    simbolos.insert(nombre.clone(), s.clone());
                }
                Some(previo) if previo.etiqueta || s.etiqueta || previo.valor != s.valor => {
                    errores.push(ErrorEnlace::Duplicado {
                        nombre: nombre.clone(),
                        primero: (previo.archivo.clone(), previo.linea),
                        segundo: (s.archivo.clone(), s.linea),
                    });
                }
                Some(_) => {}
            }
        }
    }

    // Simbolos externos que nadie define.
    for (_, o) in objetos {
        for r in &o.reubicaciones {
            for nombre in &r.simbolos {
                if !simbolos.contains_key(nombre) {
                    errores.push(ErrorEnlace::NoResuelto {
                        nombre: nombre.clone(),
                        usado_en: (r.archivo.clone(), r.linea),
                    });
                }
            }
        }
    }

    // Palabras de todos los objetos; ningun par puede escribir la misma direccion.
    let mut palabras: BTreeMap<u64, (u64, usize)> = BTreeMap::new();
    let mut avisados: Vec<(usize, usize)> = Vec::new();
    for (i, (_, o)) in objetos.iter().enumerate() {
        for (dir, p) in o.palabras() {
            if let Some(&(_, otro)) = palabras.get(&dir) {
                if !avisados.contains(&(otro, i)) {
                    avisados.push((otro, i));
                    errores.push(ErrorEnlace::Solape {
                        direccion: dir,
                        primero: objetos[otro].0.clone(),
                        segundo: objetos[i].0.clone(),
                    });
                }
            } else {
                palabras.insert(dir, (p, i));
            }
        }
    }

    if !errores.is_empty() {
        return Err(errores);
    }

    // Completar las instrucciones pendientes con el valor final de cada simbolo.
    let valores: BTreeMap<String, i128> =
        simbolos.iter().map(|(n, s)| (n.clone(), s.valor)).collect();
    for (_, o) in objetos {
        for r in &o.reubicaciones {
            match ensamblador.ensamblar_instruccion_en(
                &r.fuente,
                r.direccion,
                &valores,
                &r.archivo,
                r.linea,
            ) {
                Ok(p) => {
                    for (k, w) in p.into_iter().enumerate() {
                        if let Some(e) = palabras.get_mut(&(r.direccion + k as u64)) {
                            e.0 = w;
                        }
                    }
                }
                Err(es) => errores.extend(es.into_iter().map(ErrorEnlace::AlCompletar)),
            }
        }
    }
    if !errores.is_empty() {
        return Err(errores);
    }

    let mut segmentos: Vec<Segmento> = Vec::new();
    for (dir, (p, _)) in palabras {
        match segmentos.last_mut() {
            Some(s) if s.direccion + s.palabras.len() as u64 == dir => s.palabras.push(p),
            _ => segmentos.push(Segmento {
                direccion: dir,
                palabras: vec![p],
            }),
        }
    }
    let mut mapa: Vec<EntradaMapa> = objetos.iter().flat_map(|(_, o)| o.mapa.clone()).collect();
    mapa.sort_by_key(|e| e.direccion);
    Ok(Objeto {
        segmentos,
        simbolos,
        mapa,
        externos: BTreeMap::new(),
        reubicaciones: Vec::new(),
    })
}
