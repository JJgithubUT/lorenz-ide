//! El emulador completo: junta memoria, estado, puertos, decodificador e interprete y ejecuta
//! programas instruccion por instruccion.

use lorenz_arch::modelo::{Ciclos, Maquina};

use crate::decodificador::Decodificador;
use crate::error::ErrorEjecucion;
use crate::estado::Estado;
use crate::interprete::{Entorno, Interprete};
use crate::memoria::Memoria;
use crate::puertos::Puertos;

/// Que hizo una instruccion al ejecutarse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paso {
    /// Direccion donde estaba la instruccion.
    pub direccion: u64,
    pub mnemonico: String,
    /// Palabras que ocupa.
    pub longitud: usize,
    /// Ciclos que costo.
    pub ciclos: u64,
    /// Si asigno el contador de programa.
    pub salto: bool,
    /// Si detuvo la maquina.
    pub detenida: bool,
}

/// Resumen de una ejecucion hasta detenerse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resumen {
    /// Instrucciones ejecutadas en esta llamada.
    pub instrucciones: u64,
    /// Ciclos consumidos en esta llamada.
    pub ciclos: u64,
}

/// Una maquina emulada lista para ejecutar.
#[derive(Debug, Clone)]
pub struct Emulador {
    maquina: Maquina,
    decodificador: Decodificador,
    interprete: Interprete,
    estado: Estado,
    memoria: Memoria,
    puertos: Puertos,
}

impl Emulador {
    pub fn nuevo(maquina: Maquina) -> Result<Self, ErrorEjecucion> {
        let memoria = Memoria::nueva(&maquina)?;
        Ok(Emulador {
            decodificador: Decodificador::nuevo(&maquina),
            interprete: Interprete::nuevo(&maquina),
            estado: Estado::nuevo(&maquina),
            puertos: Puertos::nuevo(&maquina),
            memoria,
            maquina,
        })
    }

    pub fn maquina(&self) -> &Maquina {
        &self.maquina
    }

    pub fn estado(&self) -> &Estado {
        &self.estado
    }

    pub fn estado_mut(&mut self) -> &mut Estado {
        &mut self.estado
    }

    pub fn memoria(&self) -> &Memoria {
        &self.memoria
    }

    pub fn memoria_mut(&mut self) -> &mut Memoria {
        &mut self.memoria
    }

    pub fn puertos(&self) -> &Puertos {
        &self.puertos
    }

    pub fn puertos_mut(&mut self) -> &mut Puertos {
        &mut self.puertos
    }

    /// Copia un programa a memoria sin mirar los permisos, por ejemplo dentro de una rom.
    pub fn cargar_programa(&mut self, direccion: u64, palabras: &[u64]) -> Result<(), ErrorEjecucion> {
        self.memoria.cargar_bloque(direccion, palabras)
    }

    /// Vuelve registros, contadores y puertos a su estado inicial. La memoria no se toca.
    pub fn reiniciar(&mut self) {
        self.estado.reiniciar();
        self.puertos.reiniciar();
    }

    /// Ejecuta una instruccion. Devuelve `None` si la maquina ya estaba detenida.
    ///
    /// Si la instruccion falla, el contador de programa vuelve a apuntar a ella para que el
    /// depurador muestre donde ocurrio el error.
    pub fn paso(&mut self) -> Result<Option<Paso>, ErrorEjecucion> {
        if self.estado.detenida() {
            return Ok(None);
        }
        let direccion = self.estado.pc();
        let dec = self.decodificador.decodificar(&self.memoria, direccion)?;
        self.estado.fijar_pc(direccion.wrapping_add(dec.longitud as u64));
        let instruccion = &self.maquina.instrucciones[dec.indice];
        let mut entorno = Entorno {
            estado: &mut self.estado,
            memoria: &mut self.memoria,
            puertos: &mut self.puertos,
        };
        let resultado = match self.interprete.ejecutar(instruccion, &dec, &mut entorno) {
            Ok(r) => r,
            Err(e) => {
                self.estado.fijar_pc(direccion);
                return Err(e);
            }
        };
        let ciclos = match instruccion.ciclos {
            Ciclos::Fijo(n) => n,
            Ciclos::SegunSalto { tomado, no_tomado } => {
                if resultado.salto { tomado } else { no_tomado }
            }
        } as u64;
        self.estado.contar_instruccion(ciclos);
        Ok(Some(Paso {
            direccion,
            mnemonico: instruccion.mnemonico.clone(),
            longitud: dec.longitud,
            ciclos,
            salto: resultado.salto,
            detenida: resultado.detenida,
        }))
    }

    /// Ejecuta hasta que la maquina se detenga o se agote el limite de instrucciones.
    ///
    /// El limite cuenta las instrucciones de esta llamada. Si se alcanza sin detenerse, devuelve
    /// `LimiteDeInstrucciones` (RF-EMU-017): un programa que no termina no cuelga al usuario.
    pub fn ejecutar(&mut self, limite: u64) -> Result<Resumen, ErrorEjecucion> {
        let mut resumen = Resumen { instrucciones: 0, ciclos: 0 };
        while !self.estado.detenida() {
            if resumen.instrucciones >= limite {
                return Err(ErrorEjecucion::LimiteDeInstrucciones { limite });
            }
            if let Some(paso) = self.paso()? {
                resumen.instrucciones += 1;
                resumen.ciclos += paso.ciclos;
            }
        }
        Ok(resumen)
    }
}
