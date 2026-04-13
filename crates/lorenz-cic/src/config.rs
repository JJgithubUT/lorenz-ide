//! Lectura de la temporizacion declarada en la descripcion de arquitectura.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::cache::{ConfigCache, Escritura, Reemplazo};
use crate::pipeline::{ConfigPipeline, Etapa, Prediccion};
use crate::restricciones::{Metrica, Restriccion};

/// Latencia de acceso de una region de memoria (RF-CIC-020).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatenciaRegion {
    pub region: String,
    pub desde: u64,
    pub hasta: u64,
    pub ciclos: u64,
}

/// Una region de memoria que comparten todos los nucleos (RF-CIC-023).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionCompartida {
    pub region: String,
    pub desde: u64,
    pub hasta: u64,
}

/// Varios nucleos sobre memoria compartida (RF-CIC-023).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigNucleos {
    pub cantidad: usize,
    pub compartidas: Vec<RegionCompartida>,
}

impl ConfigNucleos {
    /// Si la direccion cae en una region compartida.
    pub fn es_compartida(&self, direccion: u64) -> bool {
        self.compartidas
            .iter()
            .any(|r| (r.desde..=r.hasta).contains(&direccion))
    }
}

/// Protocolo de coherencia entre las caches de los nucleos (RF-CIC-024).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocolo {
    /// Modificada, compartida e invalida; una escritura anula las copias de los demas.
    Msi,
}

impl Protocolo {
    pub fn nombre(self) -> &'static str {
        match self {
            Protocolo::Msi => "msi",
        }
    }
}

/// Como se mantienen coherentes las caches de varios nucleos (RF-CIC-024).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigCoherencia {
    pub protocolo: Protocolo,
    /// Ciclos que se detiene el nucleo que escribe por cada linea que anula en otra cache.
    pub latencia_invalidacion: u64,
}

/// Todo lo que la descripcion dice sobre tiempos.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Config {
    pub pipeline: Option<ConfigPipeline>,
    pub cache: Option<ConfigCache>,
    pub latencias: Vec<LatenciaRegion>,
    /// Limites de diseno que debe cumplir la maquina (RF-CIC-033).
    pub restricciones: Vec<Restriccion>,
    /// Mas de un nucleo sobre memoria compartida; `None` es una maquina de un solo nucleo.
    pub nucleos: Option<ConfigNucleos>,
    /// Coherencia entre las caches de los nucleos; `None` es sin espionaje.
    pub coherencia: Option<ConfigCoherencia>,
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
    nucleos: Option<NucleosCrudos>,
    coherencia: Option<CoherenciaCruda>,
    #[serde(default, rename = "restriccion")]
    restricciones: Vec<RestriccionCruda>,
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

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(untagged)]
enum Numero {
    Entero(i64),
    Real(f64),
}

impl Numero {
    fn valor(self) -> f64 {
        match self {
            Numero::Entero(n) => n as f64,
            Numero::Real(x) => x,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RestriccionCruda {
    #[serde(default)]
    nombre: Option<String>,
    metrica: String,
    #[serde(default)]
    maximo: Option<Numero>,
    #[serde(default)]
    minimo: Option<Numero>,
}

/// Mas nucleos de los que tiene sentido simular; evita que un error de tipeo pida un millon.
pub const MAX_NUCLEOS: usize = 64;

#[derive(Debug, Deserialize)]
struct NucleosCrudos {
    cantidad: usize,
    #[serde(default)]
    compartida: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CoherenciaCruda {
    #[serde(default)]
    protocolo: Option<String>,
    #[serde(default = "una_via")]
    latencia_invalidacion: u64,
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
    if let Some(n) = &a.nucleos {
        if n.cantidad == 0 || n.cantidad > MAX_NUCLEOS {
            return Err(format!(
                "nucleos.cantidad debe estar entre 1 y {MAX_NUCLEOS}, no {}",
                n.cantidad
            ));
        }
        let mut compartidas = Vec::new();
        for nombre in &n.compartida {
            let r = a
                .regiones
                .iter()
                .find(|r| &r.nombre == nombre)
                .ok_or_else(|| {
                    format!("nucleos.compartida: no hay una region llamada `{nombre}`")
                })?;
            compartidas.push(RegionCompartida {
                region: nombre.clone(),
                desde: r.desde,
                hasta: r.hasta,
            });
        }
        if n.cantidad > 1 && compartidas.is_empty() {
            return Err(
                "nucleos: con mas de un nucleo hace falta al menos una region en `compartida`"
                    .into(),
            );
        }
        cfg.nucleos = Some(ConfigNucleos {
            cantidad: n.cantidad,
            compartidas,
        });
    }
    if let Some(c) = &a.coherencia {
        let protocolo = match c.protocolo.as_deref().unwrap_or("msi") {
            "msi" => Protocolo::Msi,
            o => return Err(format!("coherencia.protocolo `{o}` no existe; usa msi")),
        };
        if a.cache.is_none() {
            return Err("coherencia: necesita una [cache] que mantener coherente".into());
        }
        cfg.coherencia = Some(ConfigCoherencia {
            protocolo,
            latencia_invalidacion: c.latencia_invalidacion,
        });
    }
    for r in &a.restricciones {
        let metrica = Metrica::desde_nombre(&r.metrica).map_err(|e| format!("restriccion: {e}"))?;
        let restriccion = Restriccion {
            nombre: r.nombre.clone().unwrap_or_else(|| metrica.nombre().into()),
            metrica,
            minimo: r.minimo.map(Numero::valor),
            maximo: r.maximo.map(Numero::valor),
        };
        restriccion.validar()?;
        cfg.restricciones.push(restriccion);
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
