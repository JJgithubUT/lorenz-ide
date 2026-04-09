//! El simulador ciclo a ciclo: el emulador funcional ejecuta y el planificador pone los tiempos
//! (RF-CIC-001, RF-CIC-002, RF-CIC-012 a RF-CIC-014, RF-CIC-025 a RF-CIC-027, RF-CIC-031).

use std::collections::BTreeMap;

use lorenz_emu::{Decodificador, Emulador, ErrorEjecucion, Trafico};

use crate::analisis::dependencias;
use crate::cache::{Cache, EstadisticasCache};
use crate::config::Config;
use crate::pipeline::{Bus, Causa, ConfigPipeline, Entrada, Planificada, Planificador};

/// Que hace una etapa en un ciclo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EstadoEtapa {
    /// Trabajando en la instruccion.
    Trabaja,
    /// La instruccion esta aqui pero esperando (RF-CIC-013).
    Espera(Causa),
    /// Una instruccion de la rama equivocada, que se descarta.
    Descartada,
}

/// Una instruccion dentro de una etapa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ocupante {
    /// Numero de orden de la instruccion; `None` para las descartadas.
    pub id: Option<u64>,
    pub direccion: Option<u64>,
    pub mnemonico: String,
    pub estado: EstadoEtapa,
}

/// El pipeline en un ciclo (RF-CIC-012).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vista {
    pub ciclo: u64,
    /// Una entrada por etapa, en orden; `None` es una burbuja.
    pub etapas: Vec<(String, Option<Ocupante>)>,
}

impl Vista {
    /// Una linea de texto: `ciclo 4 | IF ADD@0x0003 | ID - | EX (espera datos A) ...`.
    pub fn texto(&self) -> String {
        let mut s = format!("ciclo {:>4}", self.ciclo);
        for (nombre, o) in &self.etapas {
            s.push_str(" | ");
            s.push_str(nombre);
            s.push(' ');
            match o {
                None => s.push('-'),
                Some(o) => {
                    match o.direccion {
                        Some(d) => s.push_str(&format!("{}@{d:#06x}", o.mnemonico)),
                        None => s.push_str(&o.mnemonico),
                    }
                    match &o.estado {
                        EstadoEtapa::Trabaja => {}
                        EstadoEtapa::Espera(c) => s.push_str(&format!(" (espera {})", c.texto())),
                        EstadoEtapa::Descartada => s.push_str(" (descartada)"),
                    }
                }
            }
        }
        s
    }
}

/// Cuentas de una ejecucion (RF-CIC-010, RF-CIC-018, RF-CIC-025, RF-CIC-027).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Metricas {
    pub ciclos: u64,
    pub instrucciones: u64,
    /// Ciclos de espera por riesgos de datos, por ocupacion de una etapa y por saltos.
    pub esperas_datos: u64,
    pub esperas_estructurales: u64,
    pub esperas_control: u64,
    /// Ciclos esperando a que el bus compartido quedara libre (RF-CIC-021).
    pub esperas_bus: u64,
    pub saltos: u64,
    pub saltos_acertados: u64,
    /// Instrucciones de la rama equivocada que se buscaron y se descartaron.
    pub descartadas: u64,
    pub cache: Option<EstadisticasCache>,
    /// Ciclos en que la memoria principal estuvo ocupada.
    pub ciclos_de_memoria: u64,
}

impl Metricas {
    /// Ciclos por instruccion, o `None` si no se ejecuto ninguna.
    pub fn cpi(&self) -> Option<f64> {
        (self.instrucciones > 0).then(|| self.ciclos as f64 / self.instrucciones as f64)
    }

    pub fn saltos_fallados(&self) -> u64 {
        self.saltos - self.saltos_acertados
    }

    /// Fraccion de ciclos con la memoria ocupada, entre 0 y 1.
    pub fn uso_del_bus(&self) -> Option<f64> {
        (self.ciclos > 0).then(|| (self.ciclos_de_memoria as f64 / self.ciclos as f64).min(1.0))
    }
}

/// Ciclos atribuidos a una instruccion del programa (RF-CIC-031).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilaPerfil {
    pub direccion: u64,
    pub mnemonico: String,
    pub ejecuciones: u64,
    pub ciclos: u64,
}

pub struct Simulador {
    emu: Emulador,
    cfg: Config,
    planificador: Planificador,
    cache: Option<Cache>,
    decodificador: Decodificador,
    historial: Vec<Planificada>,
    ciclos_de_memoria: u64,
    ciclo: u64,
    /// Accesos a datos de la ultima instruccion ejecutada.
    ultimos_accesos: Vec<Trafico>,
}

impl Simulador {
    /// Prepara la simulacion. Sin `[pipeline]` cada instruccion pasa por una sola etapa con los
    /// ciclos que declara la maquina.
    pub fn nuevo(mut emu: Emulador, cfg: Config) -> Result<Self, String> {
        let pipe = cfg.pipeline.clone().unwrap_or_else(ConfigPipeline::simple);
        let cache = cfg.cache.clone().map(Cache::nueva).transpose()?;
        emu.memoria_mut().anotar_trafico(true);
        let decodificador = Decodificador::nuevo(emu.maquina());
        Ok(Self {
            emu,
            cfg,
            planificador: Planificador::nuevo(pipe),
            cache,
            decodificador,
            historial: Vec::new(),
            ciclos_de_memoria: 0,
            ciclo: 0,
            ultimos_accesos: Vec::new(),
        })
    }

    /// Los accesos a datos que hizo la ultima instruccion ejecutada, en orden.
    pub fn ultimos_accesos(&self) -> &[Trafico] {
        &self.ultimos_accesos
    }

    /// Cambia el bus del planificador por otro (ver `Planificador::intercambiar_bus`).
    pub fn intercambiar_bus(&mut self, otro: &mut Bus) {
        self.planificador.intercambiar_bus(otro);
    }

    pub fn emulador(&self) -> &Emulador {
        &self.emu
    }

    pub fn emulador_mut(&mut self) -> &mut Emulador {
        &mut self.emu
    }

    pub fn config(&self) -> &Config {
        &self.cfg
    }

    pub fn pipeline(&self) -> &ConfigPipeline {
        self.planificador.config()
    }

    pub fn cache(&self) -> Option<&Cache> {
        self.cache.as_ref()
    }

    /// Todo lo ejecutado hasta ahora, con su horario.
    pub fn historial(&self) -> &[Planificada] {
        &self.historial
    }

    /// `true` si la maquina se detuvo.
    pub fn detenida(&self) -> bool {
        self.emu.estado().detenida()
    }

    /// Coste de tocar una palabra: por la cache si la hay y corresponde, o directo a memoria.
    fn coste_de_acceso(&mut self, direccion: u64, escritura: bool, instruccion: bool) -> u64 {
        let usa_cache = self
            .cache
            .as_ref()
            .is_some_and(|c| !instruccion || c.config().instrucciones);
        if !usa_cache {
            let c = self.cfg.latencia_de(direccion);
            self.ciclos_de_memoria += c;
            return c;
        }
        let cache = self.cache.as_mut().expect("comprobado arriba");
        let linea = cache.config().linea;
        let acierto = cache.config().latencia_acierto;
        let r = cache.acceder(direccion, escritura);
        let base = direccion - direccion % linea;
        let linea_entera: u64 = (0..linea).map(|i| self.cfg.latencia_de(base + i)).sum();
        let una = self.cfg.latencia_de(direccion);
        let mut coste = acierto;
        let mut memoria = 0;
        if r.relleno {
            memoria += linea_entera;
        }
        if r.volcado {
            memoria += linea_entera;
        }
        if r.escritura_directa {
            memoria += una;
        }
        self.ciclos_de_memoria += memoria;
        coste = coste
            .max(memoria)
            .max(if memoria > 0 { acierto + memoria } else { 0 });
        coste
    }

    /// Ejecuta una instruccion mas y calcula su horario. `None` si la maquina ya estaba detenida.
    pub fn ejecutar_instruccion(&mut self) -> Result<Option<&Planificada>, ErrorEjecucion> {
        if self.emu.estado().detenida() {
            return Ok(None);
        }
        let pc = self.emu.estado().pc();
        let previa = self.decodificador.decodificar(self.emu.memoria(), pc).ok();
        self.emu.memoria_mut().tomar_trafico();
        let Some(paso) = self.emu.paso()? else {
            return Ok(None);
        };
        let accesos = self.emu.memoria_mut().tomar_trafico();
        self.ultimos_accesos.clone_from(&accesos);
        let dec = match previa {
            Some(d) if d.direccion == paso.direccion => d,
            _ => self
                .decodificador
                .decodificar(self.emu.memoria(), paso.direccion)?,
        };
        let instruccion = &self.emu.maquina().instrucciones[dec.indice];
        let dep = dependencias(self.emu.maquina(), instruccion, &dec.operandos);
        let ciclos_declarados = paso.ciclos;
        let (mut coste_busqueda, mut bus_busqueda) = (0, 0);
        for i in 0..paso.longitud as u64 {
            let antes = self.ciclos_de_memoria;
            coste_busqueda += self.coste_de_acceso(paso.direccion + i, false, true);
            bus_busqueda += self.ciclos_de_memoria - antes;
        }
        let (mut coste_memoria, mut bus_memoria) = (0, 0);
        let mut carga = false;
        for a in &accesos {
            carga |= !a.escritura;
            let antes = self.ciclos_de_memoria;
            coste_memoria += self.coste_de_acceso(a.direccion, a.escritura, false);
            bus_memoria += self.ciclos_de_memoria - antes;
        }
        // Lo que copia un DMA arrancado por esta instruccion: leer y escribir cada palabra.
        let mut bus_dma = 0;
        for t in self.emu.tomar_transferencias_dma() {
            for k in 0..t.palabras {
                bus_dma += self.cfg.latencia_de(t.origen.wrapping_add(k))
                    + self.cfg.latencia_de(t.destino.wrapping_add(k));
            }
        }
        self.ciclos_de_memoria += bus_dma;
        // Sin pipeline declarado, la instruccion tarda lo que declara la maquina mas lo que
        // cuestan de mas sus accesos a memoria: el modo ciclo a ciclo coincide con el funcional
        // cuando todas las memorias tienen latencia 1.
        let (ciclos_declarados, coste_busqueda, coste_memoria) = if self.cfg.pipeline.is_none() {
            let extra_busqueda = coste_busqueda.saturating_sub(paso.longitud as u64);
            let extra_memoria = coste_memoria.saturating_sub(accesos.len() as u64);
            (ciclos_declarados + extra_busqueda + extra_memoria, 0, 0)
        } else {
            (ciclos_declarados, coste_busqueda, coste_memoria)
        };
        let entrada = Entrada {
            direccion: paso.direccion,
            mnemonico: paso.mnemonico.clone(),
            longitud: paso.longitud as u64,
            lee: dep.lee,
            escribe: dep.escribe,
            es_salto: dep.es_salto,
            incondicional: dep.incondicional,
            indirecto: dep.indirecto,
            tomado: paso.salto,
            carga,
            ciclos_declarados,
            coste_busqueda,
            coste_memoria,
            bus_busqueda,
            bus_memoria,
            bus_dma,
        };
        let plan = self.planificador.programar(&entrada);
        self.historial.push(plan);
        Ok(self.historial.last())
    }

    /// Ejecuta hasta que la maquina se detenga o se agote el limite de instrucciones. Devuelve
    /// cuantas ejecuto.
    pub fn ejecutar(&mut self, limite: u64) -> Result<u64, ErrorEjecucion> {
        let mut n = 0;
        while n < limite && self.ejecutar_instruccion()?.is_some() {
            n += 1;
        }
        Ok(n)
    }

    /// Ciclos que lleva la ejecucion hasta ahora.
    pub fn ciclos(&self) -> u64 {
        self.historial.last().map_or(0, Planificada::sale)
    }

    pub fn metricas(&self) -> Metricas {
        let mut m = Metricas {
            ciclos: self.ciclos(),
            instrucciones: self.historial.len() as u64,
            cache: self.cache.as_ref().map(Cache::estadisticas),
            ciclos_de_memoria: self.ciclos_de_memoria,
            ..Default::default()
        };
        for h in &self.historial {
            for (causa, n) in h.esperas() {
                match causa {
                    Causa::Datos { .. } => m.esperas_datos += n,
                    Causa::Estructural => m.esperas_estructurales += n,
                    Causa::Control => m.esperas_control += n,
                    Causa::Bus => m.esperas_bus += n,
                }
            }
            if let Some((_, acierto)) = h.prediccion {
                m.saltos += 1;
                m.saltos_acertados += u64::from(acierto);
            }
            m.descartadas += h.descartadas;
        }
        m
    }

    // --- Vista ciclo a ciclo (RF-CIC-012, RF-CIC-014) ---

    /// Calcula lo necesario para poder mostrar el ciclo dado y devuelve lo que hay en cada etapa.
    pub fn vista(&mut self, ciclo: u64) -> Result<Vista, ErrorEjecucion> {
        loop {
            let basta = match self.historial.last() {
                Some(h) => h.entra[0] > ciclo,
                None => false,
            };
            if basta || self.detenida() || self.ejecutar_instruccion()?.is_none() {
                break;
            }
        }
        let nombres: Vec<String> = self
            .planificador
            .config()
            .etapas
            .iter()
            .map(|e| e.nombre.clone())
            .collect();
        let n = nombres.len();
        let mut etapas: Vec<Option<Ocupante>> = vec![None; n];
        let primera = self.historial.partition_point(|h| h.sale() <= ciclo);
        for h in &self.historial[primera..] {
            if h.entra[0] > ciclo {
                break;
            }
            for (s, slot) in etapas.iter_mut().enumerate() {
                if h.entra[s] <= ciclo && ciclo < h.entra[s + 1] {
                    let esperando = ciclo >= h.entra[s] + h.trabajo[s];
                    *slot = Some(Ocupante {
                        id: Some(h.id),
                        direccion: Some(h.direccion),
                        mnemonico: h.mnemonico.clone(),
                        estado: if esperando {
                            EstadoEtapa::Espera(h.espera[s].clone().unwrap_or(Causa::Estructural))
                        } else {
                            EstadoEtapa::Trabaja
                        },
                    });
                }
            }
        }
        // Las instrucciones que se buscaron por la rama equivocada y se descartan.
        for h in &self.historial {
            if h.descartadas == 0 || h.entra[1] > ciclo || ciclo >= h.entra[1] + 2 * n as u64 {
                continue;
            }
            for j in 0..h.descartadas {
                let inicio = h.entra[1] + j;
                if ciclo < inicio {
                    continue;
                }
                let s = (ciclo - inicio) as usize;
                let resuelto = ciclo < h.entra[self.planificador.config().salto + 1];
                if s < n && resuelto && etapas[s].is_none() {
                    etapas[s] = Some(Ocupante {
                        id: None,
                        direccion: None,
                        mnemonico: "--".into(),
                        estado: EstadoEtapa::Descartada,
                    });
                }
            }
        }
        Ok(Vista {
            ciclo,
            etapas: nombres.into_iter().zip(etapas).collect(),
        })
    }

    /// Ciclo en que esta la vista.
    pub fn ciclo_actual(&self) -> u64 {
        self.ciclo
    }

    /// Avanza un ciclo y devuelve como queda el pipeline (RF-CIC-014).
    pub fn avanzar_ciclo(&mut self) -> Result<Vista, ErrorEjecucion> {
        self.ciclo += 1;
        self.vista(self.ciclo)
    }

    /// Retrocede un ciclo; no vuelve antes del ciclo 0.
    pub fn retroceder_ciclo(&mut self) -> Result<Vista, ErrorEjecucion> {
        self.ciclo = self.ciclo.saturating_sub(1);
        self.vista(self.ciclo)
    }

    /// Diagrama de tiempos: una fila por instruccion y una columna por ciclo (RF-CIC-012).
    pub fn diagrama(&self, desde: usize, cuantas: usize) -> String {
        let filas = &self.historial[desde.min(self.historial.len())..];
        let filas = &filas[..cuantas.min(filas.len())];
        let fin = filas.iter().map(Planificada::sale).max().unwrap_or(0);
        let inicio = filas.first().map_or(0, |h| h.entra[0]);
        let etapas = &self.planificador.config().etapas;
        let mut s = String::new();
        s.push_str(&format!("{:<18}", "instruccion"));
        for c in inicio..fin {
            s.push_str(&format!("{:>3}", c % 100));
        }
        s.push('\n');
        for h in filas {
            s.push_str(&format!(
                "{:<18}",
                format!("{}@{:#06x}", h.mnemonico, h.direccion)
            ));
            for c in inicio..fin {
                let celda = (0..etapas.len())
                    .find(|&k| h.entra[k] <= c && c < h.entra[k + 1])
                    .map(|k| {
                        let letra: String = etapas[k].nombre.chars().take(2).collect();
                        if c >= h.entra[k] + h.trabajo[k] {
                            "..".to_string()
                        } else {
                            letra
                        }
                    })
                    .unwrap_or_default();
                s.push_str(&format!("{celda:>3}"));
            }
            s.push('\n');
        }
        s
    }

    // --- Perfilador (RF-CIC-031) ---

    /// Ciclos atribuidos a cada instruccion del programa: cada una carga con el tiempo que pasa
    /// entre que sale la anterior y sale ella. La suma es el total de ciclos.
    pub fn perfil(&self) -> Vec<FilaPerfil> {
        let mut por_direccion: BTreeMap<u64, FilaPerfil> = BTreeMap::new();
        let mut anterior = 0;
        for h in &self.historial {
            let f = por_direccion.entry(h.direccion).or_insert(FilaPerfil {
                direccion: h.direccion,
                mnemonico: h.mnemonico.clone(),
                ejecuciones: 0,
                ciclos: 0,
            });
            f.ejecuciones += 1;
            f.ciclos += h.sale() - anterior;
            anterior = h.sale();
        }
        por_direccion.into_values().collect()
    }
}
