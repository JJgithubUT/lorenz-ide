//! La seccion `[gpu]` de la descripcion de la maquina (RF-GPU-001, RF-GPU-002, RF-GPU-011).
//!
//! Como `[pipeline]` o `[nucleos]`, la seccion se ignora fuera de este crate: el emulador
//! funcional no sabe que hay una unidad grafica.

use serde::Deserialize;

use crate::error::ErrorGpu;
use crate::superficie::{ConfigSuperficie, ProfundidadColor};

/// Cuanto cuesta cada cosa a la unidad grafica, en ciclos propios (RF-GPU-017).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Costes {
    /// Cada comando, sea cual sea.
    pub comando: u64,
    /// Cada pixel que una primitiva escribe.
    pub pixel: u64,
    /// Preparar un triangulo (calcular aristas y pesos).
    pub triangulo: u64,
}

impl Default for Costes {
    fn default() -> Self {
        Self {
            comando: 4,
            pixel: 1,
            triangulo: 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigGpu {
    /// Nombre del dispositivo de la maquina al que corresponde, si la CPU la maneja.
    pub dispositivo: Option<String>,
    pub superficie: ConfigSuperficie,
    /// Cuantos comandos caben esperando (RF-GPU-011).
    pub cola: usize,
    pub costes: Costes,
    /// Linea de interrupcion que se levanta al terminar un lote (RF-GPU-012).
    pub interrupcion: Option<u32>,
    /// Ciclos de la unidad por cada ciclo de la CPU (su reloj relativo).
    pub razon_reloj: u64,
}

impl Default for ConfigGpu {
    fn default() -> Self {
        Self {
            dispositivo: None,
            superficie: ConfigSuperficie {
                ancho: 64,
                alto: 48,
                color: ProfundidadColor::Bits24,
                profundidad: false,
            },
            cola: 16,
            costes: Costes::default(),
            interrupcion: None,
            razon_reloj: 1,
        }
    }
}

#[derive(Deserialize, Default)]
struct Archivo {
    gpu: Option<Cruda>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Cruda {
    dispositivo: Option<String>,
    ancho: Option<u32>,
    alto: Option<u32>,
    color: Option<u32>,
    profundidad: Option<bool>,
    cola: Option<usize>,
    interrupcion: Option<u32>,
    ciclos_comando: Option<u64>,
    ciclos_pixel: Option<u64>,
    ciclos_triangulo: Option<u64>,
    razon_reloj: Option<u64>,
}

/// Lado maximo de la superficie: evita que un error de tipeo pida gigabytes.
pub const LADO_MAXIMO: u32 = 4096;
pub const COLA_MAXIMA: usize = 65536;

/// Lee `[gpu]` de un texto TOML. Devuelve `None` si la descripcion no declara unidad grafica
/// (RF-GPU-021: sin `[gpu]` no hay nada que simular ni se paga por ello).
pub fn cargar_config(texto: &str) -> Result<Option<ConfigGpu>, ErrorGpu> {
    let a: Archivo = toml::from_str(texto).map_err(|e| ErrorGpu::Config(e.to_string()))?;
    let Some(c) = a.gpu else { return Ok(None) };
    let d = ConfigGpu::default();
    let ancho = c.ancho.unwrap_or(d.superficie.ancho);
    let alto = c.alto.unwrap_or(d.superficie.alto);
    if !(1..=LADO_MAXIMO).contains(&ancho) || !(1..=LADO_MAXIMO).contains(&alto) {
        return Err(ErrorGpu::Config(format!(
            "gpu: la resolucion debe estar entre 1x1 y {LADO_MAXIMO}x{LADO_MAXIMO}, no {ancho}x{alto}"
        )));
    }
    let bits = c.color.unwrap_or(24);
    let color = ProfundidadColor::desde_bits(bits).ok_or_else(|| {
        ErrorGpu::Config(format!("gpu: `color` debe ser 8, 16 o 24 bits, no {bits}"))
    })?;
    let cola = c.cola.unwrap_or(d.cola);
    if !(1..=COLA_MAXIMA).contains(&cola) {
        return Err(ErrorGpu::Config(format!(
            "gpu: `cola` debe estar entre 1 y {COLA_MAXIMA}, no {cola}"
        )));
    }
    let costes = Costes {
        comando: c.ciclos_comando.unwrap_or(d.costes.comando),
        pixel: c.ciclos_pixel.unwrap_or(d.costes.pixel),
        triangulo: c.ciclos_triangulo.unwrap_or(d.costes.triangulo),
    };
    if costes.comando == 0 {
        return Err(ErrorGpu::Config(
            "gpu: `ciclos_comando` debe ser al menos 1".into(),
        ));
    }
    let razon_reloj = c.razon_reloj.unwrap_or(1);
    if !(1..=1024).contains(&razon_reloj) {
        return Err(ErrorGpu::Config(format!(
            "gpu: `razon_reloj` debe estar entre 1 y 1024, no {razon_reloj}"
        )));
    }
    Ok(Some(ConfigGpu {
        dispositivo: c.dispositivo,
        superficie: ConfigSuperficie {
            ancho,
            alto,
            color,
            profundidad: c.profundidad.unwrap_or(false),
        },
        cola,
        costes,
        interrupcion: c.interrupcion,
        razon_reloj,
    }))
}

pub fn cargar_config_archivo(
    ruta: impl AsRef<std::path::Path>,
) -> Result<Option<ConfigGpu>, ErrorGpu> {
    let t = std::fs::read_to_string(ruta.as_ref())
        .map_err(|e| ErrorGpu::Config(format!("{}: {e}", ruta.as_ref().display())))?;
    cargar_config(&t)
}
