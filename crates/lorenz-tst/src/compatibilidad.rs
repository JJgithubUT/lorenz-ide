//! Avisar cuando un cambio de ISA deja sin poder ejecutarse un binario anterior (RF-TST-029).
//!
//! Un binario son palabras sueltas: solo la arquitectura con que se hizo dice que instruccion es
//! cada una. Se desensambla con la arquitectura anterior y cada instruccion se vuelve a leer con
//! la nueva. Si ya no se decodifica, o se decodifica como otra cosa, el binario dejo de valer.

use lorenz_arch::modelo::{Instruccion, Maquina};
use lorenz_ens::desensamblador::{Desensamblador, Linea};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum Motivo {
    /// La arquitectura nueva no reconoce esas palabras como instruccion.
    NoSeReconoce,
    /// Se reconoce, pero ocupa otro numero de palabras: lo que sigue se desalinea.
    OtraLongitud { ahora: usize },
    /// Se reconoce como otra instruccion.
    OtraInstruccion { ahora: String },
    /// Es la misma instruccion pero su semantica, banderas o ciclos cambiaron.
    CambiaElComportamiento { que: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Incompatibilidad {
    pub direccion: u64,
    pub palabras: Vec<u64>,
    /// Como se leia con la arquitectura anterior.
    pub antes: String,
    pub motivo: Motivo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InformeCompatibilidad {
    pub anterior: String,
    pub nueva: String,
    /// Instrucciones del binario que se revisaron.
    pub instrucciones: usize,
    pub incompatibles: Vec<Incompatibilidad>,
    /// Si la arquitectura nueva usa otro ancho de palabra, ningun binario anterior sirve.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incompatibilidad_global: Option<String>,
}

fn diferencia(antes: &Instruccion, ahora: &Instruccion) -> Option<String> {
    if antes.programa != ahora.programa {
        return Some(format!(
            "la semantica pasa de `{}` a `{}`",
            antes.semantica, ahora.semantica
        ));
    }
    if antes.afecta != ahora.afecta {
        return Some(format!(
            "las banderas que afecta pasan de {:?} a {:?}",
            antes.afecta, ahora.afecta
        ));
    }
    if antes.ciclos != ahora.ciclos {
        return Some(format!(
            "los ciclos pasan de {:?} a {:?}",
            antes.ciclos, ahora.ciclos
        ));
    }
    None
}

/// Revisa un binario de la arquitectura `anterior` contra la `nueva`.
///
/// `tramos` son los bloques del binario como `(direccion, palabras)`.
pub fn comprobar_binario(
    anterior: &Maquina,
    nueva: &Maquina,
    tramos: &[(u64, Vec<u64>)],
) -> Result<InformeCompatibilidad, String> {
    let mut informe = InformeCompatibilidad {
        anterior: anterior.nombre.clone(),
        nueva: nueva.nombre.clone(),
        instrucciones: 0,
        incompatibles: Vec::new(),
        incompatibilidad_global: None,
    };
    if anterior.bits_palabra != nueva.bits_palabra || anterior.endianness != nueva.endianness {
        informe.incompatibilidad_global = Some(format!(
            "cambia el ancho de palabra ({} -> {} bits) o el orden de bytes: ningun binario anterior sirve",
            anterior.bits_palabra, nueva.bits_palabra
        ));
        return Ok(informe);
    }
    let viejo = Desensamblador::nuevo(anterior)?;
    let nuevo = Desensamblador::nuevo(nueva)?;
    for (origen, palabras) in tramos {
        for linea in viejo.desensamblar(*origen, palabras, None) {
            let Linea::Instruccion {
                direccion,
                palabras: p,
                texto,
                indice,
            } = linea
            else {
                continue;
            };
            informe.instrucciones += 1;
            let motivo = match nuevo.desensamblar(direccion, &p, None).into_iter().next() {
                Some(Linea::Instruccion {
                    palabras: q,
                    texto: otro,
                    indice: j,
                    ..
                }) => {
                    let (vieja, nueva_ins) =
                        (&anterior.instrucciones[indice], &nueva.instrucciones[j]);
                    if q.len() != p.len() {
                        Some(Motivo::OtraLongitud { ahora: q.len() })
                    } else if vieja.mnemonico != nueva_ins.mnemonico {
                        Some(Motivo::OtraInstruccion { ahora: otro })
                    } else {
                        diferencia(vieja, nueva_ins)
                            .map(|que| Motivo::CambiaElComportamiento { que })
                    }
                }
                _ => Some(Motivo::NoSeReconoce),
            };
            if let Some(motivo) = motivo {
                informe.incompatibles.push(Incompatibilidad {
                    direccion,
                    palabras: p,
                    antes: texto,
                    motivo,
                });
            }
        }
    }
    Ok(informe)
}

impl InformeCompatibilidad {
    pub fn es_compatible(&self) -> bool {
        self.incompatibles.is_empty() && self.incompatibilidad_global.is_none()
    }

    pub fn a_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("el informe siempre se serializa");
        s.push('\n');
        s
    }

    pub fn a_texto(&self) -> String {
        use std::fmt::Write;
        let mut t = format!(
            "compatibilidad de un binario de {} con {}\n",
            self.anterior, self.nueva
        );
        if let Some(g) = &self.incompatibilidad_global {
            let _ = writeln!(t, "  INCOMPATIBLE: {g}");
            return t;
        }
        for i in &self.incompatibles {
            let palabras: Vec<String> = i.palabras.iter().map(|p| format!("{p:02x}")).collect();
            let porque = match &i.motivo {
                Motivo::NoSeReconoce => "la arquitectura nueva no la reconoce".to_string(),
                Motivo::OtraLongitud { ahora } => {
                    format!(
                        "ahora ocupa {ahora} palabras en vez de {}",
                        i.palabras.len()
                    )
                }
                Motivo::OtraInstruccion { ahora } => format!("ahora es `{ahora}`"),
                Motivo::CambiaElComportamiento { que } => {
                    format!("cambia el comportamiento: {que}")
                }
            };
            let _ = writeln!(
                t,
                "  {:#06x} [{}] `{}`: {porque}",
                i.direccion,
                palabras.join(" "),
                i.antes
            );
        }
        let _ = writeln!(
            t,
            "{} instrucciones revisadas, {} incompatibles",
            self.instrucciones,
            self.incompatibles.len()
        );
        t
    }
}
