//! Varios nucleos sobre memoria compartida (RF-CIC-023).
//!
//! Cada nucleo es un emulador con su propio simulador de ciclos: sus registros, su pipeline y, si la
//! descripcion lo pide, su cache. Lo que comparten es lo que la seccion `[nucleos]` declara en
//! `compartida`: cada escritura de un nucleo a una direccion compartida se copia a la memoria de
//! los demas en cuanto termina su instruccion.
//!
//! Los nucleos avanzan de uno en uno. En cada paso ejecuta el que lleva menos tiempo (el que menos
//! ciclos ha consumido; si empatan, el de menor numero), de modo que el orden en que se ven las
//! escrituras es el del tiempo simulado y el resultado se repite. Es una memoria secuencialmente
//! consistente: ningun nucleo ve una escritura antes de que su autor la haya hecho, y todos las ven
//! en el mismo orden. Si el bus es compartido (`[bus]`), los accesos de todos los nucleos se turnan
//! en un mismo bus.
//!
//! Si la descripcion tiene `[coherencia]`, las caches de los nucleos se mantienen coherentes con
//! el protocolo MSI (RF-CIC-024). Las caches de este modelo no guardan datos, solo que lineas
//! tienen y en que estado: lo que la coherencia cambia es el tiempo (una lectura de una linea que
//! otro nucleo tiene modificada espera a que la entregue; una escritura espera a anular las copias
//! ajenas) y los contadores, no los valores, que ya son coherentes por la propagacion de arriba.

use lorenz_emu::{Emulador, ErrorEjecucion};

use std::collections::BTreeMap;

use crate::cache::EstadoLinea;
use crate::config::{Config, ConfigCoherencia};
use crate::pipeline::Bus;
use crate::sim::{Metricas, Simulador};

/// Cifras de una ejecucion de varios nucleos.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MetricasSistema {
    pub nucleos: Vec<Metricas>,
    /// Ciclos hasta que termina el ultimo nucleo.
    pub ciclos: u64,
    /// Instrucciones ejecutadas entre todos.
    pub instrucciones: u64,
    /// Palabras compartidas copiadas de un nucleo a los demas.
    pub propagaciones: u64,
    /// Lineas de cache anuladas por la escritura de otro nucleo, entre todos.
    pub invalidaciones: u64,
    /// Lineas modificadas que un nucleo entrego a otro (por una lectura o una escritura ajena).
    pub cesiones: u64,
    /// Ciclos en que el bus comun estuvo ocupado (0 si cada nucleo tiene el suyo).
    pub ciclos_de_bus_comun: u64,
}

pub struct Sistema {
    nucleos: Vec<Simulador>,
    compartidas: Vec<(u64, u64)>,
    bus_comun: Bus,
    bus_compartido: bool,
    propagaciones: u64,
    coherencia: Option<ConfigCoherencia>,
}

impl Sistema {
    /// Prepara el sistema. Hay que dar un emulador por nucleo, con su programa ya cargado; lo que
    /// haya en las regiones compartidas del primero se copia a los demas, para que todos partan de
    /// la misma memoria comun.
    pub fn nuevo(emuladores: Vec<Emulador>, cfg: Config) -> Result<Self, String> {
        let esperados = cfg.nucleos.as_ref().map_or(1, |n| n.cantidad);
        if emuladores.len() != esperados {
            return Err(format!(
                "la maquina declara {esperados} nucleo{} y se dieron {} emulador{}",
                if esperados == 1 { "" } else { "s" },
                emuladores.len(),
                if emuladores.len() == 1 { "" } else { "es" },
            ));
        }
        let compartidas: Vec<(u64, u64)> = cfg.nucleos.as_ref().map_or_else(Vec::new, |n| {
            n.compartidas.iter().map(|r| (r.desde, r.hasta)).collect()
        });
        let bus_compartido = cfg.pipeline.as_ref().is_some_and(|p| p.bus_compartido);
        let mut emuladores = emuladores;
        if let Some((primero, resto)) = emuladores.split_first_mut() {
            for &(desde, hasta) in &compartidas {
                for d in desde..=hasta {
                    let v = primero
                        .memoria()
                        .leer_privilegiado(d)
                        .map_err(|e| e.to_string())?;
                    for otro in resto.iter_mut() {
                        otro.memoria_mut()
                            .escribir_privilegiado(d, v)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
        }
        let nucleos = emuladores
            .into_iter()
            .map(|e| Simulador::nuevo(e, cfg.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            nucleos,
            compartidas,
            bus_comun: Bus::default(),
            bus_compartido,
            propagaciones: 0,
            coherencia: cfg.coherencia.clone(),
        })
    }

    pub fn numero_de_nucleos(&self) -> usize {
        self.nucleos.len()
    }

    pub fn nucleo(&self, i: usize) -> &Simulador {
        &self.nucleos[i]
    }

    pub fn nucleo_mut(&mut self, i: usize) -> &mut Simulador {
        &mut self.nucleos[i]
    }

    fn es_compartida(&self, direccion: u64) -> bool {
        self.compartidas
            .iter()
            .any(|&(a, b)| (a..=b).contains(&direccion))
    }

    /// Si todos los nucleos se han detenido.
    pub fn detenido(&self) -> bool {
        self.nucleos.iter().all(Simulador::detenida)
    }

    /// El nucleo que toca ejecutar: el que menos ciclos lleva entre los que siguen en marcha.
    fn siguiente(&self) -> Option<usize> {
        self.nucleos
            .iter()
            .enumerate()
            .filter(|(_, n)| !n.detenida())
            .min_by_key(|(i, n)| (n.ciclos(), *i))
            .map(|(i, _)| i)
    }

    /// Ejecuta una instruccion en el nucleo que le toca y devuelve cual fue; `None` si todos
    /// estaban detenidos.
    pub fn ejecutar_instruccion(&mut self) -> Result<Option<usize>, ErrorEjecucion> {
        let Some(i) = self.siguiente() else {
            return Ok(None);
        };
        if self.bus_compartido {
            self.nucleos[i].intercambiar_bus(&mut self.bus_comun);
        }
        let r = self.nucleos[i].ejecutar_instruccion().map(|p| p.is_some());
        if self.bus_compartido {
            self.nucleos[i].intercambiar_bus(&mut self.bus_comun);
        }
        if !r? {
            return Ok(None);
        }
        self.propagar(i)?;
        self.espiar(i);
        Ok(Some(i))
    }

    /// Mantiene coherentes las caches tras una instruccion de `autor` (RF-CIC-024): una escritura
    /// anula la linea en las demas caches (y si estaba modificada la entrega antes), una lectura
    /// hace que quien la tiene modificada la entregue y quede compartida. El autor espera lo que
    /// cuesta: la latencia de invalidacion por linea anulada y volcar a memoria cada linea
    /// entregada.
    fn espiar(&mut self, autor: usize) {
        let Some(coh) = self.coherencia.clone() else {
            return;
        };
        let Some(linea) = self.nucleos[autor].cache_mut().map(|c| c.config().linea) else {
            return;
        };
        // Lineas compartidas que toco, y si en alguna escribio.
        let mut lineas: BTreeMap<u64, bool> = BTreeMap::new();
        for a in self.nucleos[autor].ultimos_accesos() {
            if self.es_compartida(a.direccion) {
                *lineas.entry(a.direccion - a.direccion % linea).or_default() |= a.escritura;
            }
        }
        let mut espera = 0;
        for (base, escribe) in lineas {
            let volcado: u64 = (0..linea)
                .map(|k| self.nucleos[autor].config().latencia_de(base + k))
                .sum();
            for j in (0..self.nucleos.len()).filter(|&j| j != autor) {
                let Some(otra) = self.nucleos[j].cache_mut() else {
                    continue;
                };
                if escribe {
                    match otra.invalidar(base) {
                        Some(EstadoLinea::Modificada) => {
                            espera += coh.latencia_invalidacion + volcado;
                        }
                        Some(EstadoLinea::Compartida) => espera += coh.latencia_invalidacion,
                        None => {}
                    }
                } else if otra.ceder(base) {
                    espera += volcado;
                }
            }
        }
        self.nucleos[autor].retrasar(espera);
    }

    /// Comprueba el invariante de MSI en las lineas compartidas: si una cache tiene una linea
    /// modificada, ninguna otra la tiene. Devuelve la primera violacion.
    pub fn comprobar_coherencia(&self) -> Result<(), String> {
        let mut lineas: BTreeMap<u64, (usize, usize)> = BTreeMap::new();
        for n in &self.nucleos {
            let Some(c) = n.cache() else { continue };
            for f in c.contenido() {
                let Some(d) = f.direccion else { continue };
                if !self.es_compartida(d) {
                    continue;
                }
                let e = lineas.entry(d).or_default();
                e.0 += 1;
                e.1 += usize::from(f.sucia);
            }
        }
        for (d, (copias, modificadas)) in lineas {
            if modificadas > 1 || (modificadas == 1 && copias > 1) {
                return Err(format!(
                    "la linea {d:#x} esta modificada en una cache y presente en {} mas",
                    copias - 1
                ));
            }
        }
        Ok(())
    }

    /// Copia a los demas nucleos lo que `autor` escribio en memoria compartida.
    fn propagar(&mut self, autor: usize) -> Result<(), ErrorEjecucion> {
        let escritas: Vec<u64> = self.nucleos[autor]
            .ultimos_accesos()
            .iter()
            .filter(|a| a.escritura && self.es_compartida(a.direccion))
            .map(|a| a.direccion)
            .collect();
        for d in escritas {
            let v = self.nucleos[autor]
                .emulador()
                .memoria()
                .leer_privilegiado(d)?;
            for (j, n) in self.nucleos.iter_mut().enumerate() {
                if j != autor {
                    n.emulador_mut().memoria_mut().escribir_privilegiado(d, v)?;
                }
            }
            self.propagaciones += 1;
        }
        Ok(())
    }

    /// Ejecuta hasta que todos se detengan o se agoten `limite` instrucciones entre todos.
    /// Devuelve cuantas se ejecutaron.
    pub fn ejecutar(&mut self, limite: u64) -> Result<u64, ErrorEjecucion> {
        let mut n = 0;
        while n < limite && self.ejecutar_instruccion()?.is_some() {
            n += 1;
        }
        Ok(n)
    }

    pub fn metricas(&self) -> MetricasSistema {
        let nucleos: Vec<Metricas> = self.nucleos.iter().map(Simulador::metricas).collect();
        MetricasSistema {
            ciclos: nucleos.iter().map(|m| m.ciclos).max().unwrap_or(0),
            instrucciones: nucleos.iter().map(|m| m.instrucciones).sum(),
            propagaciones: self.propagaciones,
            invalidaciones: nucleos
                .iter()
                .filter_map(|m| m.cache.as_ref())
                .map(|c| c.invalidaciones)
                .sum(),
            cesiones: nucleos
                .iter()
                .filter_map(|m| m.cache.as_ref())
                .map(|c| c.cesiones)
                .sum(),
            ciclos_de_bus_comun: self.bus_comun.ciclos_ocupado(),
            nucleos,
        }
    }
}
