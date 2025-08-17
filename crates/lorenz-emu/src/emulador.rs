//! El emulador completo: junta memoria, estado, puertos, decodificador e interprete y ejecuta
//! programas instruccion por instruccion.

use std::collections::BTreeSet;

use lorenz_arch::modelo::{Ciclos, Endianness, Maquina, SentidoPila};

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
    pendientes: BTreeSet<u32>,
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
            pendientes: BTreeSet::new(),
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
    pub fn cargar_programa(
        &mut self,
        direccion: u64,
        palabras: &[u64],
    ) -> Result<(), ErrorEjecucion> {
        self.memoria.cargar_bloque(direccion, palabras)
    }

    /// Vuelve registros, contadores y puertos a su estado inicial. La memoria no se toca.
    pub fn reiniciar(&mut self) {
        self.estado.reiniciar();
        self.puertos.reiniciar();
        self.pendientes.clear();
    }

    /// Levanta una linea de interrupcion. Se atiende antes de la siguiente instruccion si las
    /// interrupciones estan habilitadas; mientras tanto queda pendiente.
    ///
    /// Si varias lineas esperan, se atiende primero la de numero menor (D-019).
    pub fn levantar_interrupcion(&mut self, linea: u32) -> Result<(), ErrorEjecucion> {
        let declarada = self
            .maquina
            .interrupciones
            .as_ref()
            .is_some_and(|i| i.vectores.contains_key(&linea));
        if !declarada {
            return Err(ErrorEjecucion::InterrupcionNoDeclarada { linea });
        }
        self.pendientes.insert(linea);
        Ok(())
    }

    /// Lineas de interrupcion que esperan ser atendidas, de menor a mayor.
    pub fn interrupciones_pendientes(&self) -> Vec<u32> {
        self.pendientes.iter().copied().collect()
    }

    /// Palabras que hacen falta para guardar un valor de `bits` bits.
    fn palabras_para(&self, bits: u32) -> u32 {
        bits.div_ceil(self.maquina.bits_palabra)
    }

    /// Escribe `valor` en `palabras` palabras consecutivas desde `direccion`, en el orden del
    /// `endianness` de la maquina.
    fn escribir_valor(&mut self, direccion: u64, valor: u64, palabras: u32) -> Result<(), ErrorEjecucion> {
        let bw = self.maquina.bits_palabra;
        for i in 0..palabras {
            let posicion = match self.maquina.endianness {
                Endianness::Little => i,
                Endianness::Big => palabras - 1 - i,
            };
            let palabra = valor.checked_shr(posicion * bw).unwrap_or(0);
            self.memoria.escribir(direccion.wrapping_add(u64::from(i)), palabra)?;
        }
        Ok(())
    }

    /// Lee un valor de `palabras` palabras consecutivas desde `direccion`.
    fn leer_valor(&self, direccion: u64, palabras: u32) -> Result<u64, ErrorEjecucion> {
        let bw = self.maquina.bits_palabra;
        let mut valor = 0u64;
        for i in 0..palabras {
            let palabra = self.memoria.leer(direccion.wrapping_add(u64::from(i)))?;
            let posicion = match self.maquina.endianness {
                Endianness::Little => i,
                Endianness::Big => palabras - 1 - i,
            };
            valor |= palabra.checked_shl(posicion * bw).unwrap_or(0);
        }
        Ok(valor)
    }

    /// Atiende la interrupcion pendiente de menor numero, si hay y estan habilitadas.
    fn atender_interrupcion(&mut self) -> Result<bool, ErrorEjecucion> {
        if !self.estado.interrupciones_habilitadas() {
            return Ok(false);
        }
        let Some(&linea) = self.pendientes.iter().next() else {
            return Ok(false);
        };
        let vector = self.maquina.interrupciones.as_ref().and_then(|i| i.vectores.get(&linea)).copied();
        let vector = vector.ok_or(ErrorEjecucion::InterrupcionNoDeclarada { linea })?;
        let sp = self.estado.sp().ok_or(ErrorEjecucion::InterrupcionSinPila { linea })?;
        let nombre_pc = self.estado.nombre_del_pc().to_string();
        let palabras_pc = self.palabras_para(self.estado.mascara_de_registro(&nombre_pc)?.count_ones());
        let palabras_dir = self.palabras_para(self.maquina.bits_direccion);
        let pc = self.estado.pc();
        let destino = self.leer_valor(vector, palabras_dir)?;
        match self.maquina.pila {
            SentidoPila::Descendente => {
                let nuevo = sp.wrapping_sub(u64::from(palabras_pc));
                self.escribir_valor(nuevo, pc, palabras_pc)?;
                self.estado.fijar_sp(nuevo);
            }
            SentidoPila::Ascendente => {
                self.escribir_valor(sp, pc, palabras_pc)?;
                self.estado.fijar_sp(sp.wrapping_add(u64::from(palabras_pc)));
            }
        }
        self.pendientes.remove(&linea);
        self.estado.habilitar_interrupciones(false);
        self.estado.fijar_pc(destino);
        Ok(true)
    }

    /// Ejecuta una instruccion. Devuelve `None` si la maquina ya estaba detenida.
    ///
    /// Si la instruccion falla, el contador de programa vuelve a apuntar a ella para que el
    /// depurador muestre donde ocurrio el error.
    pub fn paso(&mut self) -> Result<Option<Paso>, ErrorEjecucion> {
        if self.estado.detenida() {
            return Ok(None);
        }
        self.atender_interrupcion()?;
        let direccion = self.estado.pc();
        let dec = self.decodificador.decodificar(&self.memoria, direccion)?;
        self.estado
            .fijar_pc(direccion.wrapping_add(dec.longitud as u64));
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
                if resultado.salto {
                    tomado
                } else {
                    no_tomado
                }
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
        let mut resumen = Resumen {
            instrucciones: 0,
            ciclos: 0,
        };
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
