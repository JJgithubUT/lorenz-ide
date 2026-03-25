//! El desensamblador: de palabras de memoria a texto que el ensamblador acepta.
//!
//! Se genera de la misma descripcion que el ensamblador (RF-ENS-021): las codificaciones dicen que
//! palabras son cada instruccion y las sintaxis dicen como escribirlas. Una palabra que no
//! corresponde a ninguna instruccion se muestra como dato en lugar de abortar (RF-ENS-023).

use std::collections::BTreeMap;

use lorenz_arch::bits::Codificacion;
use lorenz_arch::modelo::{Endianness, Maquina, TipoOperando};
use lorenz_arch::plantilla::Plantilla;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Linea {
    Instruccion {
        direccion: u64,
        palabras: Vec<u64>,
        texto: String,
        /// Posicion de la instruccion en la lista de la maquina.
        indice: usize,
    },
    /// Una palabra que no es el comienzo de ninguna instruccion valida.
    Dato { direccion: u64, palabra: u64 },
}

impl Linea {
    pub fn direccion(&self) -> u64 {
        match self {
            Linea::Instruccion { direccion, .. } | Linea::Dato { direccion, .. } => *direccion,
        }
    }

    /// Cuantas palabras ocupa.
    pub fn longitud(&self) -> usize {
        match self {
            Linea::Instruccion { palabras, .. } => palabras.len(),
            Linea::Dato { .. } => 1,
        }
    }
}

#[derive(Debug, Clone)]
struct Entrada {
    codificacion: Codificacion,
    plantilla: Plantilla,
}

/// Un desensamblador ya generado para una maquina concreta.
#[derive(Debug, Clone)]
pub struct Desensamblador<'m> {
    maquina: &'m Maquina,
    entradas: Vec<Entrada>,
}

fn mascara(bits: u32) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    }
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

impl<'m> Desensamblador<'m> {
    pub fn nuevo(maquina: &'m Maquina) -> Result<Self, String> {
        let mut entradas = Vec::new();
        for ins in &maquina.instrucciones {
            let plantilla = Plantilla::analizar(&ins.sintaxis)
                .map_err(|e| format!("la sintaxis de `{}` no es valida: {e}", ins.mnemonico))?;
            entradas.push(Entrada {
                codificacion: ins.codificacion.clone(),
                plantilla,
            });
        }
        Ok(Desensamblador { maquina, entradas })
    }

    /// Instruccion que empieza en `palabras[0]`, si alguna encaja.
    fn buscar(&self, palabras: &[u64]) -> Option<usize> {
        self.entradas.iter().position(|e| {
            let ws = e.codificacion.palabras();
            if palabras.len() < ws.len() {
                return false;
            }
            ws.iter().zip(palabras).all(|(w, p)| {
                p & w.mascara() == w.valor_fijo() && *p <= self.maquina.palabra_maxima()
            }) && self.registros_validos(e, palabras)
        })
    }

    fn valor_de_campo(&self, e: &Entrada, nombre: &str, palabras: &[u64]) -> u64 {
        let fragmentos = e.codificacion.fragmentos(nombre);
        let orden: Vec<_> = match self.maquina.endianness {
            Endianness::Little => fragmentos.iter().collect(),
            Endianness::Big => fragmentos.iter().rev().collect(),
        };
        let mut total = 0u64;
        let mut acumulados = 0u32;
        for f in orden {
            total |= ((palabras[f.palabra] >> f.desplazamiento) & mascara(f.bits)) << acumulados;
            acumulados += f.bits;
        }
        total
    }

    /// Un campo de registro cuyo indice no existe vuelve invalida la palabra.
    fn registros_validos(&self, e: &Entrada, palabras: &[u64]) -> bool {
        let indice = self
            .entradas
            .iter()
            .position(|x| std::ptr::eq(x, e))
            .unwrap();
        self.maquina.instrucciones[indice]
            .operandos
            .iter()
            .all(|o| match &o.tipo {
                TipoOperando::Registro { permitidos } => {
                    (self.valor_de_campo(e, &o.nombre, palabras) as usize) < permitidos.len()
                }
                _ => true,
            })
    }

    fn digitos_direccion(&self) -> usize {
        (self.maquina.bits_direccion as usize).div_ceil(4)
    }

    /// Desensambla `palabras`, que empiezan en la direccion `desde`.
    ///
    /// Con `simbolos` (direccion -> nombre) las direcciones de salto se escriben como etiquetas
    /// (RF-ENS-024).
    pub fn desensamblar(
        &self,
        desde: u64,
        palabras: &[u64],
        simbolos: Option<&BTreeMap<u64, String>>,
    ) -> Vec<Linea> {
        let m = self.maquina;
        let mut salida = Vec::new();
        let mut i = 0usize;
        while i < palabras.len() {
            let direccion = desde.wrapping_add(i as u64) & m.direccion_maxima();
            match self.buscar(&palabras[i..]) {
                Some(idx) => {
                    let e = &self.entradas[idx];
                    let n = e.codificacion.longitud_en_palabras();
                    let trozo = &palabras[i..i + n];
                    let ins = &m.instrucciones[idx];
                    let texto = e.plantilla.componer(|hueco| {
                        let operando = ins.operandos.iter().find(|o| o.nombre == hueco).unwrap();
                        let bits = e.codificacion.bits_totales(hueco);
                        let crudo = self.valor_de_campo(e, hueco, trozo);
                        self.texto_de_operando(&operando.tipo, crudo, bits, direccion, n, simbolos)
                    });
                    salida.push(Linea::Instruccion {
                        direccion,
                        palabras: trozo.to_vec(),
                        texto,
                        indice: idx,
                    });
                    i += n;
                }
                None => {
                    salida.push(Linea::Dato {
                        direccion,
                        palabra: palabras[i],
                    });
                    i += 1;
                }
            }
        }
        salida
    }

    fn texto_de_operando(
        &self,
        tipo: &TipoOperando,
        crudo: u64,
        bits: u32,
        direccion: u64,
        longitud: usize,
        simbolos: Option<&BTreeMap<u64, String>>,
    ) -> String {
        let m = self.maquina;
        let etiqueta = |a: u64| simbolos.and_then(|s| s.get(&a)).cloned();
        match tipo {
            TipoOperando::Registro { permitidos } => permitidos[crudo as usize].clone(),
            TipoOperando::Inmediato { signo, .. } => {
                if *signo {
                    extender_signo(crudo, bits).to_string()
                } else {
                    crudo.to_string()
                }
            }
            TipoOperando::Direccion => etiqueta(crudo).unwrap_or_else(|| {
                format!("{:#0ancho$x}", crudo, ancho = self.digitos_direccion() + 2)
            }),
            TipoOperando::Desplazamiento => {
                let destino = (direccion as i128 + longitud as i128 + extender_signo(crudo, bits))
                    .rem_euclid(m.direccion_maxima() as i128 + 1)
                    as u64;
                etiqueta(destino).unwrap_or_else(|| {
                    format!(
                        "{:#0ancho$x}",
                        destino,
                        ancho = self.digitos_direccion() + 2
                    )
                })
            }
            TipoOperando::Puerto => format!("{:#04x}", crudo),
        }
    }

    /// Escribe las lineas como un fuente que el ensamblador acepta y que reproduce las mismas
    /// palabras (RF-ENS-025). Cada linea lleva su direccion y sus bytes como comentario.
    pub fn a_texto(&self, lineas: &[Linea], simbolos: Option<&BTreeMap<u64, String>>) -> String {
        let mut salida = String::new();
        let mut siguiente: Option<u64> = None;
        for l in lineas {
            let d = l.direccion();
            if siguiente != Some(d) {
                salida.push_str(&format!(
                    ".org {:#0ancho$x}\n",
                    d,
                    ancho = self.digitos_direccion() + 2
                ));
            }
            if let Some(nombre) = simbolos.and_then(|s| s.get(&d)) {
                salida.push_str(&format!("{nombre}:\n"));
            }
            match l {
                Linea::Instruccion {
                    texto, palabras, ..
                } => {
                    let bytes: Vec<String> =
                        palabras.iter().map(|p| self.palabra_hex(*p)).collect();
                    salida.push_str(&format!("    {texto:<24}; {d:#06x}: {}\n", bytes.join(" ")));
                }
                Linea::Dato { palabra, .. } => {
                    salida.push_str(&format!(
                        "    {:<24}; {d:#06x}\n",
                        format!(".db {}", self.palabra_hex(*palabra))
                    ));
                }
            }
            siguiente = Some((d + l.longitud() as u64) & self.maquina.direccion_maxima());
        }
        salida
    }

    fn palabra_hex(&self, p: u64) -> String {
        format!(
            "{:#0ancho$x}",
            p,
            ancho = (self.maquina.bits_palabra as usize).div_ceil(4) + 2
        )
    }
}
