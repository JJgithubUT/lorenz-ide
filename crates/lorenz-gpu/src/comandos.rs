//! Los comandos que la CPU envia a la unidad grafica (RF-GPU-010) y su codificacion en palabras.
//!
//! Un comando es una palabra de operacion seguida de un numero fijo de argumentos. Los argumentos
//! son enteros con signo; el color va como `0xRRGGBB`.

use crate::error::ErrorGpu;
use crate::raster::Vertice;

pub const OP_NADA: i64 = 0;
pub const OP_COLOR: i64 = 1;
pub const OP_LIMPIAR: i64 = 2;
pub const OP_RELLENAR: i64 = 3;
pub const OP_LINEA: i64 = 4;
pub const OP_TRIANGULO: i64 = 5;
pub const OP_TRIANGULO_COLOR: i64 = 6;
pub const OP_PROFUNDIDAD: i64 = 7;
pub const OP_PRUEBA_Z: i64 = 8;
pub const OP_FIN_LOTE: i64 = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comando {
    Nada,
    /// Fija el color de las primitivas siguientes.
    Color(u32),
    /// Borra toda la superficie con el color actual (y la profundidad).
    Limpiar,
    Rellenar {
        x: i64,
        y: i64,
        ancho: i64,
        alto: i64,
    },
    Linea {
        x0: i64,
        y0: i64,
        x1: i64,
        y1: i64,
    },
    /// Triangulo de color plano, con el color actual.
    Triangulo {
        v: [(i64, i64); 3],
    },
    /// Triangulo con el color de cada vertice interpolado.
    TrianguloColor {
        v: [Vertice; 3],
    },
    /// Fija la profundidad de las primitivas siguientes.
    Profundidad(u32),
    /// Activa o desactiva la prueba de profundidad.
    PruebaZ(bool),
    /// Marca el final de un lote: al llegar a el la unidad puede interrumpir a la CPU.
    FinLote,
}

impl Comando {
    pub fn nombre(&self) -> &'static str {
        match self {
            Self::Nada => "nada",
            Self::Color(_) => "color",
            Self::Limpiar => "limpiar",
            Self::Rellenar { .. } => "rellenar",
            Self::Linea { .. } => "linea",
            Self::Triangulo { .. } => "triangulo",
            Self::TrianguloColor { .. } => "triangulo_color",
            Self::Profundidad(_) => "profundidad",
            Self::PruebaZ(_) => "prueba_z",
            Self::FinLote => "fin_lote",
        }
    }

    /// Palabras que ocupa el comando, la operacion incluida.
    pub fn palabras_necesarias(operacion: i64) -> Option<(usize, &'static str)> {
        Some(match operacion {
            OP_NADA => (1, "nada"),
            OP_COLOR => (2, "color"),
            OP_LIMPIAR => (1, "limpiar"),
            OP_RELLENAR => (5, "rellenar"),
            OP_LINEA => (5, "linea"),
            OP_TRIANGULO => (7, "triangulo"),
            OP_TRIANGULO_COLOR => (10, "triangulo_color"),
            OP_PROFUNDIDAD => (2, "profundidad"),
            OP_PRUEBA_Z => (2, "prueba_z"),
            OP_FIN_LOTE => (1, "fin_lote"),
            _ => return None,
        })
    }

    pub fn a_palabras(&self) -> Vec<i64> {
        match *self {
            Self::Nada => vec![OP_NADA],
            Self::Color(c) => vec![OP_COLOR, c as i64],
            Self::Limpiar => vec![OP_LIMPIAR],
            Self::Rellenar { x, y, ancho, alto } => vec![OP_RELLENAR, x, y, ancho, alto],
            Self::Linea { x0, y0, x1, y1 } => vec![OP_LINEA, x0, y0, x1, y1],
            Self::Triangulo { v } => {
                vec![OP_TRIANGULO, v[0].0, v[0].1, v[1].0, v[1].1, v[2].0, v[2].1]
            }
            Self::TrianguloColor { v } => {
                let mut p = vec![OP_TRIANGULO_COLOR];
                for q in v {
                    p.extend([q.x, q.y, q.color as i64]);
                }
                p
            }
            Self::Profundidad(z) => vec![OP_PROFUNDIDAD, z as i64],
            Self::PruebaZ(b) => vec![OP_PRUEBA_Z, b as i64],
            Self::FinLote => vec![OP_FIN_LOTE],
        }
    }

    /// Decodifica el primer comando de `p`; devuelve tambien cuantas palabras uso.
    pub fn decodificar(p: &[i64]) -> Result<(Comando, usize), ErrorGpu> {
        let op = *p.first().ok_or(ErrorGpu::ComandoIncompleto {
            operacion: "comando",
            faltan: 1,
        })?;
        let (n, nombre) =
            Self::palabras_necesarias(op).ok_or(ErrorGpu::OperacionDesconocida { codigo: op })?;
        if p.len() < n {
            return Err(ErrorGpu::ComandoIncompleto {
                operacion: nombre,
                faltan: n - p.len(),
            });
        }
        let color = |w: i64| (w & 0xFF_FFFF) as u32;
        let vert = |i: usize| Vertice {
            x: p[i],
            y: p[i + 1],
            color: color(p[i + 2]),
        };
        let c = match op {
            OP_NADA => Comando::Nada,
            OP_COLOR => Comando::Color(color(p[1])),
            OP_LIMPIAR => Comando::Limpiar,
            OP_RELLENAR => Comando::Rellenar {
                x: p[1],
                y: p[2],
                ancho: p[3],
                alto: p[4],
            },
            OP_LINEA => Comando::Linea {
                x0: p[1],
                y0: p[2],
                x1: p[3],
                y1: p[4],
            },
            OP_TRIANGULO => Comando::Triangulo {
                v: [(p[1], p[2]), (p[3], p[4]), (p[5], p[6])],
            },
            OP_TRIANGULO_COLOR => Comando::TrianguloColor {
                v: [vert(1), vert(4), vert(7)],
            },
            OP_PROFUNDIDAD => Comando::Profundidad(p[1].max(0) as u32),
            OP_PRUEBA_Z => Comando::PruebaZ(p[1] != 0),
            _ => Comando::FinLote,
        };
        Ok((c, n))
    }

    /// Decodifica una tira de palabras en comandos.
    pub fn decodificar_todos(mut p: &[i64]) -> Result<Vec<Comando>, ErrorGpu> {
        let mut v = Vec::new();
        while !p.is_empty() {
            let (c, n) = Self::decodificar(p)?;
            v.push(c);
            p = &p[n..];
        }
        Ok(v)
    }
}

fn numero(t: &str, linea: usize) -> Result<i64, ErrorGpu> {
    let (neg, t) = match t.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, t),
    };
    let v = if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("#")) {
        i64::from_str_radix(h, 16)
    } else {
        t.parse::<i64>()
    }
    .map_err(|_| ErrorGpu::Texto {
        linea,
        mensaje: format!("`{t}` no es un numero"),
    })?;
    Ok(if neg { -v } else { v })
}

/// Lee un guion de comandos en texto: una orden por linea, argumentos separados por espacios,
/// `;` comienza un comentario. Los numeros pueden ir en decimal, `0x...` o `#RRGGBB`.
pub fn leer_guion(texto: &str) -> Result<Vec<Comando>, ErrorGpu> {
    let mut v = Vec::new();
    for (i, linea) in texto.lines().enumerate() {
        let n = i + 1;
        let linea = linea.split(';').next().unwrap_or("").trim();
        if linea.is_empty() {
            continue;
        }
        let mut it = linea.split_whitespace();
        let nombre = it.next().unwrap_or("");
        let op = [
            OP_NADA,
            OP_COLOR,
            OP_LIMPIAR,
            OP_RELLENAR,
            OP_LINEA,
            OP_TRIANGULO,
            OP_TRIANGULO_COLOR,
            OP_PROFUNDIDAD,
            OP_PRUEBA_Z,
            OP_FIN_LOTE,
        ]
        .into_iter()
        .find(|o| Comando::palabras_necesarias(*o).map(|(_, nom)| nom) == Some(nombre))
        .ok_or_else(|| ErrorGpu::Texto {
            linea: n,
            mensaje: format!("la orden `{nombre}` no existe"),
        })?;
        let mut palabras = vec![op];
        for t in it {
            palabras.push(numero(t, n)?);
        }
        let esperadas = Comando::palabras_necesarias(op).map_or(1, |(k, _)| k);
        if palabras.len() != esperadas {
            return Err(ErrorGpu::Texto {
                linea: n,
                mensaje: format!(
                    "`{nombre}` lleva {} argumentos y se dieron {}",
                    esperadas - 1,
                    palabras.len() - 1
                ),
            });
        }
        v.push(Comando::decodificar(&palabras)?.0);
    }
    Ok(v)
}
