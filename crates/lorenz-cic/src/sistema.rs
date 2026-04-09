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

use lorenz_emu::{Emulador, ErrorEjecucion};

use crate::config::Config;
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
    /// Ciclos en que el bus comun estuvo ocupado (0 si cada nucleo tiene el suyo).
    pub ciclos_de_bus_comun: u64,
}

pub struct Sistema {
    nucleos: Vec<Simulador>,
    compartidas: Vec<(u64, u64)>,
    bus_comun: Bus,
    bus_compartido: bool,
    propagaciones: u64,
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
        Ok(Some(i))
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
            ciclos_de_bus_comun: self.bus_comun.ciclos_ocupado(),
            nucleos,
        }
    }
}
