//! Memoria de la maquina emulada, organizada en las regiones que declara la descripcion.
//!
//! Cada direccion contiene una palabra de `bits_palabra` bits. Las regiones tienen permisos de
//! lectura y escritura que el programa debe respetar; el depurador y el cargador de programas los
//! saltan con las operaciones `*_privilegiado` (RF-EMU-019, RF-EMU-022, RF-EMU-023).

use std::collections::BTreeSet;

use lorenz_arch::modelo::{Acceso, Maquina};

use crate::error::ErrorEjecucion;

/// Maximo de celdas que se reserva para una sola region.
///
/// Evita que una descripcion con un espacio de direcciones enorme agote la memoria del equipo.
pub const MAX_CELDAS_POR_REGION: u64 = 1 << 24;

#[derive(Debug, Clone)]
struct RegionMem {
    nombre: String,
    desde: u64,
    hasta: u64,
    acceso: Acceso,
    dispositivo: Option<String>,
    celdas: Vec<u64>,
    /// Quien escribio cada celda por ultima vez; vacio mientras el seguimiento esta apagado.
    origen: Vec<u64>,
}

const SIN_ESCRIBIR: u64 = 0;
const CARGADA: u64 = 1;
const DESDE_INSTRUCCION: u64 = 2;

/// Cuantas lecturas de memoria sin inicializar se recuerdan como maximo.
pub const MAX_LECTURAS_SIN_INICIALIZAR: usize = 4096;

/// Quien escribio una celda por ultima vez (RF-EMU-025).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Escritura {
    /// Se cargo con un programa o la modifico el depurador.
    Carga,
    /// La escribio la instruccion que estaba en esta direccion.
    Instruccion { pc: u64 },
}

/// Tipo de acceso a una celda vigilada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoAcceso {
    Lectura,
    Escritura,
}

/// Un acceso de una instruccion a una celda que el depurador vigila (RF-DEP-014, RF-DEP-015).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccesoVigilado {
    pub direccion: u64,
    pub tipo: TipoAcceso,
    /// Direccion de la instruccion que accedio.
    pub pc: u64,
}

/// Una escritura hecha por una instruccion, con el valor que habia antes (RF-DEP-024).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EscrituraRegistrada {
    pub direccion: u64,
    pub antes: u64,
    pub despues: u64,
    /// Direccion de la instruccion que escribio.
    pub pc: u64,
}

/// Una lectura de una celda que nadie habia escrito (RF-EMU-026).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LecturaSinInicializar {
    pub direccion: u64,
    /// Direccion de la instruccion que la leyo.
    pub pc: u64,
}

/// El espacio de memoria completo de una maquina.
#[derive(Debug, Clone)]
pub struct Memoria {
    /// Ordenadas por direccion inicial y sin solaparse, para localizar por busqueda binaria.
    regiones: Vec<RegionMem>,
    mascara_palabra: u64,
    seguimiento: bool,
    instruccion_actual: u64,
    sin_inicializar: Vec<LecturaSinInicializar>,
    vigiladas_lectura: BTreeSet<u64>,
    vigiladas_escritura: BTreeSet<u64>,
    accesos: Vec<AccesoVigilado>,
    registrando: bool,
    escrituras: Vec<EscrituraRegistrada>,
    /// Resumen del contenido, puesto al dia en cada escritura (ver `suma_de_verificacion`).
    suma: u64,
}

/// Aporte de una celda al resumen: depende de la direccion y del valor, de modo que dos memorias
/// distintas casi nunca dan lo mismo y un cambio se puede restar y sumar sin recorrer todo.
fn aporte(direccion: u64, valor: u64) -> u64 {
    let mut z = valor.wrapping_add(direccion.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Memoria {
    /// Reserva la memoria que declara la maquina, con todas las celdas en cero.
    pub fn nueva(maquina: &Maquina) -> Result<Self, ErrorEjecucion> {
        let mut regiones = Vec::with_capacity(maquina.regiones.len());
        for r in &maquina.regiones {
            let celdas = r.tamano();
            if celdas > MAX_CELDAS_POR_REGION {
                return Err(ErrorEjecucion::RegionDemasiadoGrande {
                    region: r.nombre.clone(),
                    celdas,
                    maximo: MAX_CELDAS_POR_REGION,
                });
            }
            regiones.push(RegionMem {
                nombre: r.nombre.clone(),
                desde: r.desde,
                hasta: r.hasta,
                acceso: r.acceso,
                dispositivo: r.dispositivo.clone(),
                celdas: vec![0; celdas as usize],
                origen: Vec::new(),
            });
        }
        regiones.sort_by_key(|r| r.desde);
        let suma = regiones
            .iter()
            .flat_map(|r| (0..r.celdas.len() as u64).map(move |i| aporte(r.desde + i, 0)))
            .fold(0u64, u64::wrapping_add);
        Ok(Self {
            regiones,
            mascara_palabra: maquina.palabra_maxima(),
            seguimiento: false,
            instruccion_actual: 0,
            sin_inicializar: Vec::new(),
            vigiladas_lectura: BTreeSet::new(),
            vigiladas_escritura: BTreeSet::new(),
            accesos: Vec::new(),
            registrando: false,
            escrituras: Vec::new(),
            suma,
        })
    }

    /// Indice de la region que contiene la direccion, si hay alguna.
    fn localizar(&self, direccion: u64) -> Option<usize> {
        let i = self.regiones.partition_point(|r| r.desde <= direccion);
        if i == 0 {
            return None;
        }
        let candidata = i - 1;
        (direccion <= self.regiones[candidata].hasta).then_some(candidata)
    }

    /// Mascara de una palabra: un 1 en cada bit que la maquina usa.
    pub fn mascara_palabra(&self) -> u64 {
        self.mascara_palabra
    }

    /// Cuantas regiones hay.
    pub fn numero_de_regiones(&self) -> usize {
        self.regiones.len()
    }

    /// Nombre de la region que contiene la direccion.
    pub fn region_de(&self, direccion: u64) -> Option<&str> {
        self.localizar(direccion)
            .map(|i| self.regiones[i].nombre.as_str())
    }

    /// Dispositivo al que responde la direccion, si pertenece a una region mapeada.
    pub fn dispositivo_de(&self, direccion: u64) -> Option<&str> {
        self.localizar(direccion)
            .and_then(|i| self.regiones[i].dispositivo.as_deref())
    }

    /// Posicion de la direccion dentro de su region, contando desde cero.
    pub fn desplazamiento_en_region(&self, direccion: u64) -> Option<u64> {
        self.localizar(direccion)
            .map(|i| direccion - self.regiones[i].desde)
    }

    /// Resumen de todo el contenido. Dos memorias con el mismo contenido dan el mismo valor; sirve
    /// para comparar sin recorrerlas a mano. Se mantiene al dia en cada escritura, asi que consultarlo
    /// cuesta lo mismo con una memoria pequena que con una de 64 KiB.
    pub fn suma_de_verificacion(&self) -> u64 {
        self.suma
    }

    /// Si la direccion cae en alguna region.
    pub fn es_valida(&self, direccion: u64) -> bool {
        self.localizar(direccion).is_some()
    }

    /// Total de celdas reservadas entre todas las regiones.
    pub fn celdas_totales(&self) -> u64 {
        self.regiones.iter().map(|r| r.celdas.len() as u64).sum()
    }

    /// Lee una palabra respetando los permisos de la region (RF-EMU-021).
    pub fn leer(&self, direccion: u64) -> Result<u64, ErrorEjecucion> {
        let i = self
            .localizar(direccion)
            .ok_or(ErrorEjecucion::FueraDeRegion { direccion })?;
        let r = &self.regiones[i];
        if !r.acceso.lectura {
            return Err(ErrorEjecucion::LecturaDeSoloEscritura {
                direccion,
                region: r.nombre.clone(),
            });
        }
        Ok(r.celdas[(direccion - r.desde) as usize])
    }

    /// Escribe una palabra respetando los permisos de la region (RF-EMU-020).
    ///
    /// El valor se trunca al ancho de palabra de la maquina.
    pub fn escribir(&mut self, direccion: u64, valor: u64) -> Result<(), ErrorEjecucion> {
        let mascara = self.mascara_palabra;
        let i = self
            .localizar(direccion)
            .ok_or(ErrorEjecucion::FueraDeRegion { direccion })?;
        let r = &mut self.regiones[i];
        if !r.acceso.escritura {
            return Err(ErrorEjecucion::EscrituraEnSoloLectura {
                direccion,
                region: r.nombre.clone(),
            });
        }
        let celda = (direccion - r.desde) as usize;
        let antes = r.celdas[celda];
        r.celdas[celda] = valor & mascara;
        self.suma = self
            .suma
            .wrapping_sub(aporte(direccion, antes))
            .wrapping_add(aporte(direccion, valor & mascara));
        if self.registrando {
            self.escrituras.push(EscrituraRegistrada {
                direccion,
                antes,
                despues: valor & mascara,
                pc: self.instruccion_actual,
            });
        }
        if self.seguimiento {
            r.origen[celda] = DESDE_INSTRUCCION + self.instruccion_actual;
        }
        if self.vigiladas_escritura.contains(&direccion) {
            self.accesos.push(AccesoVigilado {
                direccion,
                tipo: TipoAcceso::Escritura,
                pc: self.instruccion_actual,
            });
        }
        Ok(())
    }

    /// Lee una palabra ignorando los permisos: para el depurador (RF-EMU-022).
    pub fn leer_privilegiado(&self, direccion: u64) -> Result<u64, ErrorEjecucion> {
        let i = self
            .localizar(direccion)
            .ok_or(ErrorEjecucion::FueraDeRegion { direccion })?;
        let r = &self.regiones[i];
        Ok(r.celdas[(direccion - r.desde) as usize])
    }

    /// Escribe una palabra ignorando los permisos: para el depurador y para cargar programas en
    /// una ROM (RF-EMU-022, RF-EMU-023).
    pub fn escribir_privilegiado(
        &mut self,
        direccion: u64,
        valor: u64,
    ) -> Result<(), ErrorEjecucion> {
        let mascara = self.mascara_palabra;
        let i = self
            .localizar(direccion)
            .ok_or(ErrorEjecucion::FueraDeRegion { direccion })?;
        let r = &mut self.regiones[i];
        let celda = (direccion - r.desde) as usize;
        let antes = r.celdas[celda];
        r.celdas[celda] = valor & mascara;
        self.suma = self
            .suma
            .wrapping_sub(aporte(direccion, antes))
            .wrapping_add(aporte(direccion, valor & mascara));
        if self.seguimiento {
            r.origen[celda] = CARGADA;
        }
        Ok(())
    }

    /// Copia un bloque de palabras desde `direccion`, ignorando los permisos.
    ///
    /// Es la operacion con la que se carga un programa antes de ejecutar (RF-EMU-023). Si el bloque
    /// no cabe entero dentro de las regiones, no se escribe nada.
    pub fn cargar_bloque(
        &mut self,
        direccion: u64,
        palabras: &[u64],
    ) -> Result<(), ErrorEjecucion> {
        let fuera = || ErrorEjecucion::BloqueFueraDeRegion {
            direccion,
            longitud: palabras.len(),
        };
        for i in 0..palabras.len() as u64 {
            let d = direccion.checked_add(i).ok_or_else(fuera)?;
            if !self.es_valida(d) {
                return Err(fuera());
            }
        }
        for (i, valor) in palabras.iter().enumerate() {
            self.escribir_privilegiado(direccion + i as u64, *valor)?;
        }
        Ok(())
    }

    /// Lee `longitud` palabras desde `direccion`, ignorando los permisos (RF-EMU-024).
    pub fn volcar(&self, direccion: u64, longitud: usize) -> Result<Vec<u64>, ErrorEjecucion> {
        (0..longitud as u64)
            .map(|i| {
                let d = direccion
                    .checked_add(i)
                    .ok_or(ErrorEjecucion::BloqueFueraDeRegion {
                        direccion,
                        longitud,
                    })?;
                self.leer_privilegiado(d)
            })
            .collect()
    }

    // --- Seguimiento de escrituras y lecturas sin inicializar (RF-EMU-025, RF-EMU-026) ---

    /// Empieza a recordar quien escribe cada celda y a avisar de las lecturas sin inicializar.
    ///
    /// Las celdas que ya valen distinto de cero se dan por cargadas; las que valen cero se dan por
    /// no escritas. Por eso conviene activarlo antes de cargar el programa.
    pub fn activar_seguimiento(&mut self) {
        if self.seguimiento {
            return;
        }
        for r in &mut self.regiones {
            r.origen = r
                .celdas
                .iter()
                .map(|c| if *c == 0 { SIN_ESCRIBIR } else { CARGADA })
                .collect();
        }
        self.seguimiento = true;
    }

    pub fn seguimiento_activo(&self) -> bool {
        self.seguimiento
    }

    /// Dice cual es la instruccion que esta ejecutandose, para atribuirle sus escrituras.
    pub fn fijar_instruccion_actual(&mut self, pc: u64) {
        self.instruccion_actual = pc;
    }

    /// Como `leer`, pero anota si la celda nunca se habia escrito.
    pub fn leer_vigilada(&mut self, direccion: u64) -> Result<u64, ErrorEjecucion> {
        let valor = self.leer(direccion)?;
        if self.vigiladas_lectura.contains(&direccion) {
            self.accesos.push(AccesoVigilado {
                direccion,
                tipo: TipoAcceso::Lectura,
                pc: self.instruccion_actual,
            });
        }
        if self.seguimiento
            && self.origen_de(direccion) == Some(SIN_ESCRIBIR)
            && self.sin_inicializar.len() < MAX_LECTURAS_SIN_INICIALIZAR
        {
            self.sin_inicializar.push(LecturaSinInicializar {
                direccion,
                pc: self.instruccion_actual,
            });
        }
        Ok(valor)
    }

    fn origen_de(&self, direccion: u64) -> Option<u64> {
        let r = &self.regiones[self.localizar(direccion)?];
        r.origen.get((direccion - r.desde) as usize).copied()
    }

    /// Quien escribio la celda por ultima vez, o `None` si nadie la escribio o el seguimiento esta
    /// apagado.
    pub fn ultima_escritura(&self, direccion: u64) -> Result<Option<Escritura>, ErrorEjecucion> {
        if !self.es_valida(direccion) {
            return Err(ErrorEjecucion::FueraDeRegion { direccion });
        }
        Ok(match self.origen_de(direccion) {
            Some(CARGADA) => Some(Escritura::Carga),
            Some(n) if n >= DESDE_INSTRUCCION => Some(Escritura::Instruccion {
                pc: n - DESDE_INSTRUCCION,
            }),
            _ => None,
        })
    }

    /// Si alguien escribio la celda. Con el seguimiento apagado todas se dan por inicializadas.
    pub fn esta_inicializada(&self, direccion: u64) -> Result<bool, ErrorEjecucion> {
        if !self.es_valida(direccion) {
            return Err(ErrorEjecucion::FueraDeRegion { direccion });
        }
        Ok(!self.seguimiento || self.origen_de(direccion) != Some(SIN_ESCRIBIR))
    }

    /// Lecturas de celdas sin inicializar desde que se activo el seguimiento.
    pub fn lecturas_sin_inicializar(&self) -> &[LecturaSinInicializar] {
        &self.sin_inicializar
    }

    /// Empieza a anotar los accesos de las instrucciones a `direccion`. Lo que escribe o carga el
    /// depurador no cuenta: solo lo que hace el programa.
    pub fn vigilar(&mut self, direccion: u64, tipo: TipoAcceso) {
        match tipo {
            TipoAcceso::Lectura => self.vigiladas_lectura.insert(direccion),
            TipoAcceso::Escritura => self.vigiladas_escritura.insert(direccion),
        };
    }

    /// Deja de vigilar. Devuelve `true` si se vigilaba.
    pub fn dejar_de_vigilar(&mut self, direccion: u64, tipo: TipoAcceso) -> bool {
        match tipo {
            TipoAcceso::Lectura => self.vigiladas_lectura.remove(&direccion),
            TipoAcceso::Escritura => self.vigiladas_escritura.remove(&direccion),
        }
    }

    /// Direcciones vigiladas para un tipo de acceso, en orden.
    pub fn vigiladas(&self, tipo: TipoAcceso) -> Vec<u64> {
        match tipo {
            TipoAcceso::Lectura => self.vigiladas_lectura.iter().copied().collect(),
            TipoAcceso::Escritura => self.vigiladas_escritura.iter().copied().collect(),
        }
    }

    /// Entrega y olvida los accesos vigilados anotados desde la ultima llamada.
    pub fn tomar_accesos(&mut self) -> Vec<AccesoVigilado> {
        std::mem::take(&mut self.accesos)
    }

    /// Empieza o deja de anotar cada escritura de las instrucciones, con el valor anterior. Lo que
    /// escribe el depurador no cuenta.
    pub fn registrar_escrituras(&mut self, activo: bool) {
        self.registrando = activo;
        if !activo {
            self.escrituras.clear();
        }
    }

    /// Entrega y olvida las escrituras anotadas desde la ultima llamada.
    pub fn tomar_escrituras(&mut self) -> Vec<EscrituraRegistrada> {
        std::mem::take(&mut self.escrituras)
    }

    /// Olvida las lecturas sin inicializar anotadas.
    pub fn olvidar_lecturas_sin_inicializar(&mut self) {
        self.sin_inicializar.clear();
    }
}
