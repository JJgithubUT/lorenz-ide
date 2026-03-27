//! Sugerencias de instrucciones nuevas a partir de los programas (RF-CIC-034).
//!
//! Se busca en lo que los programas ejecutan las secuencias de instrucciones que se repiten, y se
//! estima cuanto se ahorraria si cada una fuera una sola instruccion. Es una estimacion para
//! orientar el diseno del repertorio: da por hecho que la instruccion nueva ocuparia lo que la mas
//! larga de la secuencia y tardaria lo que la mas lenta, y no mira los operandos.

use std::collections::{BTreeMap, BTreeSet};

use lorenz_emu::Paso;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opciones {
    /// Longitud minima y maxima de las secuencias que se buscan.
    pub minima: usize,
    pub maxima: usize,
    /// Cuantas sugerencias se devuelven, las de mayor ahorro primero.
    pub cuantas: usize,
}

impl Default for Opciones {
    fn default() -> Self {
        Self {
            minima: 2,
            maxima: 4,
            cuantas: 5,
        }
    }
}

/// Una secuencia que podria ser una instruccion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sugerencia {
    pub patron: Vec<String>,
    /// Veces que aparece en el codigo de los programas.
    pub apariciones: u64,
    /// Veces que se ejecuta en total.
    pub ejecuciones: u64,
    /// Palabras de codigo que se ahorrarian, sumando todas las apariciones.
    pub palabras_ahorradas: u64,
    /// Ciclos que se ahorrarian en la ejecucion que se midio.
    pub ciclos_ahorrados: u64,
}

impl Sugerencia {
    /// El nombre con que se propone: `LDI+ADD`.
    pub fn nombre(&self) -> String {
        self.patron.join("+")
    }
}

#[derive(Debug, Clone)]
struct Instruccion {
    mnemonico: String,
    longitud: u64,
    ciclos: u64,
    ejecuciones: u64,
}

/// Analiza las ejecuciones de uno o varios programas (cada una en orden) y propone secuencias.
pub fn sugerir(ejecuciones: &[Vec<Paso>], op: &Opciones) -> Vec<Sugerencia> {
    let mut grupos: BTreeMap<Vec<String>, Sugerencia> = BTreeMap::new();
    for traza in ejecuciones {
        analizar_programa(traza, op, &mut grupos);
    }
    let mut v: Vec<Sugerencia> = grupos
        .into_values()
        .filter(|s| s.ciclos_ahorrados > 0 || s.palabras_ahorradas > 0)
        .collect();
    v.sort_by(|a, b| {
        b.ciclos_ahorrados
            .cmp(&a.ciclos_ahorrados)
            .then(b.palabras_ahorradas.cmp(&a.palabras_ahorradas))
            .then(b.patron.len().cmp(&a.patron.len()))
            .then(a.patron.cmp(&b.patron))
    });
    v.truncate(op.cuantas);
    v
}

fn analizar_programa(
    traza: &[Paso],
    op: &Opciones,
    grupos: &mut BTreeMap<Vec<String>, Sugerencia>,
) {
    // El codigo que se ejecuto, por direccion, y por donde se puede entrar o salir de una secuencia.
    let mut codigo: BTreeMap<u64, Instruccion> = BTreeMap::new();
    let mut destinos: BTreeSet<u64> = BTreeSet::new();
    let mut saltan: BTreeSet<u64> = BTreeSet::new();
    for (k, p) in traza.iter().enumerate() {
        let i = codigo.entry(p.direccion).or_insert_with(|| Instruccion {
            mnemonico: p.mnemonico.clone(),
            longitud: p.longitud as u64,
            ciclos: p.ciclos,
            ejecuciones: 0,
        });
        i.ejecuciones += 1;
        if let Some(sig) = traza.get(k + 1) {
            if sig.direccion != p.direccion + p.longitud as u64 {
                saltan.insert(p.direccion);
                destinos.insert(sig.direccion);
            }
        }
    }
    let direcciones: Vec<u64> = codigo.keys().copied().collect();
    for &a in &direcciones {
        let mut cadena: Vec<u64> = Vec::new();
        let mut actual = a;
        while cadena.len() < op.maxima {
            let Some(i) = codigo.get(&actual) else { break };
            // Una instruccion que salta, o a la que se llega saltando, no puede ir en el medio.
            if !cadena.is_empty() && destinos.contains(&actual) {
                break;
            }
            cadena.push(actual);
            if cadena.len() >= op.minima {
                anotar(&cadena, &codigo, grupos);
            }
            if saltan.contains(&actual) {
                break;
            }
            actual += i.longitud;
        }
    }
}

fn anotar(
    cadena: &[u64],
    codigo: &BTreeMap<u64, Instruccion>,
    grupos: &mut BTreeMap<Vec<String>, Sugerencia>,
) {
    let items: Vec<&Instruccion> = cadena.iter().map(|d| &codigo[d]).collect();
    let patron: Vec<String> = items.iter().map(|i| i.mnemonico.clone()).collect();
    let palabras: u64 = items.iter().map(|i| i.longitud).sum::<u64>()
        - items.iter().map(|i| i.longitud).max().unwrap_or(0);
    let ciclos: u64 = items.iter().map(|i| i.ciclos).sum::<u64>()
        - items.iter().map(|i| i.ciclos).max().unwrap_or(0);
    let veces = items[0].ejecuciones;
    let s = grupos.entry(patron.clone()).or_insert(Sugerencia {
        patron,
        apariciones: 0,
        ejecuciones: 0,
        palabras_ahorradas: 0,
        ciclos_ahorrados: 0,
    });
    s.apariciones += 1;
    s.ejecuciones += veces;
    s.palabras_ahorradas += palabras;
    s.ciclos_ahorrados += ciclos * veces;
}

/// Ejecuta un programa en el emulador y devuelve cada instruccion que ejecuto.
pub fn trazar(
    emu: &mut lorenz_emu::Emulador,
    limite: u64,
) -> Result<Vec<Paso>, lorenz_emu::ErrorEjecucion> {
    let mut v = Vec::new();
    while (v.len() as u64) < limite {
        match emu.paso()? {
            Some(p) => v.push(p),
            None => break,
        }
    }
    Ok(v)
}
