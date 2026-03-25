//! Exploracion de ordenes de eventos (RF-DEP-043).
//!
//! Dados unos eventos externos (una interrupcion, una tecla, un valor en un puerto) y una ventana
//! de instantes, se ejecuta el programa una vez por cada manera de intercalarlos con las
//! instrucciones y se agrupan los casos por su resultado. Si el resultado depende del orden, hay
//! una condicion de carrera, y cada grupo trae un orden de ejemplo para reproducirla.
//!
//! La exploracion parte del estado actual del depurador y no lo toca: trabaja en una copia.

use std::collections::{BTreeMap, BTreeSet};

use lorenz_emu::Emulador;

use crate::depurador::{Depurador, Parada};
use crate::error::ErrorDepurador;
use crate::traza::Intervencion;

/// Mas eventos que estos hacen que las permutaciones se disparen.
pub const MAX_EVENTOS: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opciones {
    /// Los eventos pueden caer entre el instante 0 y este, ambos incluidos.
    pub ventana: u64,
    /// Instrucciones que se dejan correr a cada caso.
    pub limite: u64,
    /// Maximo de casos; si hay mas, se elige una muestra repetible.
    pub max_casos: u64,
    /// Celdas de memoria que forman parte del resultado, ademas de los registros.
    pub celdas: Vec<u64>,
}

impl Default for Opciones {
    fn default() -> Self {
        Opciones {
            ventana: 20,
            limite: 100_000,
            max_casos: 2_000,
            celdas: Vec::new(),
        }
    }
}

/// Los casos que terminaron igual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grupo {
    pub resultado: String,
    pub casos: u64,
    /// Un orden que lo produce: `(instante, evento)` en el orden en que se aplican.
    pub ejemplo: Vec<(u64, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exploracion {
    pub casos: u64,
    /// `true` si se probaron todas las maneras; `false` si se tomo una muestra.
    pub completa: bool,
    /// Del resultado mas frecuente al menos frecuente.
    pub grupos: Vec<Grupo>,
}

impl Exploracion {
    /// El resultado depende del orden de los eventos.
    pub fn hay_carrera(&self) -> bool {
        self.grupos.len() > 1
    }
}

fn siguiente(estado: &mut u64) -> u64 {
    let mut x = *estado;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *estado = x;
    x
}

fn permutaciones(n: usize) -> Vec<Vec<usize>> {
    fn rec(resto: &mut Vec<usize>, actual: &mut Vec<usize>, fuera: &mut Vec<Vec<usize>>) {
        if resto.is_empty() {
            fuera.push(actual.clone());
            return;
        }
        for i in 0..resto.len() {
            let x = resto.remove(i);
            actual.push(x);
            rec(resto, actual, fuera);
            actual.pop();
            resto.insert(i, x);
        }
    }
    let mut fuera = Vec::new();
    rec(&mut (0..n).collect(), &mut Vec::new(), &mut fuera);
    fuera
}

type Calendario = Vec<(u64, usize)>;

/// Anade los calendarios que salen de unos instantes: con instantes iguales manda el orden de la
/// permutacion, y los que quedan igual no se repiten.
fn anadir(
    t: &[u64],
    perms: &[Vec<usize>],
    vistos: &mut BTreeSet<Calendario>,
    salida: &mut Vec<Calendario>,
) {
    let k = t.len();
    for p in perms {
        let mut orden: Vec<(u64, usize, usize)> = (0..k)
            .map(|i| (t[i], p.iter().position(|&x| x == i).unwrap_or(0), i))
            .collect();
        orden.sort();
        let c: Calendario = orden.iter().map(|(t, _, i)| (*t, *i)).collect();
        if vistos.insert(c.clone()) {
            salida.push(c);
        }
    }
}

/// Los ordenes distintos que hay que probar, o una muestra repetible si son demasiados.
fn calendarios(k: usize, op: &Opciones) -> (Vec<Calendario>, bool) {
    let v = op.ventana + 1;
    let total = (v as u128).checked_pow(k as u32).unwrap_or(u128::MAX);
    let perms = permutaciones(k);
    let mut vistos = BTreeSet::new();
    let mut salida = Vec::new();
    let completa = total <= u128::from(op.max_casos);
    if completa {
        let mut t = vec![0u64; k];
        loop {
            anadir(&t, &perms, &mut vistos, &mut salida);
            let mut i = 0;
            while i < k {
                t[i] += 1;
                if t[i] < v {
                    break;
                }
                t[i] = 0;
                i += 1;
            }
            if i == k {
                break;
            }
        }
    } else {
        let mut x = 0x9E37_79B9_7F4A_7C15u64;
        for _ in 0..op.max_casos {
            let t: Vec<u64> = (0..k).map(|_| siguiente(&mut x) % v).collect();
            anadir(&t, &perms, &mut vistos, &mut salida);
        }
    }
    salida.truncate(op.max_casos as usize);
    (salida, completa)
}

/// Lo que se compara entre casos: registros, banderas, pc, celdas elegidas y como termino.
fn resultado(d: &Depurador, op: &Opciones, fin: &str) -> String {
    let e = d.emulador().estado();
    let mut s = String::new();
    for r in &d.maquina().registros {
        if let Ok(v) = e.registro(&r.nombre) {
            s.push_str(&format!("{}={v:#x} ", r.nombre));
        }
    }
    for b in &d.maquina().banderas {
        if let Ok(v) = e.bandera(&b.nombre) {
            s.push_str(&format!("{}={} ", b.nombre, u8::from(v)));
        }
    }
    for a in &op.celdas {
        match d.emulador().memoria().leer_privilegiado(*a) {
            Ok(v) => s.push_str(&format!("[{a:#x}]={v:#x} ")),
            Err(_) => s.push_str(&format!("[{a:#x}]=? ")),
        }
    }
    s.push_str(fin);
    s
}

/// Explora los ordenes en que pueden llegar `eventos` (RF-DEP-043).
pub fn explorar(
    d: &Depurador,
    eventos: &[Intervencion],
    op: &Opciones,
) -> Result<Exploracion, ErrorDepurador> {
    if eventos.is_empty() {
        return Err(ErrorDepurador::FaltaArgumento {
            orden: "explorar",
            falta: "al menos un evento",
        });
    }
    if eventos.len() > MAX_EVENTOS {
        return Err(ErrorDepurador::DemasiadosEventos {
            maximo: MAX_EVENTOS,
        });
    }
    let inicio = d.emulador().instantanea();
    let mut emu = Emulador::nuevo(d.maquina().clone())?;
    emu.restaurar(&inicio)?;
    let mut copia = Depurador::nuevo(emu);
    let (calendarios, completa) = calendarios(eventos.len(), op);
    let mut grupos: BTreeMap<String, Grupo> = BTreeMap::new();
    for cal in &calendarios {
        copia.emulador_mut().restaurar(&inicio)?;
        let mut siguiente_evento = 0usize;
        let mut hechas = 0u64;
        let fin = loop {
            while siguiente_evento < cal.len() && cal[siguiente_evento].0 <= hechas {
                let (_, e) = cal[siguiente_evento];
                // Un evento que la maquina no admite (una linea que no existe) no cuenta.
                let _ = copia.aplicar_a_mano(&eventos[e]);
                siguiente_evento += 1;
            }
            if hechas >= op.limite {
                break "limite".to_string();
            }
            match copia.paso(1) {
                Parada::Detenida => break "detenida".to_string(),
                Parada::Fallo { error, .. } => break format!("fallo: {error}"),
                _ => hechas += 1,
            }
        };
        let r = resultado(&copia, op, &fin);
        grupos
            .entry(r.clone())
            .and_modify(|g| g.casos += 1)
            .or_insert(Grupo {
                resultado: r,
                casos: 1,
                ejemplo: cal.clone(),
            });
    }
    let mut grupos: Vec<Grupo> = grupos.into_values().collect();
    grupos.sort_by(|a, b| b.casos.cmp(&a.casos).then(a.resultado.cmp(&b.resultado)));
    Ok(Exploracion {
        casos: calendarios.len() as u64,
        completa,
        grupos,
    })
}
