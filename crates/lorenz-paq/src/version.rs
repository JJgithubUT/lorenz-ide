//! Versiones `mayor.menor.parche` y los requisitos de las dependencias (RF-ECO-002, RF-ECO-005).

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub mayor: u64,
    pub menor: u64,
    pub parche: u64,
}

impl Version {
    pub fn nueva(mayor: u64, menor: u64, parche: u64) -> Self {
        Self {
            mayor,
            menor,
            parche,
        }
    }

    /// Lee `1`, `1.2` o `1.2.3`; lo que falta vale 0.
    pub fn parsear(t: &str) -> Result<Self, String> {
        let t = t.trim();
        let partes: Vec<&str> = t.split('.').collect();
        if t.is_empty() || partes.len() > 3 {
            return Err(format!(
                "`{t}` no es una version (se espera mayor.menor.parche)"
            ));
        }
        let n = |i: usize| -> Result<u64, String> {
            partes.get(i).map_or(Ok(0), |p| {
                p.parse::<u64>()
                    .map_err(|_| format!("`{t}` no es una version (se espera mayor.menor.parche)"))
            })
        };
        Ok(Self {
            mayor: n(0)?,
            menor: n(1)?,
            parche: n(2)?,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.mayor, self.menor, self.parche)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Cualquiera,
    Igual,
    MayorIgual,
    Mayor,
    MenorIgual,
    Menor,
    /// Compatible: misma version mayor (o menor, si la mayor es 0) y no anterior.
    Compatible,
}

/// Que versiones acepta una dependencia: `*`, `1.2.0`, `=1.2.0`, `>=1.0`, `>1`, `<=2`, `<2`, `^1.2`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Requisito {
    op: Op,
    v: Version,
}

impl Requisito {
    pub fn parsear(t: &str) -> Result<Self, String> {
        let t = t.trim();
        if t == "*" || t.is_empty() {
            return Ok(Self {
                op: Op::Cualquiera,
                v: Version::nueva(0, 0, 0),
            });
        }
        let (op, resto) = if let Some(r) = t.strip_prefix(">=") {
            (Op::MayorIgual, r)
        } else if let Some(r) = t.strip_prefix("<=") {
            (Op::MenorIgual, r)
        } else if let Some(r) = t.strip_prefix('>') {
            (Op::Mayor, r)
        } else if let Some(r) = t.strip_prefix('<') {
            (Op::Menor, r)
        } else if let Some(r) = t.strip_prefix('^') {
            (Op::Compatible, r)
        } else if let Some(r) = t.strip_prefix('=') {
            (Op::Igual, r)
        } else {
            (Op::Igual, t)
        };
        Ok(Self {
            op,
            v: Version::parsear(resto)?,
        })
    }

    pub fn cumple(&self, v: Version) -> bool {
        let r = self.v;
        match self.op {
            Op::Cualquiera => true,
            Op::Igual => v == r,
            Op::MayorIgual => v >= r,
            Op::Mayor => v > r,
            Op::MenorIgual => v <= r,
            Op::Menor => v < r,
            Op::Compatible => {
                v >= r
                    && if r.mayor > 0 {
                        v.mayor == r.mayor
                    } else {
                        v.mayor == 0 && v.menor == r.menor
                    }
            }
        }
    }
}

impl fmt::Display for Requisito {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let o = match self.op {
            Op::Cualquiera => return f.write_str("*"),
            Op::Igual => "=",
            Op::MayorIgual => ">=",
            Op::Mayor => ">",
            Op::MenorIgual => "<=",
            Op::Menor => "<",
            Op::Compatible => "^",
        };
        write!(f, "{o}{}", self.v)
    }
}
