//! Cache asociativa por conjuntos (RF-CIC-015 a RF-CIC-019).

/// Politica de reemplazo cuando un conjunto esta lleno (RF-CIC-016).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reemplazo {
    /// La linea que lleva mas tiempo sin usarse.
    Lru,
    /// La linea que entro primero.
    Fifo,
    /// Una linea elegida con un generador determinista.
    Aleatorio,
}

/// Politica de escritura (RF-CIC-017).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Escritura {
    /// Se escribe en la cache y la linea se marca sucia; se vuelca al desalojarla.
    PostEscritura,
    /// Se escribe a la vez en la cache y en memoria; un fallo de escritura no trae la linea.
    EscrituraDirecta,
}

/// Forma de la cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigCache {
    /// Palabras en total.
    pub tamano: u64,
    /// Palabras por linea.
    pub linea: u64,
    /// Lineas por conjunto.
    pub vias: u64,
    pub reemplazo: Reemplazo,
    pub escritura: Escritura,
    /// Ciclos que cuesta un acierto.
    pub latencia_acierto: u64,
    /// Si las busquedas de instrucciones tambien pasan por la cache.
    pub instrucciones: bool,
}

impl ConfigCache {
    pub fn conjuntos(&self) -> u64 {
        self.tamano / (self.linea * self.vias)
    }

    /// Comprueba que la forma tenga sentido.
    pub fn validar(&self) -> Result<(), String> {
        if self.tamano == 0 || self.linea == 0 || self.vias == 0 {
            return Err("tamano, linea y vias deben ser mayores que cero".into());
        }
        if self.tamano % (self.linea * self.vias) != 0 {
            return Err(format!(
                "el tamano ({}) debe ser multiplo de linea x vias ({} x {})",
                self.tamano, self.linea, self.vias
            ));
        }
        if self.latencia_acierto == 0 {
            return Err("latencia_acierto debe ser al menos 1".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Linea {
    valida: bool,
    sucia: bool,
    etiqueta: u64,
    /// Ultimo uso, para LRU.
    uso: u64,
    /// Orden de llegada, para FIFO.
    entrada: u64,
}

/// Contadores de la cache, separados por lectura y escritura (RF-CIC-018).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EstadisticasCache {
    pub aciertos_lectura: u64,
    pub fallos_lectura: u64,
    pub aciertos_escritura: u64,
    pub fallos_escritura: u64,
    /// Lineas sucias volcadas a memoria al desalojarlas.
    pub volcados: u64,
    /// Lineas traidas de memoria.
    pub rellenos: u64,
}

impl EstadisticasCache {
    pub fn accesos(&self) -> u64 {
        self.aciertos_lectura
            + self.fallos_lectura
            + self.aciertos_escritura
            + self.fallos_escritura
    }

    pub fn aciertos(&self) -> u64 {
        self.aciertos_lectura + self.aciertos_escritura
    }

    /// Fraccion de aciertos, o `None` si no hubo accesos.
    pub fn tasa_de_aciertos(&self) -> Option<f64> {
        let a = self.accesos();
        (a > 0).then(|| self.aciertos() as f64 / a as f64)
    }
}

/// Que paso en un acceso, para que quien llama calcule el coste con las latencias de memoria.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResultadoCache {
    pub acierto: bool,
    /// Se trajo una linea de memoria.
    pub relleno: bool,
    /// Se volco una linea sucia a memoria.
    pub volcado: bool,
    /// Se escribio una palabra directamente en memoria.
    pub escritura_directa: bool,
}

/// Una linea tal como se muestra (RF-CIC-019).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilaCache {
    pub conjunto: u64,
    pub via: u64,
    pub valida: bool,
    pub sucia: bool,
    /// Direccion de la primera palabra de la linea, si es valida.
    pub direccion: Option<u64>,
    pub etiqueta: u64,
}

#[derive(Debug, Clone)]
pub struct Cache {
    cfg: ConfigCache,
    conjuntos: Vec<Vec<Linea>>,
    reloj: u64,
    semilla: u64,
    estadisticas: EstadisticasCache,
}

impl Cache {
    pub fn nueva(cfg: ConfigCache) -> Result<Self, String> {
        cfg.validar()?;
        let n = cfg.conjuntos() as usize;
        let vias = cfg.vias as usize;
        Ok(Self {
            cfg,
            conjuntos: vec![vec![Linea::default(); vias]; n],
            reloj: 0,
            semilla: 0x2545_F491_4F6C_DD1D,
            estadisticas: EstadisticasCache::default(),
        })
    }

    pub fn config(&self) -> &ConfigCache {
        &self.cfg
    }

    pub fn estadisticas(&self) -> EstadisticasCache {
        self.estadisticas
    }

    fn aleatorio(&mut self) -> u64 {
        // xorshift64: determinista, para que la misma ejecucion repita el mismo resultado.
        let mut x = self.semilla;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.semilla = x;
        x
    }

    /// Hace un acceso y dice que ocurrio.
    pub fn acceder(&mut self, direccion: u64, escritura: bool) -> ResultadoCache {
        self.reloj += 1;
        let numero_de_linea = direccion / self.cfg.linea;
        let conjunto = (numero_de_linea % self.cfg.conjuntos()) as usize;
        let etiqueta = numero_de_linea / self.cfg.conjuntos();
        let directa = self.cfg.escritura == Escritura::EscrituraDirecta;
        let reloj = self.reloj;
        if let Some(l) = self.conjuntos[conjunto]
            .iter_mut()
            .find(|l| l.valida && l.etiqueta == etiqueta)
        {
            l.uso = reloj;
            if escritura {
                if !directa {
                    l.sucia = true;
                }
                self.estadisticas.aciertos_escritura += 1;
            } else {
                self.estadisticas.aciertos_lectura += 1;
            }
            return ResultadoCache {
                acierto: true,
                relleno: false,
                volcado: false,
                escritura_directa: escritura && directa,
            };
        }
        if escritura {
            self.estadisticas.fallos_escritura += 1;
        } else {
            self.estadisticas.fallos_lectura += 1;
        }
        if escritura && directa {
            // Sin asignacion en el fallo de escritura: la palabra va solo a memoria.
            return ResultadoCache {
                acierto: false,
                relleno: false,
                volcado: false,
                escritura_directa: true,
            };
        }
        let via = self.elegir_victima(conjunto);
        let l = &mut self.conjuntos[conjunto][via];
        let volcado = l.valida && l.sucia;
        *l = Linea {
            valida: true,
            sucia: escritura,
            etiqueta,
            uso: reloj,
            entrada: reloj,
        };
        self.estadisticas.rellenos += 1;
        if volcado {
            self.estadisticas.volcados += 1;
        }
        ResultadoCache {
            acierto: false,
            relleno: true,
            volcado,
            escritura_directa: false,
        }
    }

    fn elegir_victima(&mut self, conjunto: usize) -> usize {
        if let Some(i) = self.conjuntos[conjunto].iter().position(|l| !l.valida) {
            return i;
        }
        match self.cfg.reemplazo {
            Reemplazo::Lru => self.conjuntos[conjunto]
                .iter()
                .enumerate()
                .min_by_key(|(_, l)| l.uso)
                .map(|(i, _)| i)
                .unwrap_or(0),
            Reemplazo::Fifo => self.conjuntos[conjunto]
                .iter()
                .enumerate()
                .min_by_key(|(_, l)| l.entrada)
                .map(|(i, _)| i)
                .unwrap_or(0),
            Reemplazo::Aleatorio => (self.aleatorio() % self.cfg.vias) as usize,
        }
    }

    /// Todas las lineas, con su etiqueta y sus bits de validez y de suciedad (RF-CIC-019).
    pub fn contenido(&self) -> Vec<FilaCache> {
        let n = self.cfg.conjuntos();
        let mut filas = Vec::new();
        for (c, vias) in self.conjuntos.iter().enumerate() {
            for (v, l) in vias.iter().enumerate() {
                filas.push(FilaCache {
                    conjunto: c as u64,
                    via: v as u64,
                    valida: l.valida,
                    sucia: l.sucia,
                    direccion: l
                        .valida
                        .then(|| (l.etiqueta * n + c as u64) * self.cfg.linea),
                    etiqueta: l.etiqueta,
                });
            }
        }
        filas
    }

    /// El contenido como texto, una linea por entrada.
    pub fn volcar(&self) -> String {
        let mut s = String::from("# conjunto via valida sucia etiqueta direccion\n");
        for f in self.contenido() {
            s.push_str(&format!(
                "{} {} {} {} {:#x} {}\n",
                f.conjunto,
                f.via,
                u8::from(f.valida),
                u8::from(f.sucia),
                f.etiqueta,
                f.direccion.map_or("-".to_string(), |d| format!("{d:#x}"))
            ));
        }
        s
    }
}
