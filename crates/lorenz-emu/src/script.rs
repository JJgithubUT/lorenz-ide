//! Interprete de los scripts de los dispositivos genericos (RF-EMU-035).
//!
//! Usa el mismo arbol y los mismos operadores que las instrucciones, pero con un entorno propio:
//! `valor` y `puerto` (lo que el programa acaba de escribir y en donde), los puertos del
//! dispositivo por su nombre y `interrumpir`. No toca la memoria ni los puertos de la maquina.

use std::collections::BTreeMap;

use lorenz_arch::semantica::{Destino, Expr, OpUnario, Programa, Sentencia};

use crate::error::ErrorEjecucion;
use crate::interprete::{aplicar_binario, como_bool};

/// Lo que un script puede ver y cambiar durante una ejecucion.
pub(crate) struct Entorno<'a> {
    /// Nombre de cada puerto del dispositivo y su numero.
    pub nombres: &'a BTreeMap<String, u64>,
    /// Valor actual de cada puerto, por numero. El script los modifica aqui.
    pub valores: &'a mut BTreeMap<u64, u64>,
    pub valor: u64,
    pub puerto: u64,
    pub mascara: u64,
    /// Se enciende si el script asigna un valor distinto de cero a `interrumpir`.
    pub interrumpir: bool,
    pub dispositivo: &'a str,
}

impl Entorno<'_> {
    fn leer(&self, nombre: &str) -> Result<i128, ErrorEjecucion> {
        match nombre {
            "valor" => Ok(i128::from(self.valor)),
            "puerto" => Ok(i128::from(self.puerto)),
            "interrumpir" => Ok(i128::from(self.interrumpir)),
            _ => match self.nombres.get(nombre) {
                Some(numero) => Ok(i128::from(self.valores.get(numero).copied().unwrap_or(0))),
                None => Err(ErrorEjecucion::NombreDeScriptDesconocido {
                    dispositivo: self.dispositivo.to_string(),
                    nombre: nombre.to_string(),
                }),
            },
        }
    }

    fn asignar(&mut self, nombre: &str, v: i128) -> Result<(), ErrorEjecucion> {
        if nombre == "interrumpir" {
            self.interrumpir = v != 0;
            return Ok(());
        }
        match self.nombres.get(nombre) {
            Some(numero) => {
                let truncado = (v as u64) & self.mascara;
                self.valores.insert(*numero, truncado);
                Ok(())
            }
            None => Err(ErrorEjecucion::NombreDeScriptDesconocido {
                dispositivo: self.dispositivo.to_string(),
                nombre: nombre.to_string(),
            }),
        }
    }
}

fn evaluar(e: &Expr, env: &Entorno<'_>) -> Result<i128, ErrorEjecucion> {
    match e {
        Expr::Literal(n) => Ok(i128::from(*n)),
        Expr::Nombre(n) => env.leer(n),
        Expr::Unario(op, a) => {
            let x = evaluar(a, env)?;
            Ok(match op {
                OpUnario::NegacionLogica => i128::from(x == 0),
                OpUnario::NegacionBits => !x,
                OpUnario::Menos => x.wrapping_neg(),
            })
        }
        Expr::Binario(op, a, b) => {
            use lorenz_arch::semantica::OpBinario::{OLogico, YLogico};
            let x = evaluar(a, env)?;
            match op {
                YLogico if x == 0 => return Ok(0),
                OLogico if x != 0 => return Ok(1),
                _ => {}
            }
            let y = evaluar(b, env)?;
            aplicar_binario(*op, x, y).ok_or_else(|| ErrorEjecucion::DivisionEntreCeroEnScript {
                dispositivo: env.dispositivo.to_string(),
            })
        }
        // La validacion de la arquitectura ya los rechaza.
        Expr::Mem(_) | Expr::Entrada(_) => Err(ErrorEjecucion::NombreDeScriptDesconocido {
            dispositivo: env.dispositivo.to_string(),
            nombre: e.to_string(),
        }),
    }
}

fn ejecutar_lista(lista: &[Sentencia], env: &mut Entorno<'_>) -> Result<(), ErrorEjecucion> {
    for s in lista {
        match s {
            Sentencia::Asignar(Destino::Nombre(n), v) => {
                let v = evaluar(v, env)?;
                env.asignar(n, v)?;
            }
            Sentencia::Asignar(otro, _) => {
                return Err(ErrorEjecucion::NombreDeScriptDesconocido {
                    dispositivo: env.dispositivo.to_string(),
                    nombre: otro.to_string(),
                })
            }
            Sentencia::Si(c, cuerpo) => {
                if como_bool(evaluar(c, env)?) != 0 {
                    ejecutar_lista(cuerpo, env)?;
                }
            }
            Sentencia::Detener => {}
        }
    }
    Ok(())
}

/// Ejecuta el script completo una vez.
pub(crate) fn ejecutar(programa: &Programa, env: &mut Entorno<'_>) -> Result<(), ErrorEjecucion> {
    ejecutar_lista(&programa.sentencias, env)
}
