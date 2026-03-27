//! Cobertura de instrucciones y de ramas (RF-TST-021, RF-TST-022, RF-TST-023).
//!
//! La cobertura de instrucciones cuenta cuantas veces se ejecuto cada instruccion de la ISA y deja
//! a la vista las que ninguna prueba ejercita. La de ramas mira cada salto condicional de un
//! programa y pide que se haya visto tomado y sin tomar.
//!
//! Un salto cuyo destino es la propia instruccion siguiente se ve como "no tomado": es lo que el
//! emulador informa y no hay otra forma de distinguirlo desde fuera.

use std::collections::BTreeMap;

use lorenz_arch::modelo::{Ciclos, Instruccion, Maquina};
use lorenz_arch::semantica::ast::{Destino, Sentencia};
use lorenz_emu::Paso;
use serde::{Deserialize, Serialize};

/// Una instruccion de salto condicional: su ciclo cambia segun se tome o no, o asigna el contador
/// de programa dentro de un `si`.
pub fn es_ramificacion(maquina: &Maquina, ins: &Instruccion) -> bool {
    if matches!(ins.ciclos, Ciclos::SegunSalto { .. }) {
        return true;
    }
    let pc = maquina
        .registro_con_rol(lorenz_arch::modelo::RolRegistro::ContadorPrograma)
        .map(|r| r.nombre.as_str());
    fn salta(s: &[Sentencia], pc: Option<&str>) -> bool {
        s.iter().any(|s| match s {
            Sentencia::Asignar(Destino::Nombre(n), _) => n == "pc" || Some(n.as_str()) == pc,
            Sentencia::Si(_, cuerpo) => salta(cuerpo, pc),
            _ => false,
        })
    }
    ins.programa.sentencias.iter().any(|s| match s {
        Sentencia::Si(_, cuerpo) => salta(cuerpo, pc),
        _ => false,
    })
}

/// Lo que se vio en un salto condicional concreto.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Rama {
    tomada: u64,
    no_tomada: u64,
}

/// Lo que ha ejecutado uno o varios programas.
#[derive(Debug, Clone)]
pub struct Cobertura {
    mnemonicos: Vec<String>,
    ramificaciones: Vec<bool>,
    ejecuciones: Vec<u64>,
    /// `(programa, direccion)` -> resultados vistos.
    ramas: BTreeMap<(String, u64), (usize, Rama)>,
}

impl Cobertura {
    pub fn nueva(maquina: &Maquina) -> Self {
        Cobertura {
            mnemonicos: maquina
                .instrucciones
                .iter()
                .map(|i| i.mnemonico.clone())
                .collect(),
            ramificaciones: maquina
                .instrucciones
                .iter()
                .map(|i| es_ramificacion(maquina, i))
                .collect(),
            ejecuciones: vec![0; maquina.instrucciones.len()],
            ramas: BTreeMap::new(),
        }
    }

    /// Anota una instruccion ejecutada por el programa `programa`.
    pub fn registrar(&mut self, programa: &str, paso: &Paso) {
        let Some(indice) = self.mnemonicos.iter().position(|m| *m == paso.mnemonico) else {
            return;
        };
        self.ejecuciones[indice] += 1;
        if self.ramificaciones[indice] {
            let (_, r) = self
                .ramas
                .entry((programa.to_string(), paso.direccion))
                .or_insert((indice, Rama::default()));
            if paso.salto {
                r.tomada += 1;
            } else {
                r.no_tomada += 1;
            }
        }
    }

    /// Suma lo visto en otra ejecucion de la misma maquina.
    pub fn fusionar(&mut self, otra: &Cobertura) {
        for (a, b) in self.ejecuciones.iter_mut().zip(&otra.ejecuciones) {
            *a += b;
        }
        for (clave, (indice, r)) in &otra.ramas {
            let (_, mio) = self
                .ramas
                .entry(clave.clone())
                .or_insert((*indice, Rama::default()));
            mio.tomada += r.tomada;
            mio.no_tomada += r.no_tomada;
        }
    }

    pub fn informe(&self) -> InformeCobertura {
        let instrucciones: Vec<EntradaInstruccion> = self
            .mnemonicos
            .iter()
            .zip(&self.ejecuciones)
            .zip(&self.ramificaciones)
            .map(|((m, n), r)| EntradaInstruccion {
                mnemonico: m.clone(),
                ejecuciones: *n,
                ramificacion: *r,
            })
            .collect();
        let ramas: Vec<EntradaRama> = self
            .ramas
            .iter()
            .map(|((programa, direccion), (indice, r))| EntradaRama {
                programa: programa.clone(),
                direccion: *direccion,
                mnemonico: self.mnemonicos[*indice].clone(),
                tomada: r.tomada,
                no_tomada: r.no_tomada,
            })
            .collect();
        InformeCobertura {
            instrucciones_ejercitadas: instrucciones.iter().filter(|i| i.ejecuciones > 0).count(),
            instrucciones,
            ramas,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntradaInstruccion {
    pub mnemonico: String,
    pub ejecuciones: u64,
    /// Si es un salto condicional.
    pub ramificacion: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntradaRama {
    pub programa: String,
    pub direccion: u64,
    pub mnemonico: String,
    pub tomada: u64,
    pub no_tomada: u64,
}

impl EntradaRama {
    pub fn completa(&self) -> bool {
        self.tomada > 0 && self.no_tomada > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InformeCobertura {
    pub instrucciones_ejercitadas: usize,
    pub instrucciones: Vec<EntradaInstruccion>,
    pub ramas: Vec<EntradaRama>,
}

impl InformeCobertura {
    /// Instrucciones de la ISA que nadie ejecuto (RF-TST-023).
    pub fn no_ejercitadas(&self) -> Vec<&str> {
        self.instrucciones
            .iter()
            .filter(|i| i.ejecuciones == 0)
            .map(|i| i.mnemonico.as_str())
            .collect()
    }

    pub fn porcentaje_de_instrucciones(&self) -> f64 {
        porcentaje(self.instrucciones_ejercitadas, self.instrucciones.len())
    }

    /// Resultados de salto cubiertos y posibles (dos por cada salto condicional visto).
    pub fn ramas_cubiertas(&self) -> (usize, usize) {
        let cubiertas = self
            .ramas
            .iter()
            .map(|r| usize::from(r.tomada > 0) + usize::from(r.no_tomada > 0))
            .sum();
        (cubiertas, self.ramas.len() * 2)
    }

    pub fn porcentaje_de_ramas(&self) -> f64 {
        let (c, t) = self.ramas_cubiertas();
        porcentaje(c, t)
    }

    pub fn a_texto(&self) -> String {
        use std::fmt::Write;
        let mut t = String::new();
        let _ = writeln!(
            t,
            "cobertura de instrucciones: {}/{} ({:.1}%)",
            self.instrucciones_ejercitadas,
            self.instrucciones.len(),
            self.porcentaje_de_instrucciones()
        );
        let sin: Vec<&str> = self.no_ejercitadas();
        if !sin.is_empty() {
            let _ = writeln!(t, "  sin ejercitar: {}", sin.join(", "));
        }
        let (c, total) = self.ramas_cubiertas();
        if total == 0 {
            let _ = writeln!(t, "cobertura de ramas: ningun salto condicional ejecutado");
        } else {
            let _ = writeln!(
                t,
                "cobertura de ramas: {c}/{total} ({:.1}%)",
                self.porcentaje_de_ramas()
            );
            for r in self.ramas.iter().filter(|r| !r.completa()) {
                let falta = if r.tomada == 0 {
                    "nunca se toma"
                } else {
                    "nunca se deja de tomar"
                };
                let _ = writeln!(
                    t,
                    "  {}:{:#06x} {}: {falta}",
                    r.programa, r.direccion, r.mnemonico
                );
            }
        }
        t
    }
}

fn porcentaje(parte: usize, total: usize) -> f64 {
    if total == 0 {
        100.0
    } else {
        parte as f64 * 100.0 / total as f64
    }
}
