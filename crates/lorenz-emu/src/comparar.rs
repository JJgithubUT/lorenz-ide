//! Comparacion de dos implementaciones sobre el mismo programa (RF-EMU-046).
//!
//! Cualquier cosa que sepa dar un paso y mostrar su estado es un [`Ejecutor`]. El comparador los
//! avanza de a una instruccion y se detiene en la primera diferencia, que es la que ayuda a
//! encontrar el error: lo que viene despues suele ser consecuencia de ella.

use crate::emulador::{Emulador, Paso};
use crate::error::ErrorEjecucion;

/// Lo observable de una maquina entre dos instrucciones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vista {
    pub pc: u64,
    /// Cada registro con su valor, en el orden de la arquitectura.
    pub registros: Vec<(String, u64)>,
    pub instrucciones: u64,
    pub ciclos: u64,
    pub detenida: bool,
    /// Suma de verificacion de toda la memoria.
    pub memoria: u64,
}

impl Vista {
    /// Descripcion de cada diferencia con otra vista; vacio si son iguales.
    pub fn diferencias(&self, otra: &Vista) -> Vec<String> {
        let mut d = Vec::new();
        if self.pc != otra.pc {
            d.push(format!("pc: {:#x} frente a {:#x}", self.pc, otra.pc));
        }
        for (nombre, valor) in &self.registros {
            match otra.registros.iter().find(|(n, _)| n == nombre) {
                Some((_, v)) if v == valor => {}
                Some((_, v)) => d.push(format!("{nombre}: {valor:#x} frente a {v:#x}")),
                None => d.push(format!("{nombre}: solo existe en la primera")),
            }
        }
        for (nombre, _) in &otra.registros {
            if !self.registros.iter().any(|(n, _)| n == nombre) {
                d.push(format!("{nombre}: solo existe en la segunda"));
            }
        }
        if self.ciclos != otra.ciclos {
            d.push(format!("ciclos: {} frente a {}", self.ciclos, otra.ciclos));
        }
        if self.instrucciones != otra.instrucciones {
            d.push(format!(
                "instrucciones: {} frente a {}",
                self.instrucciones, otra.instrucciones
            ));
        }
        if self.detenida != otra.detenida {
            d.push(format!(
                "detenida: {} frente a {}",
                self.detenida, otra.detenida
            ));
        }
        if self.memoria != otra.memoria {
            d.push("memoria: el contenido es distinto".to_string());
        }
        d
    }
}

/// Una implementacion que se puede comparar con otra.
pub trait Ejecutor {
    fn paso(&mut self) -> Result<Option<Paso>, ErrorEjecucion>;
    fn vista(&self) -> Vista;
}

impl Ejecutor for Emulador {
    fn paso(&mut self) -> Result<Option<Paso>, ErrorEjecucion> {
        Emulador::paso(self)
    }

    fn vista(&self) -> Vista {
        let estado = self.estado();
        Vista {
            pc: estado.pc(),
            registros: estado
                .nombres()
                .iter()
                .map(|n| (n.clone(), estado.registro(n).unwrap_or(0)))
                .collect(),
            instrucciones: estado.instrucciones(),
            ciclos: estado.ciclos(),
            detenida: estado.detenida(),
            memoria: self.memoria().suma_de_verificacion(),
        }
    }
}

/// Resultado de comparar dos ejecuciones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Veredicto {
    /// Ambas terminaron igual tras ejecutar este numero de instrucciones.
    Iguales { instrucciones: u64 },
    /// Se separaron en la instruccion `instruccion` (la primera es la 0).
    Divergen {
        instruccion: u64,
        diferencias: Vec<String>,
        primera: Vista,
        segunda: Vista,
    },
    /// Siguieron iguales hasta el limite sin terminar.
    LimiteAlcanzado { limite: u64 },
}

fn resumen(r: &Result<Option<Paso>, ErrorEjecucion>) -> String {
    match r {
        Ok(Some(p)) => format!(
            "{:#x} {} {} ciclos{}",
            p.direccion,
            p.mnemonico,
            p.ciclos,
            if p.salto { " con salto" } else { "" }
        ),
        Ok(None) => "ya estaba detenida".to_string(),
        Err(e) => format!("fallo: {e}"),
    }
}

/// Avanza las dos maquinas una instruccion a la vez, hasta `limite` instrucciones.
pub fn comparar(a: &mut dyn Ejecutor, b: &mut dyn Ejecutor, limite: u64) -> Veredicto {
    for n in 0..limite {
        let ra = a.paso();
        let rb = b.paso();
        let (sa, sb) = (resumen(&ra), resumen(&rb));
        let (va, vb) = (a.vista(), b.vista());
        let mut diferencias = va.diferencias(&vb);
        if sa != sb {
            diferencias.insert(0, format!("instruccion: {sa} frente a {sb}"));
        }
        if !diferencias.is_empty() {
            return Veredicto::Divergen {
                instruccion: n,
                diferencias,
                primera: va,
                segunda: vb,
            };
        }
        // Las dos terminaron, o fallaron de la misma manera: no hay mas que comparar.
        if matches!(ra, Ok(None) | Err(_)) {
            return Veredicto::Iguales { instrucciones: n };
        }
        if va.detenida {
            return Veredicto::Iguales {
                instrucciones: n + 1,
            };
        }
    }
    Veredicto::LimiteAlcanzado { limite }
}
