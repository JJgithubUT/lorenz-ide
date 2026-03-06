//! El archivo objeto: lo que produce el ensamblador para guardarlo o enlazarlo (RF-ENS-016).
//!
//! Es TOML, asi un humano lo lee y una herramienta externa lo procesa sin formato propio. Lleva el
//! codigo (segmentos con su direccion), los simbolos y la informacion de ubicacion (el mapa de
//! direccion a archivo y linea).

use serde::{Deserialize, Serialize};

use crate::ensamblador::{EntradaMapa, Externo, Objeto, Reubicacion, Segmento, Simbolo};

/// Version del formato del archivo objeto.
pub const FORMATO_OBJETO: &str = "lorenz-objeto-0.1";

#[derive(Serialize, Deserialize)]
struct Archivo {
    formato: String,
    maquina: String,
    #[serde(default, rename = "segmento")]
    segmentos: Vec<SegmentoToml>,
    #[serde(default, rename = "simbolo")]
    simbolos: Vec<SimboloToml>,
    #[serde(default, rename = "ubicacion")]
    mapa: Vec<UbicacionToml>,
    #[serde(default, rename = "externo", skip_serializing_if = "Vec::is_empty")]
    externos: Vec<ExternoToml>,
    #[serde(default, rename = "reubicacion", skip_serializing_if = "Vec::is_empty")]
    reubicaciones: Vec<ReubicacionToml>,
}

#[derive(Serialize, Deserialize)]
struct ExternoToml {
    nombre: String,
    archivo: String,
    linea: usize,
}

#[derive(Serialize, Deserialize)]
struct ReubicacionToml {
    direccion: u64,
    longitud: usize,
    archivo: String,
    linea: usize,
    fuente: String,
    simbolos: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct SegmentoToml {
    direccion: u64,
    palabras: Vec<u64>,
}

#[derive(Serialize, Deserialize)]
struct SimboloToml {
    nombre: String,
    valor: i64,
    etiqueta: bool,
    archivo: String,
    linea: usize,
}

#[derive(Serialize, Deserialize)]
struct UbicacionToml {
    direccion: u64,
    longitud: usize,
    archivo: String,
    linea: usize,
}

impl Objeto {
    /// Serializa el objeto. `maquina` es el nombre de la arquitectura para la que se ensamblo.
    pub fn a_toml(&self, maquina: &str) -> String {
        let archivo = Archivo {
            formato: FORMATO_OBJETO.to_string(),
            maquina: maquina.to_string(),
            segmentos: self
                .segmentos
                .iter()
                .map(|s| SegmentoToml {
                    direccion: s.direccion,
                    palabras: s.palabras.clone(),
                })
                .collect(),
            simbolos: self
                .simbolos
                .iter()
                .map(|(n, s)| SimboloToml {
                    nombre: n.clone(),
                    valor: s.valor as i64,
                    etiqueta: s.etiqueta,
                    archivo: s.archivo.clone(),
                    linea: s.linea,
                })
                .collect(),
            mapa: self
                .mapa
                .iter()
                .map(|e| UbicacionToml {
                    direccion: e.direccion,
                    longitud: e.longitud,
                    archivo: e.archivo.clone(),
                    linea: e.linea,
                })
                .collect(),
            externos: self
                .externos
                .iter()
                .map(|(n, e)| ExternoToml {
                    nombre: n.clone(),
                    archivo: e.archivo.clone(),
                    linea: e.linea,
                })
                .collect(),
            reubicaciones: self
                .reubicaciones
                .iter()
                .map(|r| ReubicacionToml {
                    direccion: r.direccion,
                    longitud: r.longitud,
                    archivo: r.archivo.clone(),
                    linea: r.linea,
                    fuente: r.fuente.clone(),
                    simbolos: r.simbolos.clone(),
                })
                .collect(),
        };
        toml::to_string(&archivo).expect("el objeto siempre se puede serializar")
    }

    /// Lee un objeto guardado. Devuelve el nombre de la maquina para la que se ensamblo.
    pub fn desde_toml(texto: &str) -> Result<(String, Objeto), String> {
        let a: Archivo = toml::from_str(texto).map_err(|e| format!("objeto invalido: {e}"))?;
        if a.formato != FORMATO_OBJETO {
            return Err(format!(
                "formato de objeto `{}` no reconocido; este ensamblador escribe `{FORMATO_OBJETO}`",
                a.formato
            ));
        }
        let objeto = Objeto {
            segmentos: a
                .segmentos
                .into_iter()
                .map(|s| Segmento {
                    direccion: s.direccion,
                    palabras: s.palabras,
                })
                .collect(),
            simbolos: a
                .simbolos
                .into_iter()
                .map(|s| {
                    (
                        s.nombre,
                        Simbolo {
                            valor: i128::from(s.valor),
                            etiqueta: s.etiqueta,
                            archivo: s.archivo,
                            linea: s.linea,
                        },
                    )
                })
                .collect(),
            mapa: a
                .mapa
                .into_iter()
                .map(|u| EntradaMapa {
                    direccion: u.direccion,
                    longitud: u.longitud,
                    archivo: u.archivo,
                    linea: u.linea,
                })
                .collect(),
            externos: a
                .externos
                .into_iter()
                .map(|e| {
                    (
                        e.nombre,
                        Externo {
                            archivo: e.archivo,
                            linea: e.linea,
                        },
                    )
                })
                .collect(),
            reubicaciones: a
                .reubicaciones
                .into_iter()
                .map(|r| Reubicacion {
                    direccion: r.direccion,
                    longitud: r.longitud,
                    archivo: r.archivo,
                    linea: r.linea,
                    fuente: r.fuente,
                    simbolos: r.simbolos,
                })
                .collect(),
            avisos: Vec::new(),
        };
        Ok((a.maquina, objeto))
    }
}
