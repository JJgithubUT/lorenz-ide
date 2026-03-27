//! Historico de metricas por version de la arquitectura (RF-CIC-030).
//!
//! Cada medicion se anade al final de un archivo TOML con una tabla `[[medicion]]`. La version es
//! la huella de la descripcion, asi que dos mediciones con la misma version son comparables y una
//! version nueva aparece sola en cuanto cambia la arquitectura.

use std::io::Write;

use crate::ciclos::Corrida;
use crate::ordenes::{SALIDA_ERROR_USO, SALIDA_OK};

fn cadena(t: &str) -> String {
    format!("\"{}\"", t.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Anade la medicion de una corrida al archivo, que se crea si no existe.
pub fn anadir(
    ruta: &str,
    c: &Corrida,
    programa: &str,
    etiqueta: Option<&str>,
) -> Result<(), String> {
    let m = c.sim.metricas();
    let previas = std::fs::read_to_string(ruta)
        .map(|t| t.matches("[[medicion]]").count())
        .unwrap_or(0);
    let mut s = String::new();
    s.push_str("[[medicion]]\n");
    s.push_str(&format!("numero = {}\n", previas + 1));
    s.push_str(&format!("version = {}\n", cadena(&c.version)));
    s.push_str(&format!("maquina = {}\n", cadena(&c.nombre_maquina)));
    s.push_str(&format!("programa = {}\n", cadena(programa)));
    if let Some(e) = etiqueta {
        s.push_str(&format!("etiqueta = {}\n", cadena(e)));
    }
    s.push_str(&format!("ciclos = {}\n", m.ciclos));
    s.push_str(&format!("instrucciones = {}\n", m.instrucciones));
    s.push_str(&format!("esperas_datos = {}\n", m.esperas_datos));
    s.push_str(&format!(
        "esperas_estructurales = {}\n",
        m.esperas_estructurales
    ));
    s.push_str(&format!("esperas_control = {}\n", m.esperas_control));
    s.push_str(&format!("saltos = {}\n", m.saltos));
    s.push_str(&format!("saltos_acertados = {}\n", m.saltos_acertados));
    s.push_str(&format!("bytes_de_codigo = {}\n\n", c.bytes_de_codigo));
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(ruta)
        .map_err(|e| e.to_string())?;
    f.write_all(s.as_bytes()).map_err(|e| e.to_string())
}

/// Una fila leida del historico.
#[derive(Debug, Clone, PartialEq)]
pub struct Medicion {
    pub numero: i64,
    pub version: String,
    pub maquina: String,
    pub programa: String,
    pub etiqueta: Option<String>,
    pub ciclos: i64,
    pub instrucciones: i64,
    pub bytes_de_codigo: i64,
}

pub fn leer(ruta: &str) -> Result<Vec<Medicion>, String> {
    let texto = std::fs::read_to_string(ruta).map_err(|e| format!("no se pudo leer: {e}"))?;
    let t: toml::Table = toml::from_str(&texto).map_err(|e| e.to_string())?;
    let Some(filas) = t.get("medicion").and_then(|v| v.as_array()) else {
        return Ok(Vec::new());
    };
    let mut v = Vec::new();
    for f in filas {
        let texto = |k: &str| f.get(k).and_then(|x| x.as_str()).map(str::to_string);
        let numero = |k: &str| f.get(k).and_then(|x| x.as_integer()).unwrap_or(0);
        v.push(Medicion {
            numero: numero("numero"),
            version: texto("version").unwrap_or_default(),
            maquina: texto("maquina").unwrap_or_default(),
            programa: texto("programa").unwrap_or_default(),
            etiqueta: texto("etiqueta"),
            ciclos: numero("ciclos"),
            instrucciones: numero("instrucciones"),
            bytes_de_codigo: numero("bytes_de_codigo"),
        });
    }
    Ok(v)
}

/// El historico como tabla, con el cambio de cada medicion respecto de la anterior del mismo
/// programa.
pub fn tabla(mediciones: &[Medicion]) -> String {
    let mut s = String::from("# n version maquina programa ciclos cambio bytes etiqueta\n");
    for (i, m) in mediciones.iter().enumerate() {
        let anterior = mediciones[..i]
            .iter()
            .rev()
            .find(|x| x.programa == m.programa);
        let cambio = match anterior {
            Some(a) if a.ciclos > 0 => {
                format!(
                    "{:+.1}%",
                    (m.ciclos - a.ciclos) as f64 / a.ciclos as f64 * 100.0
                )
            }
            _ => "-".into(),
        };
        s.push_str(&format!(
            "{} {} {} {} {} {} {} {}\n",
            m.numero,
            m.version,
            m.maquina,
            m.programa,
            m.ciclos,
            cambio,
            m.bytes_de_codigo,
            m.etiqueta.as_deref().unwrap_or("-")
        ));
    }
    s
}

pub fn historial_cli(ruta: &str, salida: &mut dyn Write, errores: &mut dyn Write) -> i32 {
    match leer(ruta) {
        Ok(m) => {
            let _ = salida.write_all(tabla(&m).as_bytes());
            SALIDA_OK
        }
        Err(e) => {
            let _ = errores.write_all(format!("{ruta}: {e}\n").as_bytes());
            SALIDA_ERROR_USO
        }
    }
}
