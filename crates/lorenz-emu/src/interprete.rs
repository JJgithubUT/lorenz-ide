//! Interprete del arbol de semantica (D-009).
//!
//! Las expresiones se evaluan con enteros de 128 bits con signo y se truncan al ancho del destino
//! solo al asignar (ver `docs/SEMANTICA.md`).

use lorenz_arch::modelo::{Instruccion, Maquina};
use lorenz_arch::semantica::ast::{Expr, OpBinario, OpUnario};

use crate::decodificador::Decodificada;
use crate::error::ErrorEjecucion;
use crate::estado::{mascara_de, Estado};
use crate::memoria::Memoria;
use crate::puertos::Puertos;

/// Las tres partes de la maquina que una instruccion puede leer y modificar.
pub struct Entorno<'a> {
    pub estado: &'a mut Estado,
    pub memoria: &'a mut Memoria,
    pub puertos: &'a mut Puertos,
}

/// Interprete de la semantica de las instrucciones de una maquina.
#[derive(Debug, Clone)]
pub struct Interprete {
    mascara_direccion: u64,
    mascara_palabra: u64,
}

/// Desplazamiento a la izquierda sin desbordar el entero de 128 bits.
fn desplazar_izq(a: i128, n: i128) -> i128 {
    if !(0..127).contains(&n) {
        0
    } else {
        a.wrapping_shl(n as u32)
    }
}

/// Desplazamiento aritmetico a la derecha.
fn desplazar_der(a: i128, n: i128) -> i128 {
    if n < 0 {
        0
    } else if n >= 127 {
        if a < 0 { -1 } else { 0 }
    } else {
        a >> n
    }
}

pub(crate) fn como_bool(v: i128) -> i128 {
    i128::from(v != 0)
}

impl Interprete {
    pub fn nuevo(maquina: &Maquina) -> Self {
        Interprete {
            mascara_direccion: mascara_de(maquina.bits_direccion),
            mascara_palabra: mascara_de(maquina.bits_palabra),
        }
    }

    pub(crate) fn mascara_palabra(&self) -> u64 {
        self.mascara_palabra
    }

    pub(crate) fn direccion(&self, valor: i128) -> u64 {
        (valor as u64) & self.mascara_direccion
    }

    /// Valor de un nombre: operando, registro, bandera o alias `pc` / `sp`.
    fn leer_nombre(
        &self,
        nombre: &str,
        dec: &Decodificada,
        entorno: &Entorno<'_>,
    ) -> Result<i128, ErrorEjecucion> {
        if let Some(o) = dec.operandos.iter().find(|o| o.nombre == nombre) {
            return match &o.registro {
                Some(registro) => Ok(entorno.estado.registro(registro)? as i128),
                None => Ok(o.valor),
            };
        }
        if let Ok(v) = entorno.estado.registro(nombre) {
            return Ok(v as i128);
        }
        if entorno.estado.tiene_bandera(nombre) {
            return Ok(i128::from(entorno.estado.bandera(nombre)?));
        }
        match nombre {
            "pc" => Ok(entorno.estado.pc() as i128),
            "sp" => entorno
                .estado
                .sp()
                .map(|v| v as i128)
                .ok_or_else(|| ErrorEjecucion::RegistroInexistente { nombre: nombre.to_string() }),
            _ => Err(ErrorEjecucion::RegistroInexistente { nombre: nombre.to_string() }),
        }
    }

    /// Evalua una expresion sin truncar el resultado.
    pub(crate) fn evaluar(
        &self,
        expr: &Expr,
        instruccion: &Instruccion,
        dec: &Decodificada,
        entorno: &mut Entorno<'_>,
    ) -> Result<i128, ErrorEjecucion> {
        match expr {
            Expr::Literal(n) => Ok(i128::from(*n)),
            Expr::Nombre(n) => self.leer_nombre(n, dec, entorno),
            Expr::Mem(direccion) => {
                let d = self.evaluar(direccion, instruccion, dec, entorno)?;
                Ok(i128::from(entorno.memoria.leer(self.direccion(d))?))
            }
            Expr::Entrada(puerto) => {
                let p = self.evaluar(puerto, instruccion, dec, entorno)?;
                Ok(i128::from(entorno.puertos.leer(p as u64)?))
            }
            Expr::Unario(op, e) => {
                let v = self.evaluar(e, instruccion, dec, entorno)?;
                Ok(match op {
                    OpUnario::NegacionLogica => i128::from(v == 0),
                    OpUnario::NegacionBits => !v,
                    OpUnario::Menos => v.wrapping_neg(),
                })
            }
            Expr::Binario(op, a, b) => self.evaluar_binario(*op, a, b, instruccion, dec, entorno),
        }
    }

    fn evaluar_binario(
        &self,
        op: OpBinario,
        a: &Expr,
        b: &Expr,
        instruccion: &Instruccion,
        dec: &Decodificada,
        entorno: &mut Entorno<'_>,
    ) -> Result<i128, ErrorEjecucion> {
        let x = self.evaluar(a, instruccion, dec, entorno)?;
        // Los operadores logicos no evaluan el lado derecho si el izquierdo ya decide.
        match op {
            OpBinario::YLogico if x == 0 => return Ok(0),
            OpBinario::OLogico if x != 0 => return Ok(1),
            _ => {}
        }
        let y = self.evaluar(b, instruccion, dec, entorno)?;
        let cero = || ErrorEjecucion::DivisionEntreCero { mnemonico: instruccion.mnemonico.clone() };
        Ok(match op {
            OpBinario::Suma => x.wrapping_add(y),
            OpBinario::Resta => x.wrapping_sub(y),
            OpBinario::Producto => x.wrapping_mul(y),
            OpBinario::Division => x.checked_div(y).ok_or_else(cero)?,
            OpBinario::Modulo => x.checked_rem(y).ok_or_else(cero)?,
            OpBinario::DesplazaIzq => desplazar_izq(x, y),
            OpBinario::DesplazaDer => desplazar_der(x, y),
            OpBinario::Y => x & y,
            OpBinario::O => x | y,
            OpBinario::Xor => x ^ y,
            OpBinario::Igual => i128::from(x == y),
            OpBinario::Distinto => i128::from(x != y),
            OpBinario::Menor => i128::from(x < y),
            OpBinario::MenorIgual => i128::from(x <= y),
            OpBinario::Mayor => i128::from(x > y),
            OpBinario::MayorIgual => i128::from(x >= y),
            OpBinario::YLogico | OpBinario::OLogico => como_bool(y),
        })
    }
}
