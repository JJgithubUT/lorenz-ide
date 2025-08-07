//! Decodificacion de instrucciones: de palabras de memoria a una instruccion con sus operandos.

use lorenz_arch::bits::Codificacion;
use lorenz_arch::modelo::{Endianness, Maquina, TipoOperando};

use crate::error::ErrorEjecucion;
use crate::estado::mascara_de;
use crate::memoria::Memoria;

/// Un operando ya decodificado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperandoDecodificado {
    pub nombre: String,
    /// Valor numerico: con signo si el operando lo tiene, sin signo en los demas casos.
    /// Para un registro, el indice dentro de la lista de permitidos.
    pub valor: i128,
    /// Nombre del registro, solo para operandos de tipo registro.
    pub registro: Option<String>,
}

/// Una instruccion leida de memoria.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decodificada {
    /// Posicion de la instruccion en la lista de la maquina.
    pub indice: usize,
    /// Direccion de su primera palabra.
    pub direccion: u64,
    /// Cuantas palabras ocupa.
    pub longitud: usize,
    pub operandos: Vec<OperandoDecodificado>,
}

#[derive(Debug, Clone)]
struct Entrada {
    codificacion: Codificacion,
    mascara: u64,
    valor: u64,
    operandos: Vec<(String, TipoOperando)>,
}

/// Tabla de decodificacion construida a partir de las codificaciones de la maquina.
#[derive(Debug, Clone)]
pub struct Decodificador {
    entradas: Vec<Entrada>,
    endianness: Endianness,
    mascara_direccion: u64,
}

fn extender_signo(valor: u64, bits: u32) -> i128 {
    if bits == 0 || bits >= 64 {
        return valor as i64 as i128;
    }
    if valor >> (bits - 1) & 1 == 1 {
        valor as i128 - (1i128 << bits)
    } else {
        valor as i128
    }
}

impl Decodificador {
    pub fn nuevo(maquina: &Maquina) -> Self {
        let entradas = maquina
            .instrucciones
            .iter()
            .map(|i| {
                let primera = &i.codificacion.palabras()[0];
                Entrada {
                    codificacion: i.codificacion.clone(),
                    mascara: primera.mascara(),
                    valor: primera.valor_fijo(),
                    operandos: i.operandos.iter().map(|o| (o.nombre.clone(), o.tipo.clone())).collect(),
                }
            })
            .collect();
        Decodificador {
            entradas,
            endianness: maquina.endianness,
            mascara_direccion: mascara_de(maquina.bits_direccion),
        }
    }

    fn buscar(&self, primera: u64) -> Option<usize> {
        self.entradas.iter().position(|e| primera & e.mascara == e.valor)
    }

    /// Cuantas palabras ocupa la instruccion que empieza con `primera`; `None` si ninguna encaja.
    pub fn longitud_de(&self, primera: u64) -> Option<usize> {
        self.buscar(primera).map(|i| self.entradas[i].codificacion.longitud_en_palabras())
    }

    /// Valor de un campo juntando sus fragmentos (D-014).
    fn valor_de_campo(&self, entrada: &Entrada, nombre: &str, palabras: &[u64]) -> u64 {
        let fragmentos = entrada.codificacion.fragmentos(nombre);
        let mut total = 0u64;
        let mut acumulados = 0u32;
        let ordenados: Vec<_> = match self.endianness {
            Endianness::Little => fragmentos.iter().collect(),
            Endianness::Big => fragmentos.iter().rev().collect(),
        };
        for f in ordenados {
            let parte = (palabras[f.palabra] >> f.desplazamiento) & mascara_de(f.bits);
            total |= parte << acumulados;
            acumulados += f.bits;
        }
        total
    }

    /// Decodifica una instruccion a partir de sus palabras ya leidas.
    ///
    /// `palabras` debe tener al menos la longitud de la instruccion; las sobrantes se ignoran.
    pub fn decodificar_palabras(&self, direccion: u64, palabras: &[u64]) -> Result<Decodificada, ErrorEjecucion> {
        let invalida = |palabra| ErrorEjecucion::InstruccionInvalida { direccion, palabra };
        let primera = *palabras.first().ok_or_else(|| invalida(0))?;
        let indice = self.buscar(primera).ok_or_else(|| invalida(primera))?;
        let entrada = &self.entradas[indice];
        let longitud = entrada.codificacion.longitud_en_palabras();
        if palabras.len() < longitud {
            return Err(invalida(primera));
        }
        // Los bits fijos de las palabras siguientes tambien deben coincidir.
        for (n, palabra) in entrada.codificacion.palabras().iter().enumerate().skip(1) {
            if palabras[n] & palabra.mascara() != palabra.valor_fijo() {
                return Err(invalida(palabras[n]));
            }
        }
        let mut operandos = Vec::with_capacity(entrada.operandos.len());
        for (nombre, tipo) in &entrada.operandos {
            let bits = entrada.codificacion.bits_totales(nombre);
            let crudo = self.valor_de_campo(entrada, nombre, palabras);
            let (valor, registro) = match tipo {
                TipoOperando::Registro { permitidos } => {
                    let nombre_reg = permitidos.get(crudo as usize).ok_or_else(|| invalida(primera))?;
                    (crudo as i128, Some(nombre_reg.clone()))
                }
                TipoOperando::Inmediato { signo: true, .. } | TipoOperando::Desplazamiento => {
                    (extender_signo(crudo, bits), None)
                }
                _ => (crudo as i128, None),
            };
            operandos.push(OperandoDecodificado { nombre: nombre.clone(), valor, registro });
        }
        Ok(Decodificada { indice, direccion, longitud, operandos })
    }

    /// Lee y decodifica la instruccion que empieza en `direccion`.
    pub fn decodificar(&self, memoria: &Memoria, direccion: u64) -> Result<Decodificada, ErrorEjecucion> {
        let primera = memoria.leer(direccion & self.mascara_direccion)?;
        let longitud = self.longitud_de(primera).ok_or(ErrorEjecucion::InstruccionInvalida { direccion, palabra: primera })?;
        let mut palabras = Vec::with_capacity(longitud);
        palabras.push(primera);
        for n in 1..longitud {
            palabras.push(memoria.leer(direccion.wrapping_add(n as u64) & self.mascara_direccion)?);
        }
        self.decodificar_palabras(direccion, &palabras)
    }
}
