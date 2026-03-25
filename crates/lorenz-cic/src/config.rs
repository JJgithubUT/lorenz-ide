//! Lectura de la temporizacion declarada en la descripcion de arquitectura.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::cache::{ConfigCache, Escritura, Reemplazo};
use crate::pipeline::{ConfigPipeline, Etapa, Prediccion};

/// Latencia de acceso de una region de memoria (RF-CIC-020).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatenciaRegion {
    pub region: String,
    pub desde: u64,
    pub hasta: u64,
    pub ciclos: u64,
}

/// Todo lo que la descripcion dice sobre tiempos.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Config {
    pub pipeline: Option<ConfigPipeline>,
    pub cache: Option<ConfigCache>,
    pub latencias: Vec<LatenciaRegion>,
}

impl Config {
    /// Ciclos que cuesta tocar una palabra de la direccion dada; 1 si la region no declara nada.
    pub fn latencia_de(&self, direccion: u64) -> u64 {
        self.latencias
            .iter()
            .find(|l| (l.desde..=l.hasta).contains(&direccion))
            .map_or(1, |l| l.ciclos)
    }
}

#[derive(Debug, Deserialize, Default)]
struct Archivo {
    pipeline: Option<PipelineCrudo>,
    cache: Option<CacheCruda>,
    bus: Option<BusCrudo>,
    #[serde(default, rename = "region")]
    regiones: Vec<RegionCruda>,
}

#[derive(Debug, Deserialize)]
struct PipelineCrudo {
    #[serde(default)]
    etapas: Vec<String>,
    #[serde(default, rename = "etapa")]
    detalle: Vec<EtapaCruda>,
    #[serde(default = "verdadero")]
    adelantamiento: bool,
    #[serde(default)]
    prediccion: Option<String>,
    #[serde(default)]
    lectura: Option<String>,
    #[serde(default)]
    ejecucion: Option<String>,
    #[serde(default)]
    memoria: Option<String>,
    #[serde(default)]
    escritura: Option<String>,
    #[serde(default)]
    salto: Option<String>,
    #[serde(default)]
    ciclos_declarados: bool,
    #[serde(default)]
    ciclos_objetivo: u64,
}

#[derive(Debug, Deserialize)]
struct BusCrudo {
    #[serde(default)]
    compartido: bool,
}

#[derive(Debug, Deserialize)]
struct EtapaCruda {
    nombre: String,
    #[serde(default)]
    hace: Vec<String>,
}

fn verdadero() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct CacheCruda {
    tamano: u64,
    linea: u64,
    #[serde(default = "una_via")]
    vias: u64,
    #[serde(default)]
    reemplazo: Option<String>,
    #[serde(default)]
    escritura: Option<String>,
    #[serde(default = "una_via")]
    latencia_acierto: u64,
    #[serde(default)]
    instrucciones: bool,
}

fn una_via() -> u64 {
    1
}

#[derive(Debug, Deserialize)]
struct RegionCruda {
    nombre: String,
    desde: u64,
    hasta: u64,
    #[serde(default)]
    latencia: Option<u64>,
}

/// Lee la temporizacion de una descripcion en TOML. Un archivo sin secciones de tiempo da una
/// configuracion vacia: el modelo ciclo a ciclo funciona igual, con un pipeline de una etapa.
pub fn cargar_config(texto: &str) -> Result<Config, String> {
    let a: Archivo = toml::from_str(texto).map_err(|e| e.to_string())?;
    desde_archivo(a)
}

/// Como [`cargar_config`], pero desde un archivo y siguiendo su herencia: un hijo que no declara
/// `[pipeline]` usa el de su padre, y puede redefinir una region para cambiarle la latencia.
pub fn cargar_config_archivo(ruta: impl AsRef<std::path::Path>) -> Result<Config, String> {
    let tabla = lorenz_arch::mezclar_archivo(ruta).map_err(|e| e.to_string())?;
    let a: Archivo = toml::Value::Table(tabla)
        .try_into()
        .map_err(|e: toml::de::Error| e.to_string())?;
    desde_archivo(a)
}

fn desde_archivo(a: Archivo) -> Result<Config, String> {
    let mut cfg = Config::default();
    for r in &a.regiones {
        if let Some(l) = r.latencia {
            if l == 0 {
                return Err(format!(
                    "region `{}`: la latencia debe ser al menos 1 ciclo",
                    r.nombre
                ));
            }
            cfg.latencias.push(LatenciaRegion {
                region: r.nombre.clone(),
                desde: r.desde,
                hasta: r.hasta,
                ciclos: l,
            });
        }
    }
    if let Some(c) = a.cache {
        let reemplazo = match c.reemplazo.as_deref().unwrap_or("lru") {
            "lru" => Reemplazo::Lru,
            "fifo" => Reemplazo::Fifo,
            "aleatorio" => Reemplazo::Aleatorio,
            o => {
                return Err(format!(
                    "cache.reemplazo `{o}` no existe; usa lru, fifo o aleatorio"
                ))
            }
        };
        let escritura = match c.escritura.as_deref().unwrap_or("post-escritura") {
            "post-escritura" => Escritura::PostEscritura,
            "escritura-directa" => Escritura::EscrituraDirecta,
            o => {
                return Err(format!(
                    "cache.escritura `{o}` no existe; usa post-escritura o escritura-directa"
                ))
            }
        };
        let cfg_cache = ConfigCache {
            tamano: c.tamano,
            linea: c.linea,
            vias: c.vias,
            reemplazo,
            escritura,
            latencia_acierto: c.latencia_acierto,
            instrucciones: c.instrucciones,
        };
        cfg_cache.validar().map_err(|e| format!("cache: {e}"))?;
        cfg.cache = Some(cfg_cache);
    }
    if let Some(p) = a.pipeline {
        let mut etapas: Vec<Etapa> = if p.detalle.is_empty() {
            p.etapas
                .iter()
                .map(|n| Etapa {
                    nombre: n.clone(),
                    hace: Vec::new(),
                })
                .collect()
        } else {
            p.detalle
                .iter()
                .map(|e| Etapa {
                    nombre: e.nombre.clone(),
                    hace: e.hace.clone(),
                })
                .collect()
        };
        if etapas.is_empty() {
            etapas.push(Etapa {
                nombre: "EJ".into(),
                hace: Vec::new(),
            });
        }
        let prediccion = match p.prediccion.as_deref() {
            None => Prediccion::NoTomado,
            Some(n) => Prediccion::desde_nombre(n)?,
        };
        let nombres: BTreeMap<&str, usize> = etapas
            .iter()
            .enumerate()
            .map(|(i, e)| (e.nombre.as_str(), i))
            .collect();
        if nombres.len() != etapas.len() {
            return Err("pipeline: hay dos etapas con el mismo nombre".into());
        }
        let n = etapas.len();
        let buscar =
            |clave: &str, valor: &Option<String>, defecto: usize| -> Result<usize, String> {
                match valor {
                    None => Ok(defecto),
                    Some(v) => nombres
                        .get(v.as_str())
                        .copied()
                        .ok_or_else(|| format!("pipeline.{clave}: no hay una etapa llamada `{v}`")),
                }
            };
        let escritura_i = buscar("escritura", &p.escritura, n - 1)?;
        let memoria_i = buscar("memoria", &p.memoria, n.saturating_sub(2))?;
        let ejecucion_i = buscar("ejecucion", &p.ejecucion, n.saturating_sub(3))?;
        let lectura_i = buscar("lectura", &p.lectura, n.saturating_sub(4))?;
        let salto_i = buscar("salto", &p.salto, ejecucion_i)?;
        cfg.pipeline = Some(ConfigPipeline {
            etapas,
            adelantamiento: p.adelantamiento,
            prediccion,
            lectura: lectura_i,
            ejecucion: ejecucion_i,
            memoria: memoria_i,
            escritura: escritura_i,
            salto: salto_i,
            ciclos_declarados: p.ciclos_declarados,
            ciclos_objetivo: p.ciclos_objetivo,
            bus_compartido: a.bus.as_ref().is_some_and(|b| b.compartido),
        });
    } else if a.bus.is_some_and(|b| b.compartido) {
        return Err("bus: un bus compartido necesita un [pipeline] declarado".into());
    }
    Ok(cfg)
}
