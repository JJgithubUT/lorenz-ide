//! Generacion de casos de conformidad: valores de operandos y estados de partida (RF-TST-016,
//! RF-TST-020).
//!
//! Todo es determinista: la misma arquitectura da siempre los mismos casos, de modo que un fallo
//! se puede repetir y comparar entre ejecuciones.

use lorenz_arch::modelo::{Instruccion, Maquina, Operando, Region, RolRegistro, TipoOperando};

/// Cuantas palabras de RAM libre hacen falta como minimo para poner a prueba una instruccion.
pub const RAM_MINIMA: u64 = 64;

/// Estado de partida de un caso.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patron {
    pub registros: Vec<(String, u64)>,
    pub banderas: Vec<(String, bool)>,
    pub ie: bool,
}

/// Una instruccion con valores concretos para sus operandos y un estado de partida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caso {
    /// Posicion de la instruccion en la maquina.
    pub indice: usize,
    pub mnemonico: String,
    /// Valor de cada operando; para un registro, su indice entre los permitidos.
    pub operandos: Vec<(String, i128)>,
    pub patron: Patron,
}

impl Caso {
    /// Texto corto del caso, para los informes.
    pub fn describir(&self, maquina: &Maquina) -> String {
        let ins = &maquina.instrucciones[self.indice];
        let mut partes: Vec<String> = Vec::new();
        for (nombre, valor) in &self.operandos {
            let o = ins.operandos.iter().find(|o| &o.nombre == nombre);
            match o.map(|o| &o.tipo) {
                Some(TipoOperando::Registro { permitidos }) => partes.push(format!(
                    "{nombre}={}",
                    permitidos
                        .get(*valor as usize)
                        .cloned()
                        .unwrap_or_else(|| valor.to_string())
                )),
                _ => partes.push(format!("{nombre}={valor}")),
            }
        }
        let regs: Vec<String> = self
            .patron
            .registros
            .iter()
            .map(|(n, v)| format!("{n}={v:#x}"))
            .collect();
        let banderas: String = self
            .patron
            .banderas
            .iter()
            .filter(|(_, v)| *v)
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>()
            .join("");
        format!(
            "{} [{}] con {} banderas={}",
            ins.mnemonico,
            partes.join(" "),
            regs.join(" "),
            if banderas.is_empty() { "-" } else { &banderas }
        )
    }
}

/// La region de RAM donde se colocan las instrucciones de prueba, si hay una adecuada.
pub fn region_de_pruebas(maquina: &Maquina) -> Option<&Region> {
    maquina.regiones.iter().find(|r| {
        r.acceso.lectura
            && r.acceso.escritura
            && r.dispositivo.is_none()
            && r.contenido.is_none()
            && r.tamano() >= RAM_MINIMA
    })
}

/// Si una direccion cae en memoria ordinaria de lectura y escritura.
pub fn es_ram(maquina: &Maquina, direccion: u64) -> bool {
    maquina
        .region_de(direccion)
        .is_some_and(|r| r.acceso.lectura && r.acceso.escritura && r.dispositivo.is_none())
}

fn bits_del_operando(ins: &Instruccion, nombre: &str) -> u32 {
    ins.codificacion
        .fragmentos(nombre)
        .iter()
        .map(|f| f.bits)
        .sum()
}

fn ordenar_sin_repetir(mut v: Vec<i128>) -> Vec<i128> {
    v.sort_unstable();
    v.dedup();
    v
}

/// Valores representativos de un operando: los extremos, el cero, el uno y algunos intermedios.
///
/// Siempre incluye el minimo y el maximo del rango (RF-TST-020). Los valores que no caben en el
/// campo codificado se descartan.
pub fn candidatos(maquina: &Maquina, ins: &Instruccion, operando: &Operando) -> Vec<i128> {
    let bits = bits_del_operando(ins, &operando.nombre).min(64);
    let sin_signo_max: i128 = (1i128 << bits) - 1;
    let ram = region_de_pruebas(maquina);
    let valores = match &operando.tipo {
        TipoOperando::Registro { permitidos } => (0..permitidos.len() as i128).collect(),
        TipoOperando::Inmediato { signo, rango } => {
            let (min, max) = (i128::from(rango.0), i128::from(rango.1));
            let medio = min + (max - min) / 2;
            let mut v = vec![min, min + 1, -1, 0, 1, medio, max - 1, max];
            if *signo {
                v.push(-2);
            }
            v.into_iter().filter(|x| (min..=max).contains(x)).collect()
        }
        TipoOperando::Direccion => {
            let max = i128::from(maquina.direccion_maxima()).min(sin_signo_max);
            let mut v = vec![0, 1, max];
            if let Some(r) = ram {
                v.extend([
                    i128::from(r.desde),
                    i128::from(r.desde) + 1,
                    i128::from(r.desde) + i128::from(r.tamano() / 2),
                    i128::from(r.hasta),
                ]);
            }
            v.into_iter().filter(|x| (0..=max).contains(x)).collect()
        }
        TipoOperando::Desplazamiento => {
            let min = -(1i128 << (bits.max(1) - 1));
            let max = (1i128 << (bits.max(1) - 1)) - 1;
            vec![min, -2, -1, 0, 1, 2, max]
                .into_iter()
                .filter(|x| (min..=max).contains(x))
                .collect()
        }
        TipoOperando::Puerto => {
            let max = sin_signo_max;
            vec![0, 1, max / 2, max]
                .into_iter()
                .filter(|x| (0..=max).contains(x))
                .collect()
        }
    };
    ordenar_sin_repetir(valores)
}

/// Los valores justo fuera del rango de un operando, que el ensamblador debe rechazar.
pub fn fuera_de_rango(operando: &Operando) -> Vec<i128> {
    match &operando.tipo {
        TipoOperando::Inmediato { rango, .. } => {
            vec![i128::from(rango.0) - 1, i128::from(rango.1) + 1]
        }
        TipoOperando::Registro { permitidos } => vec![-1, permitidos.len() as i128],
        _ => Vec::new(),
    }
}

/// Todas las combinaciones de valores de los operandos, o `tope` de ellas repartidas por todo el
/// espacio si son demasiadas. Siempre incluye la primera y la ultima.
pub fn combinaciones(
    maquina: &Maquina,
    ins: &Instruccion,
    tope: usize,
) -> Vec<Vec<(String, i128)>> {
    let listas: Vec<(String, Vec<i128>)> = ins
        .operandos
        .iter()
        .map(|o| (o.nombre.clone(), candidatos(maquina, ins, o)))
        .collect();
    if listas.iter().any(|(_, v)| v.is_empty()) {
        return Vec::new();
    }
    let total: u128 = listas
        .iter()
        .fold(1u128, |t, (_, v)| t.saturating_mul(v.len() as u128));
    let tope = tope.max(1) as u128;
    let quiero = total.min(tope);
    let mut indices: Vec<u128> = if total <= tope {
        (0..total).collect()
    } else {
        (0..quiero)
            .map(|k| k * (total - 1) / (quiero - 1).max(1))
            .collect()
    };
    indices.dedup();
    indices
        .into_iter()
        .map(|mut n| {
            let mut fila = Vec::with_capacity(listas.len());
            for (nombre, vals) in listas.iter().rev() {
                fila.push((nombre.clone(), vals[(n % vals.len() as u128) as usize]));
                n /= vals.len() as u128;
            }
            fila.reverse();
            fila
        })
        .collect()
}

/// Estados de partida variados: ceros, unos, maximos, patrones de bits alternos, valores que caen
/// en RAM y todas las combinaciones de banderas que caben en `cantidad` estados.
pub fn patrones(maquina: &Maquina, cantidad: usize) -> Vec<Patron> {
    let ram = region_de_pruebas(maquina);
    let base_ram = ram.map_or(0, |r| r.desde + 32);
    let cantidad = cantidad.max(1);
    (0..cantidad)
        .map(|k| {
            let registros = maquina
                .registros
                .iter()
                .filter(|r| {
                    !matches!(
                        r.rol,
                        RolRegistro::ContadorPrograma | RolRegistro::Banderas | RolRegistro::Fijo
                    )
                })
                .enumerate()
                .map(|(i, r)| {
                    let m = if r.bits >= 64 {
                        u64::MAX
                    } else {
                        (1u64 << r.bits) - 1
                    };
                    let tipos: [u64; 8] = [
                        0,
                        1,
                        m,
                        0x5555_5555_5555_5555,
                        0xAAAA_AAAA_AAAA_AAAA,
                        base_ram + k as u64,
                        m.wrapping_sub(1),
                        1u64 << (r.bits.max(1) - 1),
                    ];
                    let v = if r.rol == RolRegistro::PunteroPila {
                        base_ram + 16 + (k as u64 % 8)
                    } else {
                        tipos[(k + i * 3) % tipos.len()]
                    };
                    (r.nombre.clone(), v & m)
                })
                .collect();
            let banderas = maquina
                .banderas
                .iter()
                .enumerate()
                .map(|(i, b)| (b.nombre.clone(), (k >> (i % 16)) & 1 == 1))
                .collect();
            Patron {
                registros,
                banderas,
                ie: k % 2 == 1,
            }
        })
        .collect()
}

/// Casos de una instruccion: cada combinacion de operandos con `repeticiones` estados distintos.
pub fn casos_de(
    maquina: &Maquina,
    indice: usize,
    tope_combinaciones: usize,
    repeticiones: usize,
    patrones: &[Patron],
) -> Vec<Caso> {
    let ins = &maquina.instrucciones[indice];
    let mut casos = Vec::new();
    for (j, combo) in combinaciones(maquina, ins, tope_combinaciones)
        .into_iter()
        .enumerate()
    {
        for p in 0..repeticiones.max(1) {
            casos.push(Caso {
                indice,
                mnemonico: ins.mnemonico.clone(),
                operandos: combo.clone(),
                patron: patrones[(j * repeticiones.max(1) + p) % patrones.len()].clone(),
            });
        }
    }
    casos
}
