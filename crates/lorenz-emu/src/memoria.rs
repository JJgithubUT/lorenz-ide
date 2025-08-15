//! Memoria de la maquina emulada, organizada en las regiones que declara la descripcion.
//!
//! Cada direccion contiene una palabra de `bits_palabra` bits. Las regiones tienen permisos de
//! lectura y escritura que el programa debe respetar; el depurador y el cargador de programas los
//! saltan con las operaciones `*_privilegiado` (RF-EMU-019, RF-EMU-022, RF-EMU-023).

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
}

/// El espacio de memoria completo de una maquina.
#[derive(Debug, Clone)]
pub struct Memoria {
    /// Ordenadas por direccion inicial y sin solaparse, para localizar por busqueda binaria.
    regiones: Vec<RegionMem>,
    mascara_palabra: u64,
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
            });
        }
        regiones.sort_by_key(|r| r.desde);
        Ok(Self {
            regiones,
            mascara_palabra: maquina.palabra_maxima(),
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
        r.celdas[(direccion - r.desde) as usize] = valor & mascara;
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
        r.celdas[(direccion - r.desde) as usize] = valor & mascara;
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
}
