//! Lenguaje de los hilos de la unidad grafica: un ensamblador pequeno (RF-GPU-013).
//!
//! Un *kernel* es el programa que corren todos los hilos a la vez, cada uno con sus registros. No
//! es la ISA de la CPU de la maquina: la unidad grafica tiene su propio repertorio, mas corto.
//!
//! ```text
//! ; suma de vectores: c[i] = a[i] + b[i], con a en 0, b en 8 y c en 16
//! .global 24
//! .dato 0  1 2 3 4 5 6 7 8
//! .dato 8  10 20 30 40 50 60 70 80
//! .lanzar 2 4              ; 2 grupos de 4 hilos
//!         gid   r0
//!         ld    r1, [r0+0]
//!         ld    r2, [r0+8]
//!         add   r3, r1, r2
//!         st    [r0+16], r3
//!         exit
//! ```
//!
//! Registros `r0` a `r15`. El segundo operando de las operaciones aritmeticas es un registro o una
//! constante. Los comentarios empiezan con `;`.

use std::collections::BTreeMap;

use crate::error::ErrorGpu;

pub const REGISTROS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fuente {
    Reg(usize),
    Imm(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpAlu {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Xor,
    Shl,
    Shr,
    /// 1 si a < b, 0 si no.
    Slt,
    /// 1 si a == b, 0 si no.
    Seq,
}

impl OpAlu {
    pub fn nombre(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Sub => "sub",
            Self::Mul => "mul",
            Self::Div => "div",
            Self::Rem => "rem",
            Self::And => "and",
            Self::Or => "or",
            Self::Xor => "xor",
            Self::Shl => "shl",
            Self::Shr => "shr",
            Self::Slt => "slt",
            Self::Seq => "seq",
        }
    }

    fn desde(n: &str) -> Option<Self> {
        [
            Self::Add,
            Self::Sub,
            Self::Mul,
            Self::Div,
            Self::Rem,
            Self::And,
            Self::Or,
            Self::Xor,
            Self::Shl,
            Self::Shr,
            Self::Slt,
            Self::Seq,
        ]
        .into_iter()
        .find(|o| o.nombre() == n)
    }
}

/// Que registro especial de hilo se lee.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Especial {
    /// Indice del hilo dentro de su grupo.
    Tid,
    /// Indice global del hilo.
    Gid,
    /// Indice del grupo.
    Bid,
    /// Hilos por grupo.
    Bdim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instr {
    Li(usize, i64),
    Mov(usize, usize),
    Alu(OpAlu, usize, usize, Fuente),
    /// `ld rd, [ra+off]`: memoria global (o compartida si el booleano es verdadero).
    Ld(usize, usize, i64, bool),
    St(usize, i64, usize, bool),
    Esp(usize, Especial),
    Bra(usize),
    Bz(usize, usize),
    Bnz(usize, usize),
    Bar,
    Exit,
}

impl Instr {
    pub fn es_salto(&self) -> bool {
        matches!(self, Self::Bra(_) | Self::Bz(..) | Self::Bnz(..))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kernel {
    pub instrucciones: Vec<Instr>,
    /// Texto de cada instruccion, para las trazas.
    pub textos: Vec<String>,
    /// Linea del fuente de cada instruccion.
    pub lineas: Vec<usize>,
    pub etiquetas: BTreeMap<String, usize>,
    /// `.global N`: palabras de memoria global que pide el kernel.
    pub global: Option<usize>,
    /// `.compartida N`
    pub compartida: Option<usize>,
    /// `.dato DIR v...`: valores iniciales de la memoria global.
    pub datos: Vec<(usize, Vec<i64>)>,
    /// `.lanzar GRUPOS HILOS`
    pub lanzar: Option<(usize, usize)>,
}

fn err(linea: usize, m: impl Into<String>) -> ErrorGpu {
    ErrorGpu::Texto {
        linea,
        mensaje: m.into(),
    }
}

fn registro(t: &str, linea: usize) -> Result<usize, ErrorGpu> {
    let t = t.trim().to_lowercase();
    t.strip_prefix('r')
        .and_then(|n| n.parse::<usize>().ok())
        .filter(|n| *n < REGISTROS)
        .ok_or_else(|| {
            err(
                linea,
                format!("`{t}` no es un registro (r0 a r{})", REGISTROS - 1),
            )
        })
}

fn entero(t: &str, linea: usize) -> Result<i64, ErrorGpu> {
    let t = t.trim();
    let (neg, s) = match t.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let v = if let Some(h) = s.strip_prefix("0x") {
        i64::from_str_radix(h, 16)
    } else {
        s.parse::<i64>()
    }
    .map_err(|_| err(linea, format!("`{t}` no es un numero")))?;
    Ok(if neg { -v } else { v })
}

fn fuente(t: &str, linea: usize) -> Result<Fuente, ErrorGpu> {
    if t.trim().to_lowercase().starts_with('r') {
        Ok(Fuente::Reg(registro(t, linea)?))
    } else {
        Ok(Fuente::Imm(entero(t, linea)?))
    }
}

/// `[ra+off]`, `[ra-off]` o `[ra]`.
fn direccion(t: &str, linea: usize) -> Result<(usize, i64), ErrorGpu> {
    let t = t.trim();
    let dentro = t
        .strip_prefix('[')
        .and_then(|r| r.strip_suffix(']'))
        .ok_or_else(|| {
            err(
                linea,
                format!("`{t}` no es una direccion `[registro+desplazamiento]`"),
            )
        })?;
    match dentro.find(['+', '-']) {
        None => Ok((registro(dentro, linea)?, 0)),
        Some(i) => Ok((registro(&dentro[..i], linea)?, entero(&dentro[i..], linea)?)),
    }
}

/// Lee un kernel de texto. Los saltos a etiquetas que no existen y los operandos mal formados
/// senalan la linea del error.
pub fn ensamblar(texto: &str) -> Result<Kernel, ErrorGpu> {
    let mut k = Kernel {
        instrucciones: Vec::new(),
        textos: Vec::new(),
        lineas: Vec::new(),
        etiquetas: BTreeMap::new(),
        global: None,
        compartida: None,
        datos: Vec::new(),
        lanzar: None,
    };
    // Los saltos se resuelven al final, cuando ya se conocen todas las etiquetas.
    let mut saltos: Vec<(usize, String, usize)> = Vec::new();
    for (i, cruda) in texto.lines().enumerate() {
        let n = i + 1;
        let mut linea = cruda.split(';').next().unwrap_or("").trim();
        while let Some((et, resto)) = linea.split_once(':') {
            let et = et.trim();
            if et.is_empty() || et.contains(char::is_whitespace) {
                break;
            }
            if k.etiquetas
                .insert(et.to_string(), k.instrucciones.len())
                .is_some()
            {
                return Err(err(n, format!("la etiqueta `{et}` ya existe")));
            }
            linea = resto.trim();
        }
        if linea.is_empty() {
            continue;
        }
        let (op, resto) = match linea.split_once(char::is_whitespace) {
            Some((o, r)) => (o.to_lowercase(), r.trim()),
            None => (linea.to_lowercase(), ""),
        };
        let args: Vec<&str> = if resto.is_empty() {
            Vec::new()
        } else {
            resto.split(',').map(str::trim).collect()
        };
        let pide = |cuantos: usize| -> Result<(), ErrorGpu> {
            if args.len() == cuantos {
                Ok(())
            } else {
                Err(err(
                    n,
                    format!(
                        "`{op}` lleva {cuantos} operandos y se dieron {}",
                        args.len()
                    ),
                ))
            }
        };
        if let Some(d) = op.strip_prefix('.') {
            let nums = resto
                .split_whitespace()
                .map(|t| entero(t, n))
                .collect::<Result<Vec<_>, _>>()?;
            match (d, nums.as_slice()) {
                ("global", [v]) if *v > 0 => k.global = Some(*v as usize),
                ("compartida", [v]) if *v > 0 => k.compartida = Some(*v as usize),
                ("lanzar", [g, h]) if *g > 0 && *h > 0 => {
                    k.lanzar = Some((*g as usize, *h as usize))
                }
                ("dato", [a, rest @ ..]) if *a >= 0 && !rest.is_empty() => {
                    k.datos.push((*a as usize, rest.to_vec()))
                }
                ("global" | "compartida" | "lanzar" | "dato", _) => {
                    return Err(err(
                        n,
                        format!("la directiva `.{d}` no tiene los argumentos correctos"),
                    ))
                }
                _ => return Err(err(n, format!("la directiva `.{d}` no existe"))),
            }
            continue;
        }
        let ins = match op.as_str() {
            "li" => {
                pide(2)?;
                Instr::Li(registro(args[0], n)?, entero(args[1], n)?)
            }
            "mov" => {
                pide(2)?;
                Instr::Mov(registro(args[0], n)?, registro(args[1], n)?)
            }
            "tid" | "gid" | "bid" | "bdim" => {
                pide(1)?;
                let e = match op.as_str() {
                    "tid" => Especial::Tid,
                    "gid" => Especial::Gid,
                    "bid" => Especial::Bid,
                    _ => Especial::Bdim,
                };
                Instr::Esp(registro(args[0], n)?, e)
            }
            "ld" | "lds" => {
                pide(2)?;
                let (r, o) = direccion(args[1], n)?;
                Instr::Ld(registro(args[0], n)?, r, o, op == "lds")
            }
            "st" | "sts" => {
                pide(2)?;
                let (r, o) = direccion(args[0], n)?;
                Instr::St(r, o, registro(args[1], n)?, op == "sts")
            }
            "bra" => {
                pide(1)?;
                saltos.push((k.instrucciones.len(), args[0].to_string(), n));
                Instr::Bra(0)
            }
            "bz" | "bnz" => {
                pide(2)?;
                saltos.push((k.instrucciones.len(), args[1].to_string(), n));
                let r = registro(args[0], n)?;
                if op == "bz" {
                    Instr::Bz(r, 0)
                } else {
                    Instr::Bnz(r, 0)
                }
            }
            "bar" => {
                pide(0)?;
                Instr::Bar
            }
            "exit" => {
                pide(0)?;
                Instr::Exit
            }
            otro => match OpAlu::desde(otro) {
                Some(a) => {
                    pide(3)?;
                    Instr::Alu(
                        a,
                        registro(args[0], n)?,
                        registro(args[1], n)?,
                        fuente(args[2], n)?,
                    )
                }
                None => return Err(err(n, format!("la instruccion `{otro}` no existe"))),
            },
        };
        k.instrucciones.push(ins);
        k.textos.push(linea.to_string());
        k.lineas.push(n);
    }
    for (i, et, n) in saltos {
        let destino = *k
            .etiquetas
            .get(&et)
            .ok_or_else(|| err(n, format!("la etiqueta `{et}` no existe")))?;
        k.instrucciones[i] = match k.instrucciones[i] {
            Instr::Bra(_) => Instr::Bra(destino),
            Instr::Bz(r, _) => Instr::Bz(r, destino),
            Instr::Bnz(r, _) => Instr::Bnz(r, destino),
            otra => otra,
        };
    }
    if k.instrucciones.is_empty() {
        return Err(err(1, "el kernel no tiene instrucciones"));
    }
    Ok(k)
}
