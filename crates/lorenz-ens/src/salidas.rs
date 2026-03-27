//! Salidas legibles del ensamblador: mapa, listado y tabla de simbolos.
//!
//! * El mapa relaciona cada direccion con el archivo y la linea que la origino (RF-ENS-017).
//! * El listado reproduce el fuente con la direccion y los bytes de cada linea (RF-ENS-018).
//! * La tabla de simbolos es TOML, asi la leen herramientas externas sin un analizador propio
//!   (RF-ENS-029) y el desensamblador la usa para escribir etiquetas (RF-ENS-024).

use std::collections::BTreeMap;

use crate::ensamblador::{Fuente, Objeto};

/// Cuantos bytes caben en una fila del listado antes de continuar en la siguiente.
const PALABRAS_POR_FILA: usize = 6;

fn digitos_hex(maximo: u64) -> usize {
    format!("{maximo:x}").len().max(4)
}

/// Una linea por tramo: `direccion  longitud  archivo:linea`.
pub fn mapa(objeto: &Objeto) -> String {
    let ancho = digitos_hex(
        objeto
            .mapa
            .iter()
            .map(|e| e.direccion)
            .max()
            .unwrap_or_default(),
    );
    let mut salida = String::from("# direccion  palabras  origen\n");
    for e in &objeto.mapa {
        salida.push_str(&format!(
            "{:#0ancho$x}  {:>8}  {}:{}\n",
            e.direccion,
            e.longitud,
            e.archivo,
            e.linea,
            ancho = ancho + 2
        ));
    }
    salida
}

/// El fuente completo con la direccion y las palabras que genero cada linea.
///
/// `bits_palabra` fija cuantos digitos hexadecimales se escriben por palabra.
pub fn listado(objeto: &Objeto, fuentes: &[Fuente], bits_palabra: u32) -> String {
    let palabras: BTreeMap<u64, u64> = objeto.palabras().into_iter().collect();
    let por_linea: BTreeMap<(&str, usize), Vec<(u64, usize)>> =
        objeto.mapa.iter().fold(BTreeMap::new(), |mut acc, e| {
            acc.entry((e.archivo.as_str(), e.linea))
                .or_default()
                .push((e.direccion, e.longitud));
            acc
        });
    let ancho_dir = digitos_hex(palabras.keys().next_back().copied().unwrap_or_default());
    let ancho_palabra = (bits_palabra as usize).div_ceil(4);
    let ancho_bytes = PALABRAS_POR_FILA * (ancho_palabra + 1);
    let mut salida = String::new();
    for f in fuentes {
        if fuentes.len() > 1 {
            salida.push_str(&format!("; === {} ===\n", f.nombre));
        }
        for (i, texto) in f.texto.lines().enumerate() {
            let numero = i + 1;
            let tramos = por_linea.get(&(f.nombre.as_str(), numero));
            let mut filas: Vec<(u64, String)> = Vec::new();
            for &(dir, n) in tramos.into_iter().flatten() {
                let ws: Vec<String> = (0..n as u64)
                    .filter_map(|k| palabras.get(&(dir + k)))
                    .map(|p| format!("{p:0ancho_palabra$X}"))
                    .collect();
                for (k, trozo) in ws.chunks(PALABRAS_POR_FILA).enumerate() {
                    filas.push((dir + (k * PALABRAS_POR_FILA) as u64, trozo.join(" ")));
                }
            }
            if filas.is_empty() {
                salida.push_str(&format!(
                    "{:>5}  {:ancho_dir$}  {:<ancho_bytes$}  {}\n",
                    numero, "", "", texto
                ));
            } else {
                for (k, (dir, bytes)) in filas.iter().enumerate() {
                    let texto = if k == 0 { texto } else { "" };
                    let numero = if k == 0 {
                        numero.to_string()
                    } else {
                        String::new()
                    };
                    salida.push_str(&format!(
                        "{:>5}  {:0ancho_dir$X}  {:<ancho_bytes$}  {}\n",
                        numero, dir, bytes, texto
                    ));
                }
            }
        }
    }
    salida
}

/// Tabla de simbolos en TOML: etiquetas (direcciones) y constantes.
pub fn tabla_de_simbolos(objeto: &Objeto) -> String {
    let mut etiquetas = String::new();
    let mut constantes = String::new();
    for (nombre, s) in &objeto.simbolos {
        if s.etiqueta {
            etiquetas.push_str(&format!("{nombre} = {:#06x}\n", s.valor));
        } else {
            constantes.push_str(&format!("{nombre} = {}\n", s.valor));
        }
    }
    format!("[etiquetas]\n{etiquetas}\n[constantes]\n{constantes}")
}

/// Lee una tabla escrita por [`tabla_de_simbolos`] y devuelve `direccion -> etiqueta`.
///
/// Si dos etiquetas comparten direccion se queda con la primera por orden alfabetico.
pub fn leer_etiquetas(texto: &str) -> Result<BTreeMap<u64, String>, String> {
    let tabla: toml::Table = texto
        .parse()
        .map_err(|e| format!("tabla de simbolos invalida: {e}"))?;
    let mut salida = BTreeMap::new();
    if let Some(toml::Value::Table(etiquetas)) = tabla.get("etiquetas") {
        for (nombre, valor) in etiquetas {
            let toml::Value::Integer(v) = valor else {
                return Err(format!("la etiqueta `{nombre}` no tiene un valor entero"));
            };
            let d = u64::try_from(*v)
                .map_err(|_| format!("la etiqueta `{nombre}` tiene una direccion negativa"))?;
            salida.entry(d).or_insert_with(|| nombre.clone());
        }
    }
    Ok(salida)
}
