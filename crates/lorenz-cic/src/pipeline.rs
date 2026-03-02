//! Pipeline en orden: etapas, riesgos de datos, riesgos de control y prediccion de saltos
//! (RF-CIC-003 a RF-CIC-011).
//!
//! El modelo se alimenta del emulador funcional: este dice que instrucciones se ejecutan, con que
//! registros y a que direcciones acceden, y el planificador calcula en que ciclo pasa cada una por
//! cada etapa. Como la ejecucion es en orden, el horario de una instruccion solo depende de las
//! anteriores y puede calcularse sin rehacer nada.

use std::collections::BTreeMap;

/// Una etapa con las microoperaciones que realiza (RF-CIC-004).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Etapa {
    pub nombre: String,
    pub hace: Vec<String>,
}

/// Politica de prediccion de saltos (RF-CIC-009).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prediccion {
    /// Se supone siempre que no se salta.
    NoTomado,
    /// Se supone siempre que se salta.
    Tomado,
    /// Un bit por salto: se repite lo ultimo que hizo.
    UnBit,
    /// Contador de dos bits por salto.
    DosBits,
}

impl Prediccion {
    pub fn desde_nombre(n: &str) -> Result<Self, String> {
        match n {
            "no-tomado" => Ok(Prediccion::NoTomado),
            "tomado" => Ok(Prediccion::Tomado),
            "1-bit" => Ok(Prediccion::UnBit),
            "2-bits" => Ok(Prediccion::DosBits),
            o => Err(format!(
                "pipeline.prediccion `{o}` no existe; usa no-tomado, tomado, 1-bit o 2-bits"
            )),
        }
    }

    pub fn nombre(self) -> &'static str {
        match self {
            Prediccion::NoTomado => "no-tomado",
            Prediccion::Tomado => "tomado",
            Prediccion::UnBit => "1-bit",
            Prediccion::DosBits => "2-bits",
        }
    }
}

/// Como es el pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigPipeline {
    pub etapas: Vec<Etapa>,
    /// Si los resultados pasan de una etapa a otra sin esperar a escribirse (RF-CIC-007).
    pub adelantamiento: bool,
    pub prediccion: Prediccion,
    /// Indices de las etapas con cada papel.
    pub lectura: usize,
    pub ejecucion: usize,
    pub memoria: usize,
    pub escritura: usize,
    /// Etapa donde se sabe si un salto se toma.
    pub salto: usize,
    /// Si la etapa de ejecucion dura los ciclos que declara cada instruccion.
    pub ciclos_declarados: bool,
    /// Ciclos extra al saltar cuando se acierta que se salta (calcular el destino).
    pub ciclos_objetivo: u64,
}

impl ConfigPipeline {
    /// Un solo paso por instruccion, con los ciclos que declara la maquina: lo que hace el modo
    /// funcional si no se declara nada.
    pub fn simple() -> Self {
        Self {
            etapas: vec![Etapa {
                nombre: "EJ".into(),
                hace: Vec::new(),
            }],
            adelantamiento: true,
            prediccion: Prediccion::NoTomado,
            lectura: 0,
            ejecucion: 0,
            memoria: 0,
            escritura: 0,
            salto: 0,
            ciclos_declarados: true,
            ciclos_objetivo: 0,
        }
    }
}

/// Por que una instruccion espero (RF-CIC-006, RF-CIC-013).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Causa {
    /// Esperaba el valor de un registro o bandera que produce otra instruccion.
    Datos { registro: String },
    /// La etapa siguiente estaba ocupada.
    Estructural,
    /// Esperaba a saber adonde va un salto.
    Control,
}

impl Causa {
    pub fn texto(&self) -> String {
        match self {
            Causa::Datos { registro } => format!("datos ({registro})"),
            Causa::Estructural => "estructural".into(),
            Causa::Control => "control".into(),
        }
    }
}

/// Predictor de saltos con estado.
#[derive(Debug, Clone)]
pub struct Predictor {
    politica: Prediccion,
    tabla: BTreeMap<u64, u8>,
}

impl Predictor {
    pub fn nuevo(politica: Prediccion) -> Self {
        Self {
            politica,
            tabla: BTreeMap::new(),
        }
    }

    /// Si se cree que el salto en `direccion` se tomara.
    pub fn predecir(&self, direccion: u64) -> bool {
        match self.politica {
            Prediccion::NoTomado => false,
            Prediccion::Tomado => true,
            Prediccion::UnBit => self.tabla.get(&direccion).copied().unwrap_or(0) != 0,
            Prediccion::DosBits => self.tabla.get(&direccion).copied().unwrap_or(1) >= 2,
        }
    }

    pub fn aprender(&mut self, direccion: u64, tomado: bool) {
        match self.politica {
            Prediccion::UnBit => {
                self.tabla.insert(direccion, u8::from(tomado));
            }
            Prediccion::DosBits => {
                let c = self.tabla.entry(direccion).or_insert(1);
                *c = if tomado {
                    (*c + 1).min(3)
                } else {
                    c.saturating_sub(1)
                };
            }
            _ => {}
        }
    }
}

/// Lo que el planificador necesita saber de una instruccion ya ejecutada.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Entrada {
    pub direccion: u64,
    pub mnemonico: String,
    pub longitud: u64,
    /// Registros y banderas que lee, y que escribe.
    pub lee: Vec<String>,
    pub escribe: Vec<String>,
    /// Escribe el contador de programa.
    pub es_salto: bool,
    /// Lo escribe sin condicion.
    pub incondicional: bool,
    /// El destino sale de un registro o de memoria: no se conoce hasta ejecutarla.
    pub indirecto: bool,
    pub tomado: bool,
    /// Lee de memoria de datos; su resultado llega al final de la etapa de memoria.
    pub carga: bool,
    pub ciclos_declarados: u64,
    /// Ciclos de la busqueda de la instruccion.
    pub coste_busqueda: u64,
    /// Ciclos de sus accesos a datos, ya con cache y latencias.
    pub coste_memoria: u64,
}

/// El horario de una instruccion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planificada {
    pub id: u64,
    pub direccion: u64,
    pub mnemonico: String,
    /// Ciclo en que entra a cada etapa; el ultimo valor es el ciclo en que sale de la ultima.
    pub entra: Vec<u64>,
    /// Ciclos de trabajo de cada etapa (el resto del tiempo esta esperando).
    pub trabajo: Vec<u64>,
    /// Por que espero en cada etapa, si espero.
    pub espera: Vec<Option<Causa>>,
    /// Ciclos que esperó antes de entrar al pipeline (un salto sin resolver) y por qué.
    pub previa: Option<(Causa, u64)>,
    /// Si es un salto: lo que se predijo y si se acerto.
    pub prediccion: Option<(bool, bool)>,
    /// Instrucciones de la rama equivocada que se buscaron y hubo que descartar.
    pub descartadas: u64,
}

impl Planificada {
    /// Ciclo en que sale del pipeline.
    pub fn sale(&self) -> u64 {
        *self.entra.last().unwrap_or(&0)
    }

    /// Ciclos de espera por cada causa.
    pub fn esperas(&self) -> Vec<(Causa, u64)> {
        let mut v: Vec<(Causa, u64)> = Vec::new();
        if let Some(p) = &self.previa {
            v.push(p.clone());
        }
        for (s, c) in self.espera.iter().enumerate() {
            let Some(c) = c else { continue };
            let n = self.entra[s + 1] - self.entra[s] - self.trabajo[s];
            if n > 0 {
                v.push((c.clone(), n));
            }
        }
        v
    }
}

/// Calcula el horario de cada instruccion, en el orden en que se ejecutan.
#[derive(Debug, Clone)]
pub struct Planificador {
    cfg: ConfigPipeline,
    predictor: Predictor,
    /// Cuando queda libre cada etapa para la instruccion siguiente.
    libre: Vec<u64>,
    /// Hasta cuando no se puede empezar a buscar (salto sin resolver).
    bloqueo: u64,
    /// Para cada registro: quien lo escribio por ultima vez y cuando esta listo.
    productores: BTreeMap<String, Productor>,
    siguiente_id: u64,
}

#[derive(Debug, Clone, Copy)]
struct Productor {
    /// Ciclo en que el valor se puede adelantar.
    adelantado: u64,
    /// Ciclo en que se escribe en el registro.
    escrito: u64,
}

impl Planificador {
    pub fn nuevo(cfg: ConfigPipeline) -> Self {
        let n = cfg.etapas.len();
        let predictor = Predictor::nuevo(cfg.prediccion);
        Self {
            cfg,
            predictor,
            libre: vec![0; n],
            bloqueo: 0,
            productores: BTreeMap::new(),
            siguiente_id: 0,
        }
    }

    pub fn config(&self) -> &ConfigPipeline {
        &self.cfg
    }

    pub fn programar(&mut self, i: &Entrada) -> Planificada {
        let n = self.cfg.etapas.len();
        let c = &self.cfg;
        let mut trabajo = vec![1u64; n];
        trabajo[0] = trabajo[0].max(i.coste_busqueda);
        if c.ciclos_declarados {
            trabajo[c.ejecucion] = trabajo[c.ejecucion].max(i.ciclos_declarados);
        }
        trabajo[c.memoria] = trabajo[c.memoria].max(i.coste_memoria);
        // Etapa a la que no se puede entrar sin los operandos. Con adelantamiento hacen falta al
        // empezar a ejecutar; sin el, la instruccion espera en la etapa de lectura hasta que
        // quien produce el valor lo escribe (en la primera mitad del ciclo) y lo lee despues.
        let etapa_operandos = if c.adelantamiento {
            c.ejecucion
        } else {
            c.lectura + 1
        };
        let mut listo = 0;
        let mut por = None;
        for r in &i.lee {
            if let Some(p) = self.productores.get(r) {
                let t = if c.adelantamiento {
                    p.adelantado
                } else {
                    p.escrito + 1
                };
                if t > listo {
                    listo = t;
                    por = Some(r.clone());
                }
            }
        }
        let mut entra = vec![0u64; n + 1];
        let mut espera: Vec<Option<Causa>> = vec![None; n];
        // Entrada al pipeline: la etapa 0 debe estar libre y el salto anterior resuelto.
        let mut t0 = self.libre[0];
        let mut previa = None;
        if self.bloqueo > t0 {
            previa = Some((Causa::Control, self.bloqueo - t0));
            t0 = self.bloqueo;
        }
        if etapa_operandos == 0 && listo > t0 {
            previa = por
                .clone()
                .map(|registro| (Causa::Datos { registro }, listo - t0));
            t0 = listo;
        }
        entra[0] = t0;
        for s in 0..n {
            let fin = entra[s] + trabajo[s];
            let libre = if s + 1 < n { self.libre[s + 1] } else { fin };
            let datos = if s + 1 == etapa_operandos { listo } else { 0 };
            let sale = fin.max(libre).max(datos);
            if sale > fin {
                espera[s] = Some(if datos >= libre && datos > fin {
                    Causa::Datos {
                        registro: por.clone().unwrap_or_default(),
                    }
                } else {
                    Causa::Estructural
                });
            }
            entra[s + 1] = sale;
        }
        // Registros que escribe: cuando se pueden adelantar y cuando se escriben.
        let etapa_resultado = if i.carga { c.memoria } else { c.ejecucion };
        for r in &i.escribe {
            self.productores.insert(
                r.clone(),
                Productor {
                    adelantado: entra[etapa_resultado + 1],
                    escrito: entra[c.escritura],
                },
            );
        }
        // Riesgos de control.
        let mut prediccion = None;
        let mut descartadas = 0;
        let mut bloqueo = 0;
        if i.es_salto {
            let (predicho, acierto) = if i.incondicional && !i.indirecto {
                (true, true)
            } else if i.incondicional {
                (true, false)
            } else {
                let p = self.predictor.predecir(i.direccion);
                self.predictor.aprender(i.direccion, i.tomado);
                (p, p == i.tomado)
            };
            prediccion = Some((predicho, acierto));
            if !acierto {
                bloqueo = entra[c.salto + 1];
                descartadas = entra[c.salto + 1].saturating_sub(entra[1]);
            } else if predicho {
                bloqueo = entra[1] + c.ciclos_objetivo;
            }
        }
        self.bloqueo = bloqueo;
        self.libre[..n].copy_from_slice(&entra[1..=n]);
        let id = self.siguiente_id;
        self.siguiente_id += 1;
        Planificada {
            id,
            direccion: i.direccion,
            mnemonico: i.mnemonico.clone(),
            entra,
            trabajo,
            espera,
            previa,
            prediccion,
            descartadas,
        }
    }
}
