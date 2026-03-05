//! Interprete del arbol de semantica (D-009).
//!
//! Las expresiones se evaluan con enteros de 128 bits con signo y se truncan al ancho del destino
//! solo al asignar (ver `docs/SEMANTICA.md`).

use lorenz_arch::modelo::{Instruccion, Maquina};
use lorenz_arch::semantica::ast::{Destino, Expr, OpBinario, OpUnario, Programa, Sentencia};

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

/// Lo que paso al ejecutar una instruccion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Resultado {
    /// La instruccion asigno el contador de programa.
    pub salto: bool,
    /// La instruccion detuvo la maquina.
    pub detenida: bool,
}

/// Seguimiento de las asignaciones de una instruccion, para calcular las banderas (D-013).
#[derive(Debug, Default)]
struct Seguimiento {
    /// Ultimo valor asignado a un destino que no es una bandera: sin truncar, truncado y ancho.
    ultimo: Option<(i128, u64, u32)>,
    /// Si la expresion de esa asignacion terminaba en `+`, `-`, `*` o `<<`.
    con_acarreo: bool,
    /// Banderas asignadas de forma explicita, que mandan sobre la convencion.
    explicitas: Vec<String>,
    resultado: Resultado,
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
        if a < 0 {
            -1
        } else {
            0
        }
    } else {
        a >> n
    }
}

/// Aplica un operador binario a dos valores ya evaluados. `None` si divide entre cero.
pub(crate) fn aplicar_binario(op: OpBinario, x: i128, y: i128) -> Option<i128> {
    Some(match op {
        OpBinario::Suma => x.wrapping_add(y),
        OpBinario::Resta => x.wrapping_sub(y),
        OpBinario::Producto => x.wrapping_mul(y),
        OpBinario::Division => x.checked_div(y)?,
        OpBinario::Modulo => x.checked_rem(y)?,
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

    pub(crate) fn direccion(&self, valor: i128) -> u64 {
        (valor as u64) & self.mascara_direccion
    }

    /// Valor de un nombre: operando, registro, bandera o alias `pc` / `sp`.
    fn leer_nombre(
        &self,
        nombre: &str,
        dec: &Decodificada,
        entorno: &mut Entorno<'_>,
    ) -> Result<i128, ErrorEjecucion> {
        if let Some(o) = dec.operandos.iter().find(|o| o.nombre == nombre) {
            return match &o.registro {
                Some(registro) => {
                    entorno.estado.registrar_lectura(registro, dec.direccion);
                    Ok(entorno.estado.registro(registro)? as i128)
                }
                None => Ok(o.valor),
            };
        }
        if let Ok(v) = entorno.estado.registro(nombre) {
            entorno.estado.registrar_lectura(nombre, dec.direccion);
            return Ok(v as i128);
        }
        if entorno.estado.tiene_bandera(nombre) {
            return Ok(i128::from(entorno.estado.bandera(nombre)?));
        }
        match nombre {
            "pc" => Ok(entorno.estado.pc() as i128),
            "ie" => Ok(i128::from(entorno.estado.interrupciones_habilitadas())),
            "sp" => entorno.estado.sp().map(|v| v as i128).ok_or_else(|| {
                ErrorEjecucion::RegistroInexistente {
                    nombre: nombre.to_string(),
                }
            }),
            _ => Err(ErrorEjecucion::RegistroInexistente {
                nombre: nombre.to_string(),
            }),
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
                Ok(i128::from(
                    entorno.memoria.leer_vigilada(self.direccion(d))?,
                ))
            }
            Expr::Entrada(puerto) => {
                let p = self.evaluar(puerto, instruccion, dec, entorno)?;
                if let Some(banco) = entorno.memoria.banco_de_puerto(p as u64) {
                    return Ok(i128::from(banco));
                }
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
        aplicar_binario(op, x, y).ok_or_else(|| ErrorEjecucion::DivisionEntreCero {
            mnemonico: instruccion.mnemonico.clone(),
        })
    }

    /// Escribe `valor` en el destino y devuelve el valor truncado y el ancho en bits del destino.
    fn asignar(
        &self,
        destino: &Destino,
        valor: i128,
        instruccion: &Instruccion,
        dec: &Decodificada,
        entorno: &mut Entorno<'_>,
        seg: &mut Seguimiento,
    ) -> Result<(), ErrorEjecucion> {
        let mut registrar = |truncado: u64, mascara: u64, seg: &mut Seguimiento| {
            seg.ultimo = Some((valor, truncado, mascara.count_ones()));
        };
        match destino {
            Destino::Nombre(nombre) => {
                if let Some(o) = dec.operandos.iter().find(|o| &o.nombre == nombre) {
                    let registro =
                        o.registro
                            .clone()
                            .ok_or_else(|| ErrorEjecucion::RegistroInexistente {
                                nombre: nombre.clone(),
                            })?;
                    return self.asignar_registro(&registro, valor, entorno, seg, &mut registrar);
                }
                if entorno.estado.registro(nombre).is_ok() {
                    return self.asignar_registro(nombre, valor, entorno, seg, &mut registrar);
                }
                if entorno.estado.tiene_bandera(nombre) {
                    entorno.estado.fijar_bandera(nombre, valor != 0)?;
                    seg.explicitas.push(nombre.clone());
                    return Ok(());
                }
                if nombre == "ie" {
                    entorno.estado.habilitar_interrupciones(valor != 0);
                    return Ok(());
                }
                let alias = match nombre.as_str() {
                    "pc" => Some(entorno.estado.nombre_del_pc().to_string()),
                    "sp" => entorno.estado.nombre_del_sp().map(str::to_string),
                    _ => None,
                };
                match alias {
                    Some(registro) => {
                        self.asignar_registro(&registro, valor, entorno, seg, &mut registrar)
                    }
                    None => Err(ErrorEjecucion::RegistroInexistente {
                        nombre: nombre.clone(),
                    }),
                }
            }
            Destino::Mem(direccion) => {
                let d = self.evaluar(direccion, instruccion, dec, entorno)?;
                let truncado = (valor as u64) & self.mascara_palabra;
                let direccion = self.direccion(d);
                entorno.memoria.escribir(direccion, truncado)?;
                let mapeado = entorno
                    .memoria
                    .dispositivo_de(direccion)
                    .map(str::to_string);
                if let Some(dispositivo) = mapeado {
                    let desplazamiento = entorno.memoria.desplazamiento_en_region(direccion);
                    entorno.puertos.escribir_region(
                        &dispositivo,
                        desplazamiento.unwrap_or(0),
                        truncado,
                    )?;
                }
                registrar(truncado, self.mascara_palabra, seg);
                Ok(())
            }
            Destino::Salida(puerto) => {
                let p = self.evaluar(puerto, instruccion, dec, entorno)?;
                let truncado = (valor as u64) & self.mascara_palabra;
                if entorno.memoria.es_selector_de_banco(p as u64) {
                    entorno.memoria.seleccionar_banco(p as u64, truncado);
                    registrar(truncado, self.mascara_palabra, seg);
                    return Ok(());
                }
                entorno.puertos.escribir(p as u64, truncado)?;
                registrar(truncado, self.mascara_palabra, seg);
                Ok(())
            }
        }
    }

    fn asignar_registro(
        &self,
        registro: &str,
        valor: i128,
        entorno: &mut Entorno<'_>,
        seg: &mut Seguimiento,
        registrar: &mut impl FnMut(u64, u64, &mut Seguimiento),
    ) -> Result<(), ErrorEjecucion> {
        let mascara = entorno.estado.mascara_de_registro(registro)?;
        let truncado = (valor as u64) & mascara;
        entorno.estado.fijar_registro(registro, truncado)?;
        registrar(truncado, mascara, seg);
        if registro == entorno.estado.nombre_del_pc() {
            seg.resultado.salto = true;
        }
        Ok(())
    }

    fn ejecutar_sentencias(
        &self,
        sentencias: &[Sentencia],
        instruccion: &Instruccion,
        dec: &Decodificada,
        entorno: &mut Entorno<'_>,
        seg: &mut Seguimiento,
    ) -> Result<(), ErrorEjecucion> {
        for sentencia in sentencias {
            match sentencia {
                Sentencia::Detener => {
                    entorno.estado.detener();
                    seg.resultado.detenida = true;
                    return Ok(());
                }
                Sentencia::Si(condicion, cuerpo) => {
                    if self.evaluar(condicion, instruccion, dec, entorno)? != 0 {
                        self.ejecutar_sentencias(cuerpo, instruccion, dec, entorno, seg)?;
                        if seg.resultado.detenida {
                            return Ok(());
                        }
                    }
                }
                Sentencia::Asignar(destino, expr) => {
                    let valor = self.evaluar(expr, instruccion, dec, entorno)?;
                    let acarreo =
                        matches!(expr, Expr::Binario(op, _, _) if op.puede_producir_acarreo());
                    self.asignar(destino, valor, instruccion, dec, entorno, seg)?;
                    if !matches!(destino, Destino::Nombre(n) if entorno.estado.tiene_bandera(n)) {
                        seg.con_acarreo = acarreo;
                    }
                }
            }
        }
        Ok(())
    }

    /// Aplica las convenciones Z, N y C de las banderas que la instruccion declara en `afecta`.
    fn actualizar_banderas(
        &self,
        instruccion: &Instruccion,
        entorno: &mut Entorno<'_>,
        seg: &Seguimiento,
    ) -> Result<(), ErrorEjecucion> {
        let Some((sin_truncar, truncado, ancho)) = seg.ultimo else {
            return Ok(());
        };
        for nombre in &instruccion.afecta {
            if seg.explicitas.contains(nombre) || !entorno.estado.tiene_bandera(nombre) {
                continue;
            }
            let encendida = match nombre.as_str() {
                "Z" => truncado == 0,
                "N" => ancho > 0 && (truncado >> (ancho - 1)) & 1 == 1,
                "C" => seg.con_acarreo && (sin_truncar < 0 || sin_truncar >> ancho != 0),
                _ => continue,
            };
            entorno.estado.fijar_bandera(nombre, encendida)?;
        }
        Ok(())
    }

    /// Ejecuta la semantica de una instruccion ya decodificada.
    ///
    /// El contador de programa debe apuntar ya a la instruccion siguiente: `pc` en la semantica
    /// vale esa direccion y asignarlo es un salto.
    pub fn ejecutar(
        &self,
        instruccion: &Instruccion,
        dec: &Decodificada,
        entorno: &mut Entorno<'_>,
    ) -> Result<Resultado, ErrorEjecucion> {
        let programa: &Programa = &instruccion.programa;
        let mut seg = Seguimiento::default();
        self.ejecutar_sentencias(&programa.sentencias, instruccion, dec, entorno, &mut seg)?;
        self.actualizar_banderas(instruccion, entorno, &seg)?;
        Ok(seg.resultado)
    }
}
