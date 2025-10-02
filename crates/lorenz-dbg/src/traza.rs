//! La traza: lo que hizo cada instruccion mientras la grabacion estuvo activa.
//!
//! Para volver a un instante pasado no se guarda la maquina entera en cada paso (RF-DEP-027):
//! se guardan instantaneas cada cierto numero de pasos y se vuelve a ejecutar desde la mas cercana.
//! Eso funciona porque el emulador es determinista; lo unico que no lo es son las cosas que hace el
//! usuario a mano durante la grabacion, que por eso se anotan como [`Intervencion`].

use lorenz_emu::{EscrituraRegistrada, Instantanea};

/// Un registro que cambio en un paso.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CambioRegistro {
    pub nombre: String,
    pub antes: u64,
    pub despues: u64,
}

/// Una bandera que cambio en un paso.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CambioBandera {
    pub nombre: String,
    pub antes: bool,
    pub despues: bool,
}

/// Lo que hizo una instruccion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasoTraza {
    /// Cuantos pasos se habian dado desde que empezo la grabacion (0 para el primero).
    pub instante: u64,
    /// Direccion de la instruccion.
    pub pc: u64,
    pub mnemonico: String,
    pub ciclos: u64,
    pub registros: Vec<CambioRegistro>,
    pub banderas: Vec<CambioBandera>,
    pub escrituras: Vec<EscrituraRegistrada>,
}

/// Algo que el usuario hizo a mano entre dos pasos y que hay que repetir al reejecutar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intervencion {
    Registro {
        nombre: String,
        valor: u64,
    },
    Bandera {
        nombre: String,
        encendida: bool,
    },
    Memoria {
        direccion: u64,
        valor: u64,
    },
    /// Escritura en un puerto.
    Puerto {
        numero: u64,
        valor: u64,
    },
    /// Un valor dejado en la cola de entrada de un puerto.
    Entrada {
        numero: u64,
        valor: u64,
    },
    /// Una lectura de un puerto, que consume su cola.
    LecturaDePuerto {
        numero: u64,
    },
}

/// Limites por omision.
pub const LIMITE_DE_PASOS: usize = 1_000_000;
pub const INTERVALO_DE_INSTANTANEAS: u64 = 1_000;
pub const MAX_INSTANTANEAS: usize = 64;

/// La grabacion de una ejecucion.
#[derive(Debug, Clone)]
pub struct Traza {
    pasos: Vec<PasoTraza>,
    /// `(instante, copia de la maquina)`, por instante creciente. Siempre incluye el 0.
    instantaneas: Vec<(u64, Instantanea)>,
    intervenciones: Vec<(u64, Intervencion)>,
    intervalo: u64,
    limite: usize,
    llena: bool,
}

impl Traza {
    /// Empieza una traza desde el estado que describe `inicial`.
    pub fn nueva(inicial: Instantanea) -> Self {
        Traza {
            pasos: Vec::new(),
            instantaneas: vec![(0, inicial)],
            intervenciones: Vec::new(),
            intervalo: INTERVALO_DE_INSTANTANEAS,
            limite: LIMITE_DE_PASOS,
            llena: false,
        }
    }

    /// Cuantos pasos tiene la traza; es tambien el instante final.
    pub fn longitud(&self) -> u64 {
        self.pasos.len() as u64
    }

    pub fn pasos(&self) -> &[PasoTraza] {
        &self.pasos
    }

    pub fn limite(&self) -> usize {
        self.limite
    }

    pub fn fijar_limite(&mut self, limite: usize) {
        self.limite = limite.max(1);
        self.llena = self.pasos.len() >= self.limite;
    }

    pub fn intervalo(&self) -> u64 {
        self.intervalo
    }

    pub fn fijar_intervalo(&mut self, intervalo: u64) {
        self.intervalo = intervalo.max(1);
    }

    /// `true` si se alcanzo el limite y ya no se graba (RF-DEP-028).
    pub fn llena(&self) -> bool {
        self.llena
    }

    pub fn instantaneas(&self) -> usize {
        self.instantaneas.len()
    }

    pub fn intervenciones(&self) -> &[(u64, Intervencion)] {
        &self.intervenciones
    }

    /// Anota un paso. Devuelve `false` si la traza esta llena y no lo guardo.
    ///
    /// `despues` es la maquina tras el paso; solo se copia si toca tomar una instantanea.
    pub fn anotar(&mut self, paso: PasoTraza, despues: impl FnOnce() -> Instantanea) -> bool {
        if self.llena {
            return false;
        }
        debug_assert_eq!(paso.instante, self.longitud());
        self.pasos.push(paso);
        let instante = self.longitud();
        if instante % self.intervalo == 0 {
            self.instantaneas.push((instante, despues()));
            self.adelgazar();
        }
        if self.pasos.len() >= self.limite {
            self.llena = true;
        }
        true
    }

    /// Si hay demasiadas instantaneas, se queda con una de cada dos y duplica el intervalo: la
    /// memoria usada tiene un tope y el coste es mas reejecucion (RF-DEP-027).
    fn adelgazar(&mut self) {
        while self.instantaneas.len() > MAX_INSTANTANEAS {
            let mut i = 0;
            self.instantaneas.retain(|_| {
                i += 1;
                (i - 1) % 2 == 0
            });
            self.intervalo *= 2;
        }
    }

    /// La instantanea mas reciente que no pasa de `instante`.
    pub fn instantanea_hasta(&self, instante: u64) -> (u64, &Instantanea) {
        let i = self
            .instantaneas
            .partition_point(|(t, _)| *t <= instante)
            .saturating_sub(1);
        let (t, s) = &self.instantaneas[i];
        (*t, s)
    }

    /// Anota algo hecho a mano en el instante `instante`.
    pub fn intervenir(&mut self, instante: u64, que: Intervencion) {
        self.intervenciones.push((instante, que));
    }

    /// Las intervenciones hechas justo antes de ejecutar el paso `instante`.
    pub fn intervenciones_en(&self, instante: u64) -> impl Iterator<Item = &Intervencion> {
        self.intervenciones
            .iter()
            .filter(move |(t, _)| *t == instante)
            .map(|(_, i)| i)
    }

    /// Descarta todo lo que paso desde `instante`: la historia se reescribe a partir de ahi.
    pub fn truncar(&mut self, instante: u64) {
        self.pasos.truncate(instante as usize);
        self.instantaneas.retain(|(t, _)| *t <= instante);
        self.intervenciones.retain(|(t, _)| *t < instante);
        self.llena = self.pasos.len() >= self.limite;
    }

    /// El ultimo paso, hasta `instante` sin incluirlo, que escribio `direccion` (RF-DEP-024).
    pub fn ultima_escritura(&self, direccion: u64, instante: u64) -> Option<&PasoTraza> {
        self.pasos[..(instante as usize).min(self.pasos.len())]
            .iter()
            .rev()
            .find(|p| p.escrituras.iter().any(|e| e.direccion == direccion))
    }

    /// El ultimo paso, hasta `instante` sin incluirlo, que cambio un registro (RF-DEP-025).
    pub fn ultimo_cambio_de_registro(&self, nombre: &str, instante: u64) -> Option<&PasoTraza> {
        self.pasos[..(instante as usize).min(self.pasos.len())]
            .iter()
            .rev()
            .find(|p| {
                p.registros
                    .iter()
                    .any(|r| r.nombre.eq_ignore_ascii_case(nombre))
            })
    }

    /// El ultimo paso, hasta `instante` sin incluirlo, que cambio una bandera (RF-DEP-025).
    pub fn ultimo_cambio_de_bandera(&self, nombre: &str, instante: u64) -> Option<&PasoTraza> {
        self.pasos[..(instante as usize).min(self.pasos.len())]
            .iter()
            .rev()
            .find(|p| {
                p.banderas
                    .iter()
                    .any(|b| b.nombre.eq_ignore_ascii_case(nombre))
            })
    }
}
