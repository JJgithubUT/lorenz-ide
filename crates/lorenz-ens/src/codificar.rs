//! De los valores de los operandos a las palabras de una instruccion.
//!
//! Es la inversa de la decodificacion del emulador: los bits fijos salen de la codificacion y cada
//! campo recibe su operando, repartido entre fragmentos si el campo aparece en varias palabras.
//! El primer fragmento es el menos significativo en una maquina little endian y el mas
//! significativo en una big endian.

use std::collections::BTreeMap;

use lorenz_arch::modelo::{Endianness, Instruccion, Maquina, Operando, TipoOperando};

use crate::error::Causa;

fn mascara(bits: u32) -> u128 {
    if bits >= 128 {
        u128::MAX
    } else {
        (1u128 << bits) - 1
    }
}

/// Convierte el valor escrito por el usuario en el valor que se guarda en el campo.
///
/// `direccion` es donde empieza la instruccion y `longitud` cuantas palabras ocupa, necesarias para
/// los saltos relativos: el contador de programa ya apunta a la siguiente instruccion cuando la
/// semantica se ejecuta, asi que el desplazamiento se mide desde `direccion + longitud`.
pub fn valor_de_campo(
    maquina: &Maquina,
    operando: &Operando,
    valor: i128,
    bits: u32,
    direccion: u64,
    longitud: usize,
) -> Result<u128, Causa> {
    let nombre = operando.nombre.clone();
    match &operando.tipo {
        TipoOperando::Registro { permitidos } => {
            // El valor ya es el indice dentro de la lista de permitidos.
            if valor < 0 || valor as usize >= permitidos.len() {
                return Err(Causa::Otra(format!(
                    "el indice de registro {valor} no existe en `{nombre}`"
                )));
            }
            comprobar_sin_signo(&nombre, valor, bits)
        }
        TipoOperando::Inmediato { signo, rango } => {
            if valor < rango.0 as i128 || valor > rango.1 as i128 {
                return Err(Causa::InmediatoFueraDeRango {
                    operando: nombre,
                    valor,
                    minimo: rango.0,
                    maximo: rango.1,
                });
            }
            if *signo {
                comprobar_con_signo(&nombre, valor, bits)
            } else {
                comprobar_sin_signo(&nombre, valor, bits)
            }
        }
        TipoOperando::Direccion => {
            let maximo = maquina.direccion_maxima();
            if valor < 0 || valor > maximo as i128 {
                return Err(Causa::DireccionFueraDeRango { valor, maximo });
            }
            comprobar_sin_signo(&nombre, valor, bits)
        }
        TipoOperando::Desplazamiento => {
            let desplazamiento = valor - (direccion as i128 + longitud as i128);
            let (minimo, maximo) = limites_con_signo(bits);
            if desplazamiento < minimo || desplazamiento > maximo {
                return Err(Causa::SaltoFueraDeAlcance {
                    desplazamiento,
                    minimo,
                    maximo,
                });
            }
            Ok(desplazamiento as u128 & mascara(bits))
        }
        TipoOperando::Puerto => comprobar_sin_signo(&nombre, valor, bits),
    }
}

fn limites_con_signo(bits: u32) -> (i128, i128) {
    if bits == 0 {
        return (0, 0);
    }
    (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1)
}

fn comprobar_sin_signo(nombre: &str, valor: i128, bits: u32) -> Result<u128, Causa> {
    if valor < 0 || (bits < 127 && valor >> bits != 0) {
        return Err(Causa::NoCabeEnElCampo {
            operando: nombre.to_string(),
            valor,
            bits,
        });
    }
    Ok(valor as u128)
}

fn comprobar_con_signo(nombre: &str, valor: i128, bits: u32) -> Result<u128, Causa> {
    let (minimo, maximo) = limites_con_signo(bits);
    if valor < minimo || valor > maximo {
        return Err(Causa::NoCabeEnElCampo {
            operando: nombre.to_string(),
            valor,
            bits,
        });
    }
    Ok(valor as u128 & mascara(bits))
}

/// Arma las palabras de una instruccion.
///
/// `valores` asocia el nombre de cada operando con su valor ya resuelto (indice para un registro,
/// numero para el resto). El error trae el nombre del operando culpable y la causa.
pub fn codificar(
    maquina: &Maquina,
    instruccion: &Instruccion,
    valores: &BTreeMap<String, i128>,
    direccion: u64,
) -> Result<Vec<u64>, (String, Causa)> {
    let longitud = instruccion.codificacion.longitud_en_palabras();
    let mut palabras: Vec<u64> = instruccion
        .codificacion
        .palabras()
        .iter()
        .map(|p| p.valor_fijo())
        .collect();
    for operando in &instruccion.operandos {
        let valor = *valores.get(&operando.nombre).ok_or_else(|| {
            (
                operando.nombre.clone(),
                Causa::Otra(format!("falta el valor del operando `{}`", operando.nombre)),
            )
        })?;
        let fragmentos = instruccion.codificacion.fragmentos(&operando.nombre);
        let bits: u32 = fragmentos.iter().map(|f| f.bits).sum();
        let mut crudo = valor_de_campo(maquina, operando, valor, bits, direccion, longitud)
            .map_err(|c| (operando.nombre.clone(), c))?;
        let orden: Vec<_> = match maquina.endianness {
            Endianness::Little => fragmentos.iter().collect(),
            Endianness::Big => fragmentos.iter().rev().collect(),
        };
        for f in orden {
            let parte = (crudo & mascara(f.bits)) as u64;
            palabras[f.palabra] |= parte << f.desplazamiento;
            crudo >>= f.bits;
        }
    }
    Ok(palabras)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn ejemplo() -> Maquina {
        let t = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
        lorenz_arch::cargar(&t).unwrap()
    }

    fn vals(pares: &[(&str, i128)]) -> BTreeMap<String, i128> {
        pares.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn instruccion_sin_operandos_son_sus_bits_fijos() {
        let m = ejemplo();
        let hlt = m.instruccion("HLT").unwrap();
        assert_eq!(codificar(&m, hlt, &vals(&[]), 0).unwrap(), vec![0xF0]);
    }

    #[test]
    fn registros_se_codifican_por_posicion() {
        let m = ejemplo();
        let add = m.instruccion("ADD").unwrap();
        // 0011 rd:2 rs:2 con rd=A(0), rs=B(1)
        assert_eq!(
            codificar(&m, add, &vals(&[("rd", 0), ("rs", 1)]), 0).unwrap(),
            vec![0x31]
        );
    }

    #[test]
    fn inmediato_ocupa_la_segunda_palabra() {
        let m = ejemplo();
        let ldi = m.instruccion("LDI").unwrap();
        assert_eq!(
            codificar(&m, ldi, &vals(&[("rd", 1), ("imm", 200)]), 0).unwrap(),
            vec![0x24, 200]
        );
    }

    #[test]
    fn direccion_de_16_bits_en_dos_palabras_little_endian() {
        let m = ejemplo();
        let ld = m.instruccion("LD").unwrap();
        // dir:8 | dir:8, little endian: el primer fragmento es el byte bajo.
        assert_eq!(
            codificar(&m, ld, &vals(&[("rd", 2), ("dir", 0x2010)]), 0).unwrap(),
            vec![0x48, 0x10, 0x20]
        );
    }

    #[test]
    fn inmediato_fuera_de_rango_indica_los_limites() {
        let m = ejemplo();
        let ldi = m.instruccion("LDI").unwrap();
        let (op, causa) = codificar(&m, ldi, &vals(&[("rd", 0), ("imm", 256)]), 0).unwrap_err();
        assert_eq!(op, "imm");
        assert!(matches!(
            causa,
            Causa::InmediatoFueraDeRango {
                minimo: 0,
                maximo: 255,
                valor: 256,
                ..
            }
        ));
        assert!(codificar(&m, ldi, &vals(&[("rd", 0), ("imm", -1)]), 0).is_err());
    }

    #[test]
    fn direccion_mayor_que_el_espacio_se_rechaza() {
        let m = ejemplo();
        let ld = m.instruccion("LD").unwrap();
        let (_, causa) = codificar(&m, ld, &vals(&[("rd", 0), ("dir", 0x10000)]), 0).unwrap_err();
        assert!(matches!(
            causa,
            Causa::DireccionFueraDeRango { maximo: 0xFFFF, .. }
        ));
    }

    #[test]
    fn falta_un_operando_es_error() {
        let m = ejemplo();
        let add = m.instruccion("ADD").unwrap();
        assert!(codificar(&m, add, &vals(&[("rd", 0)]), 0).is_err());
    }
}
