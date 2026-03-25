//! La aritmetica de SGT, definida una sola vez (RF-SGT-020).
//!
//! El plegado de constantes, el interprete de la representacion intermedia y las pruebas usan esta
//! misma tabla, de modo que el comportamiento del desbordamiento no depende de quien lo calcule.
//!
//! Reglas:
//! - `+`, `-`, `*` y `<<` dan la vuelta: el resultado es el valor modulo 2^n.
//! - `+|` y `-|` saturan en el extremo del rango del tipo.
//! - `/` y `%` truncan hacia cero; dividir entre cero es un error.
//! - Un desplazamiento de `n` o mas bits da cero (o -1 si es `>>` de un negativo con signo).
//! - Las comparaciones respetan el signo del tipo y devuelven 0 o 1.

use crate::ast::{OpBinario, OpUnario};
use crate::tipos::{envolver, rango};

/// Por que no se pudo calcular una operacion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorAritmetica {
    DivisionPorCero,
}

/// Calcula `a op b` para enteros de `bits` bits. `a` y `b` ya estan envueltos al tipo.
pub fn binario(
    op: OpBinario,
    a: i128,
    b: i128,
    bits: u32,
    signo: bool,
) -> Result<i128, ErrorAritmetica> {
    let (min, max) = rango(bits, signo);
    let ajustar = |v: i128| envolver(v, bits, signo);
    Ok(match op {
        OpBinario::Suma => ajustar(a + b),
        OpBinario::Resta => ajustar(a - b),
        OpBinario::Mul => ajustar(a.wrapping_mul(b)),
        OpBinario::SumaSat => (a + b).clamp(min, max),
        OpBinario::RestaSat => (a - b).clamp(min, max),
        OpBinario::Div => {
            if b == 0 {
                return Err(ErrorAritmetica::DivisionPorCero);
            }
            ajustar(a / b)
        }
        OpBinario::Mod => {
            if b == 0 {
                return Err(ErrorAritmetica::DivisionPorCero);
            }
            ajustar(a % b)
        }
        OpBinario::Y => ajustar(a & b),
        OpBinario::O => ajustar(a | b),
        OpBinario::Xor => ajustar(a ^ b),
        OpBinario::DespIzq => {
            if b < 0 || b >= i128::from(bits) {
                0
            } else {
                ajustar(a << b)
            }
        }
        OpBinario::DespDer => {
            if b < 0 || b >= i128::from(bits) {
                if signo && a < 0 {
                    -1
                } else {
                    0
                }
            } else {
                // En un tipo sin signo `a` no es negativo, asi que el desplazamiento es logico.
                ajustar(a >> b)
            }
        }
        OpBinario::Igual => i128::from(a == b),
        OpBinario::Distinto => i128::from(a != b),
        OpBinario::Menor => i128::from(a < b),
        OpBinario::MenorIgual => i128::from(a <= b),
        OpBinario::Mayor => i128::from(a > b),
        OpBinario::MayorIgual => i128::from(a >= b),
        OpBinario::YLogico => i128::from(a != 0 && b != 0),
        OpBinario::OLogico => i128::from(a != 0 || b != 0),
    })
}

pub fn unario(op: OpUnario, a: i128, bits: u32, signo: bool) -> i128 {
    match op {
        OpUnario::Negar => envolver(-a, bits, signo),
        OpUnario::Complemento => envolver(!a, bits, signo),
        OpUnario::No => i128::from(a == 0),
    }
}
