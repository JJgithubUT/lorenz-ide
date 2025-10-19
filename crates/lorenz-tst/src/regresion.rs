//! Comparar dos ejecuciones de la suite para ver que se rompio o se volvio mas lento
//! (RF-TST-027, RF-TST-028, RF-TST-036).
//!
//! Se compara un informe de resultados anterior con el actual. Una prueba que pasaba y ahora no
//! es una ruptura; una que sigue pasando pero gasta mas ciclos que antes es una lentitud. Los
//! ciclos sirven de medida porque no dependen de la velocidad del equipo que corre la suite.

use std::fmt::Write;

use serde::{Deserialize, Serialize};

use crate::ejecutor::Veredicto;
use crate::suite::Informe;

pub const SALIDA_SIN_REGRESION: i32 = 0;
/// Una prueba que pasaba ahora falla o da error.
pub const SALIDA_ROMPE: i32 = 1;
/// Ninguna prueba se rompio pero alguna gasta mas ciclos que antes.
pub const SALIDA_MAS_LENTA: i32 = 6;

/// Cuanto puede crecer el numero de ciclos antes de contar como lentitud.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Umbral {
    /// Porcentaje de aumento tolerado; 0 marca cualquier aumento.
    pub porcentaje: f64,
}

impl Default for Umbral {
    fn default() -> Self {
        Umbral { porcentaje: 0.0 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum Cambio {
    /// Pasaba y ahora no.
    Rompe {
        nombre: String,
        ahora: Veredicto,
        detalle: String,
    },
    /// No pasaba y ahora si.
    Repara { nombre: String },
    /// Pasa en las dos, pero con mas ciclos.
    MasLenta {
        nombre: String,
        antes: u64,
        ahora: u64,
    },
    /// Pasa en las dos, pero con menos ciclos.
    MasRapida {
        nombre: String,
        antes: u64,
        ahora: u64,
    },
    /// Una prueba que antes no estaba.
    Nueva {
        nombre: String,
        veredicto: Veredicto,
    },
    /// Una prueba que estaba y ya no.
    Desaparecida { nombre: String },
}

impl Cambio {
    pub fn nombre(&self) -> &str {
        match self {
            Cambio::Rompe { nombre, .. }
            | Cambio::Repara { nombre }
            | Cambio::MasLenta { nombre, .. }
            | Cambio::MasRapida { nombre, .. }
            | Cambio::Nueva { nombre, .. }
            | Cambio::Desaparecida { nombre } => nombre,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InformeRegresion {
    pub cambios: Vec<Cambio>,
    /// Pruebas que se compararon y no cambiaron.
    pub iguales: usize,
}

fn porcentaje(antes: u64, ahora: u64) -> f64 {
    if antes == 0 {
        if ahora == 0 {
            0.0
        } else {
            f64::INFINITY
        }
    } else {
        (ahora as f64 - antes as f64) * 100.0 / antes as f64
    }
}

/// Compara el informe `anterior` con el `actual`.
pub fn comparar_resultados(
    anterior: &Informe,
    actual: &Informe,
    umbral: Umbral,
) -> InformeRegresion {
    let mut cambios = Vec::new();
    let mut iguales = 0;
    for ahora in &actual.pruebas {
        let Some(antes) = anterior.pruebas.iter().find(|p| p.nombre == ahora.nombre) else {
            cambios.push(Cambio::Nueva {
                nombre: ahora.nombre.clone(),
                veredicto: ahora.veredicto,
            });
            continue;
        };
        match (antes.veredicto, ahora.veredicto) {
            (Veredicto::Pasa, Veredicto::Pasa) => {
                let p = porcentaje(antes.ciclos, ahora.ciclos);
                if ahora.ciclos > antes.ciclos && p > umbral.porcentaje {
                    cambios.push(Cambio::MasLenta {
                        nombre: ahora.nombre.clone(),
                        antes: antes.ciclos,
                        ahora: ahora.ciclos,
                    });
                } else if ahora.ciclos < antes.ciclos {
                    cambios.push(Cambio::MasRapida {
                        nombre: ahora.nombre.clone(),
                        antes: antes.ciclos,
                        ahora: ahora.ciclos,
                    });
                } else {
                    iguales += 1;
                }
            }
            (Veredicto::Pasa, v) => {
                let detalle = ahora
                    .motivo
                    .clone()
                    .or_else(|| {
                        ahora.fallos.first().map(|f| {
                            format!(
                                "{}: esperado {}, obtenido {}",
                                f.que, f.esperado, f.obtenido
                            )
                        })
                    })
                    .unwrap_or_default();
                cambios.push(Cambio::Rompe {
                    nombre: ahora.nombre.clone(),
                    ahora: v,
                    detalle,
                });
            }
            (_, Veredicto::Pasa) => cambios.push(Cambio::Repara {
                nombre: ahora.nombre.clone(),
            }),
            _ => iguales += 1,
        }
    }
    for antes in &anterior.pruebas {
        if !actual.pruebas.iter().any(|p| p.nombre == antes.nombre) {
            cambios.push(Cambio::Desaparecida {
                nombre: antes.nombre.clone(),
            });
        }
    }
    InformeRegresion { cambios, iguales }
}

impl InformeRegresion {
    pub fn rompe(&self) -> bool {
        self.cambios
            .iter()
            .any(|c| matches!(c, Cambio::Rompe { .. }))
    }

    pub fn mas_lentas(&self) -> bool {
        self.cambios
            .iter()
            .any(|c| matches!(c, Cambio::MasLenta { .. }))
    }

    pub fn hay_regresion(&self) -> bool {
        self.rompe() || self.mas_lentas()
    }

    /// 1 si algo se rompio, 6 si solo hay lentitud, 0 si no hay regresion.
    pub fn codigo_de_salida(&self) -> i32 {
        if self.rompe() {
            SALIDA_ROMPE
        } else if self.mas_lentas() {
            SALIDA_MAS_LENTA
        } else {
            SALIDA_SIN_REGRESION
        }
    }

    pub fn a_texto(&self) -> String {
        let mut t = String::from("comparacion con la ejecucion anterior\n");
        for c in &self.cambios {
            let _ = match c {
                Cambio::Rompe {
                    nombre,
                    ahora,
                    detalle,
                } => {
                    let v = match ahora {
                        Veredicto::Falla => "falla",
                        Veredicto::Error => "da error",
                        Veredicto::Pasa => "pasa",
                    };
                    let mut s = format!("  SE ROMPE   {nombre}: pasaba y ahora {v}");
                    if !detalle.is_empty() {
                        let primera = detalle.lines().next().unwrap_or("");
                        let _ = write!(s, " ({primera})");
                    }
                    writeln!(t, "{s}")
                }
                Cambio::Repara { nombre } => writeln!(t, "  se repara  {nombre}"),
                Cambio::MasLenta {
                    nombre,
                    antes,
                    ahora,
                } => writeln!(
                    t,
                    "  MAS LENTA  {nombre}: {antes} -> {ahora} ciclos (+{:.1}%)",
                    porcentaje(*antes, *ahora)
                ),
                Cambio::MasRapida {
                    nombre,
                    antes,
                    ahora,
                } => writeln!(
                    t,
                    "  mas rapida {nombre}: {antes} -> {ahora} ciclos ({:.1}%)",
                    porcentaje(*antes, *ahora)
                ),
                Cambio::Nueva { nombre, veredicto } => {
                    writeln!(t, "  nueva      {nombre} ({veredicto:?})")
                }
                Cambio::Desaparecida { nombre } => writeln!(t, "  desaparece {nombre}"),
            };
        }
        let _ = writeln!(
            t,
            "{} sin cambios; {}",
            self.iguales,
            if self.rompe() {
                "hay pruebas rotas"
            } else if self.mas_lentas() {
                "hay pruebas mas lentas"
            } else {
                "sin regresion"
            }
        );
        t
    }
}
