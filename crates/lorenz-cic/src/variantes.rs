//! Variantes de diseno que cumplan las restricciones (RF-CIC-033).
//!
//! A partir de la temporizacion de una maquina se generan las variantes que resultan de cambiar
//! los parametros que mas pesan en los ciclos (adelantamiento, prediccion de saltos, tamano de la
//! cache, bus compartido), se simula el programa en cada una y se ordenan: primero las que cumplen
//! todas las restricciones, y dentro de ellas las mas rapidas.

use lorenz_emu::Emulador;

use crate::cache::ConfigCache;
use crate::config::Config;
use crate::pipeline::Prediccion;
use crate::restricciones::{comprobar, todas_cumplen, Medidas, Restriccion, Veredicto};
use crate::sim::Simulador;
use crate::sugerir::Sugerencia;

/// Que parametros se hacen variar y entre que valores. Un parametro que no aplica a la maquina
/// (la cache en una maquina sin cache) se deja como esta.
#[derive(Debug, Clone, PartialEq)]
pub struct Espacio {
    pub adelantamiento: bool,
    pub predicciones: Vec<Prediccion>,
    /// Factores por los que se multiplica el tamano de la cache.
    pub factores_cache: Vec<f64>,
    pub bus: bool,
}

impl Default for Espacio {
    fn default() -> Self {
        Self {
            adelantamiento: true,
            predicciones: vec![
                Prediccion::NoTomado,
                Prediccion::Tomado,
                Prediccion::UnBit,
                Prediccion::DosBits,
            ],
            factores_cache: vec![0.5, 1.0, 2.0, 4.0],
            bus: false,
        }
    }
}

/// Una maquina distinta de la base solo en los parametros que dice su nombre.
#[derive(Debug, Clone, PartialEq)]
pub struct Variante {
    pub nombre: String,
    pub config: Config,
}

/// Todas las combinaciones del espacio sobre la base. La primera es siempre la base, llamada
/// `base`; las que dan una configuracion igual a otra ya generada se descartan.
pub fn variantes(base: &Config, espacio: &Espacio) -> Vec<Variante> {
    let mut salida = vec![Variante {
        nombre: "base".into(),
        config: base.clone(),
    }];
    let adelantamientos: Vec<Option<bool>> = if espacio.adelantamiento && base.pipeline.is_some() {
        vec![Some(true), Some(false)]
    } else {
        vec![None]
    };
    let predicciones: Vec<Option<Prediccion>> =
        if !espacio.predicciones.is_empty() && base.pipeline.is_some() {
            espacio.predicciones.iter().map(|p| Some(*p)).collect()
        } else {
            vec![None]
        };
    let factores: Vec<Option<f64>> = if !espacio.factores_cache.is_empty() && base.cache.is_some() {
        espacio.factores_cache.iter().map(|f| Some(*f)).collect()
    } else {
        vec![None]
    };
    let buses: Vec<Option<bool>> = if espacio.bus && base.pipeline.is_some() {
        vec![Some(false), Some(true)]
    } else {
        vec![None]
    };
    for ad in &adelantamientos {
        for pr in &predicciones {
            for fa in &factores {
                for bu in &buses {
                    let mut cfg = base.clone();
                    let mut partes = Vec::new();
                    if let (Some(a), Some(p)) = (ad, cfg.pipeline.as_mut()) {
                        p.adelantamiento = *a;
                        partes.push(format!("adelantamiento={}", if *a { "si" } else { "no" }));
                    }
                    if let (Some(x), Some(p)) = (pr, cfg.pipeline.as_mut()) {
                        p.prediccion = *x;
                        partes.push(format!("prediccion={}", x.nombre()));
                    }
                    if let (Some(f), Some(c)) = (fa, cfg.cache.as_mut()) {
                        let tamano = (c.tamano as f64 * f).round() as u64;
                        let mut nueva: ConfigCache = c.clone();
                        nueva.tamano = tamano;
                        if nueva.validar().is_err() {
                            continue;
                        }
                        *c = nueva;
                        partes.push(format!("cache={tamano}"));
                    }
                    if let (Some(b), Some(p)) = (bu, cfg.pipeline.as_mut()) {
                        p.bus_compartido = *b;
                        partes.push(format!(
                            "bus={}",
                            if *b { "compartido" } else { "separado" }
                        ));
                    }
                    if salida.iter().any(|v| v.config == cfg) {
                        continue;
                    }
                    salida.push(Variante {
                        nombre: partes.join(", "),
                        config: cfg,
                    });
                }
            }
        }
    }
    salida
}

/// Lo que dio simular el programa en una variante.
#[derive(Debug, Clone, PartialEq)]
pub struct Resultado {
    pub nombre: String,
    pub medidas: Medidas,
    pub veredictos: Vec<Veredicto>,
    pub cumple: bool,
}

/// Simula el programa en cada variante y las ordena: las que cumplen primero, y dentro de cada
/// grupo por ciclos (el orden de generacion desempata). `nuevo_emulador` da una maquina con el
/// programa ya cargado.
pub fn evaluar(
    variantes: &[Variante],
    restricciones: &[Restriccion],
    palabras_codigo: u64,
    limite: u64,
    nuevo_emulador: &mut dyn FnMut() -> Result<Emulador, String>,
) -> Result<Vec<Resultado>, String> {
    let mut resultados = Vec::new();
    for v in variantes {
        let mut sim = Simulador::nuevo(nuevo_emulador()?, v.config.clone())?;
        sim.ejecutar(limite)
            .map_err(|e| format!("variante `{}`: fallo durante la ejecucion: {e}", v.nombre))?;
        if !sim.detenida() {
            return Err(format!(
                "variante `{}`: el programa no termino en {limite} instrucciones",
                v.nombre
            ));
        }
        let medidas = Medidas::new(sim.metricas(), palabras_codigo);
        let veredictos = comprobar(restricciones, &medidas);
        let cumple = todas_cumplen(&veredictos);
        resultados.push(Resultado {
            nombre: v.nombre.clone(),
            medidas,
            veredictos,
            cumple,
        });
    }
    resultados.sort_by_key(|r| (!r.cumple, r.medidas.metricas.ciclos));
    Ok(resultados)
}

/// Una variante del repertorio: el diseno actual con una instruccion nueva que fusiona una
/// secuencia que se repite.
#[derive(Debug, Clone, PartialEq)]
pub struct VarianteIsa {
    pub instruccion: String,
    /// Medidas estimadas si la instruccion existiera (no se simulo: ver `sugerir`).
    pub medidas: Medidas,
    pub veredictos: Vec<Veredicto>,
    pub cumple: bool,
}

/// Para cada sugerencia de instruccion nueva, estima las medidas que tendria el programa y si
/// cumpliria las restricciones. Sirve para ver si el repertorio actual es el que no llega, y
/// que instrucciones lo arreglarian. Resta a los ciclos y a las palabras de codigo lo que la
/// sugerencia dice que ahorra, y a las instrucciones ejecutadas las que se fusionan.
pub fn variantes_isa(
    base: &Medidas,
    sugerencias: &[Sugerencia],
    restricciones: &[Restriccion],
) -> Vec<VarianteIsa> {
    let mut v: Vec<VarianteIsa> = sugerencias
        .iter()
        .map(|s| {
            let mut m = base.clone();
            m.metricas.ciclos = m.metricas.ciclos.saturating_sub(s.ciclos_ahorrados);
            m.palabras_codigo = m.palabras_codigo.saturating_sub(s.palabras_ahorradas);
            let quitadas = s.ejecuciones * (s.patron.len() as u64).saturating_sub(1);
            m.metricas.instrucciones = m.metricas.instrucciones.saturating_sub(quitadas);
            let veredictos = comprobar(restricciones, &m);
            VarianteIsa {
                instruccion: s.nombre(),
                cumple: todas_cumplen(&veredictos),
                medidas: m,
                veredictos,
            }
        })
        .collect();
    v.sort_by_key(|x| (!x.cumple, x.medidas.metricas.ciclos));
    v
}
