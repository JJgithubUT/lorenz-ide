//! Una segunda maquina completa construida sobre el evaluador de referencia (RF-TST-025).
//!
//! Decodifica con el decodificador del emulador (la codificacion se prueba aparte, con la ida y
//! vuelta de la conformidad) pero ejecuta la semantica con [`Referencia`]. Asi un programa
//! aleatorio se puede correr en las dos implementaciones y comparar instruccion a instruccion
//! con [`lorenz_emu::comparar`].
//!
//! No modela puertos ni dispositivos. Si un programa llega a uno, [`EjecutorReferencia::no_modelado`]
//! queda en `true` y el resultado de esa comparacion no vale.

use lorenz_arch::modelo::{Maquina, RolRegistro};
use lorenz_emu::{Decodificador, Ejecutor, ErrorEjecucion, Memoria, Paso, Vista};

use crate::referencia::{AccesoMemoria, ErrorRef, EstadoRef, Referencia, ValorDeOperando};

/// Lo que el emulador responde cuando un programa pide algo que la referencia no modela.
fn no_modelado_error() -> ErrorEjecucion {
    ErrorEjecucion::RegistroInexistente {
        nombre: "(no modelado por la referencia)".to_string(),
    }
}

/// La memoria vista por la semantica: los permisos salen de las regiones de la maquina.
struct Permisos<'a> {
    maquina: &'a Maquina,
    memoria: &'a Memoria,
}

impl AccesoMemoria for Permisos<'_> {
    fn leer(&self, direccion: u64) -> Result<u64, ErrorRef> {
        let r = self.maquina.region_de(direccion).ok_or(ErrorRef::Memoria(
            ErrorEjecucion::FueraDeRegion { direccion },
        ))?;
        if r.dispositivo.is_some() {
            return Err(ErrorRef::UsaPuertos);
        }
        if !r.acceso.lectura {
            return Err(ErrorRef::Memoria(ErrorEjecucion::LecturaDeSoloEscritura {
                direccion,
                region: r.nombre.clone(),
            }));
        }
        self.memoria
            .leer_privilegiado(direccion)
            .map_err(ErrorRef::Memoria)
    }

    fn comprobar_escritura(&self, direccion: u64) -> Result<(), ErrorRef> {
        let r = self.maquina.region_de(direccion).ok_or(ErrorRef::Memoria(
            ErrorEjecucion::FueraDeRegion { direccion },
        ))?;
        if r.dispositivo.is_some() {
            return Err(ErrorRef::UsaPuertos);
        }
        if !r.acceso.escritura {
            return Err(ErrorRef::Memoria(ErrorEjecucion::EscrituraEnSoloLectura {
                direccion,
                region: r.nombre.clone(),
            }));
        }
        Ok(())
    }
}

pub struct EjecutorReferencia<'m> {
    maquina: &'m Maquina,
    referencia: Referencia<'m>,
    decodificador: Decodificador,
    memoria: Memoria,
    estado: EstadoRef,
    pc: String,
    instrucciones: u64,
    ciclos: u64,
    mascara_direccion: u64,
    /// El programa toco algo que la referencia no modela (puertos o dispositivos).
    pub no_modelado: bool,
}

impl<'m> EjecutorReferencia<'m> {
    pub fn nuevo(maquina: &'m Maquina) -> Result<Self, String> {
        let referencia = Referencia::nueva(maquina);
        let pc = maquina
            .registro_con_rol(RolRegistro::ContadorPrograma)
            .map(|r| r.nombre.clone())
            .ok_or("la maquina no tiene contador de programa")?;
        let mut estado = referencia.estado_inicial();
        estado.registros.insert(pc.clone(), maquina.arranque);
        estado.ie = maquina
            .interrupciones
            .as_ref()
            .is_some_and(|i| i.habilitadas_al_arrancar);
        let mascara_direccion = maquina.direccion_maxima();
        Ok(EjecutorReferencia {
            maquina,
            referencia,
            decodificador: Decodificador::nuevo(maquina),
            memoria: Memoria::nueva(maquina).map_err(|e| e.to_string())?,
            estado,
            pc,
            instrucciones: 0,
            ciclos: 0,
            mascara_direccion,
            no_modelado: false,
        })
    }

    /// Pone un programa en memoria, ignorando los permisos como hace el cargador.
    pub fn cargar(&mut self, direccion: u64, palabras: &[u64]) -> Result<(), String> {
        for (i, p) in palabras.iter().enumerate() {
            self.memoria
                .escribir_privilegiado(direccion + i as u64, *p)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn fijar_registro(&mut self, nombre: &str, valor: u64) {
        if let Some(r) = self.maquina.registro(nombre) {
            let m = if r.bits >= 64 {
                u64::MAX
            } else {
                (1u64 << r.bits) - 1
            };
            self.estado.registros.insert(nombre.to_string(), valor & m);
        }
    }

    fn pc_actual(&self) -> u64 {
        self.estado.registros[&self.pc]
    }
}

impl Ejecutor for EjecutorReferencia<'_> {
    fn paso(&mut self) -> Result<Option<Paso>, ErrorEjecucion> {
        if self.estado.detenida {
            return Ok(None);
        }
        let direccion = self.pc_actual();
        let dec = self.decodificador.decodificar(&self.memoria, direccion)?;
        let instruccion = &self.maquina.instrucciones[dec.indice];
        let siguiente = direccion.wrapping_add(dec.longitud as u64) & self.mascara_direccion;
        let operandos: Vec<ValorDeOperando> = dec
            .operandos
            .iter()
            .map(|o| ValorDeOperando {
                nombre: o.nombre.clone(),
                valor: o.valor,
            })
            .collect();
        let mut trabajo = self.estado.clone();
        trabajo.escrituras.clear();
        trabajo.lecturas_iniciales.clear();
        trabajo.accedidas.clear();
        trabajo.salto = false;
        let resultado = {
            let acceso = Permisos {
                maquina: self.maquina,
                memoria: &self.memoria,
            };
            self.referencia
                .ejecutar_en(instruccion, &operandos, &mut trabajo, siguiente, &acceso)
        };
        // Lo que se alcanzo a hacer se conserva, haya fallado la instruccion o no.
        for (a, v) in &trabajo.escrituras {
            self.memoria.escribir_privilegiado(*a, *v)?;
        }
        let EstadoRef {
            registros,
            banderas,
            ie,
            salto,
            detenida,
            ..
        } = trabajo;
        self.estado.registros = registros;
        self.estado.banderas = banderas;
        self.estado.ie = ie;
        match resultado {
            Ok(ciclos) => {
                self.estado.detenida = detenida;
                self.instrucciones += 1;
                self.ciclos += ciclos;
                Ok(Some(Paso {
                    direccion,
                    mnemonico: instruccion.mnemonico.clone(),
                    longitud: dec.longitud,
                    ciclos,
                    salto,
                    detenida,
                }))
            }
            Err(e) => {
                self.estado.registros.insert(self.pc.clone(), direccion);
                Err(match e {
                    ErrorRef::Memoria(m) => m,
                    ErrorRef::DivisionEntreCero => ErrorEjecucion::DivisionEntreCero {
                        mnemonico: instruccion.mnemonico.clone(),
                    },
                    ErrorRef::DestinoNoAsignable(nombre) => {
                        ErrorEjecucion::RegistroDeSoloLectura { nombre }
                    }
                    ErrorRef::NombreDesconocido(nombre) => {
                        ErrorEjecucion::RegistroInexistente { nombre }
                    }
                    ErrorRef::UsaPuertos => {
                        self.no_modelado = true;
                        no_modelado_error()
                    }
                })
            }
        }
    }

    fn no_comparable(&self) -> bool {
        self.no_modelado
    }

    fn vista(&self) -> Vista {
        let registros = self
            .maquina
            .registros
            .iter()
            .map(|r| {
                let v = if r.rol == RolRegistro::Banderas {
                    self.referencia.banderas_empaquetadas(&self.estado)
                } else {
                    self.estado.registros[&r.nombre]
                };
                (r.nombre.clone(), v)
            })
            .collect();
        Vista {
            pc: self.pc_actual(),
            registros,
            instrucciones: self.instrucciones,
            ciclos: self.ciclos,
            detenida: self.estado.detenida,
            memoria: self.memoria.suma_de_verificacion(),
        }
    }
}
