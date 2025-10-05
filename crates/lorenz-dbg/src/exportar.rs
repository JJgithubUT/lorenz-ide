//! Sacar la traza del depurador para analizarla fuera (RF-DEP-029).

use std::fmt::Write;

use crate::traza::Traza;

/// Formatos de exportacion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Formato {
    Csv,
    Json,
}

impl Formato {
    /// Lee `csv` o `json` (sin distinguir mayusculas).
    pub fn desde_nombre(nombre: &str) -> Option<Self> {
        match nombre.to_ascii_lowercase().as_str() {
            "csv" => Some(Formato::Csv),
            "json" => Some(Formato::Json),
            _ => None,
        }
    }

    /// El formato que corresponde a la extension de un archivo.
    pub fn de_archivo(ruta: &str) -> Option<Self> {
        ruta.rsplit_once('.')
            .and_then(|(_, e)| Self::desde_nombre(e))
    }
}

fn csv(campo: &str) -> String {
    if campo.contains([',', '"', '\n']) {
        format!("\"{}\"", campo.replace('"', "\"\""))
    } else {
        campo.to_string()
    }
}

fn json(texto: &str) -> String {
    let mut s = String::from("\"");
    for c in texto.chars() {
        match c {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            '\n' => s.push_str("\\n"),
            c if (c as u32) < 0x20 => {
                let _ = write!(s, "\\u{:04x}", c as u32);
            }
            c => s.push(c),
        }
    }
    s.push('"');
    s
}

/// Una fila por paso. Los cambios van como `nombre:antes>despues` separados por `;`.
pub fn a_csv(traza: &Traza) -> String {
    let mut s = String::from("instante,pc,mnemonico,ciclos,registros,banderas,escrituras\n");
    for p in traza.pasos() {
        let registros: Vec<String> = p
            .registros
            .iter()
            .map(|r| format!("{}:{:#x}>{:#x}", r.nombre, r.antes, r.despues))
            .collect();
        let banderas: Vec<String> = p
            .banderas
            .iter()
            .map(|b| format!("{}:{}>{}", b.nombre, u8::from(b.antes), u8::from(b.despues)))
            .collect();
        let escrituras: Vec<String> = p
            .escrituras
            .iter()
            .map(|e| format!("{:#x}:{:#x}>{:#x}", e.direccion, e.antes, e.despues))
            .collect();
        let _ = writeln!(
            s,
            "{},{:#x},{},{},{},{},{}",
            p.instante,
            p.pc,
            csv(&p.mnemonico),
            p.ciclos,
            csv(&registros.join(";")),
            csv(&banderas.join(";")),
            csv(&escrituras.join(";")),
        );
    }
    s
}

/// Un objeto JSON con la lista de pasos.
pub fn a_json(traza: &Traza) -> String {
    let mut s = String::from("{\"formato\":\"lorenz-traza-0.1\",\"pasos\":[");
    for (i, p) in traza.pasos().iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let _ = write!(
            s,
            "{{\"instante\":{},\"pc\":{},\"mnemonico\":{},\"ciclos\":{},\"registros\":[",
            p.instante,
            p.pc,
            json(&p.mnemonico),
            p.ciclos
        );
        for (j, r) in p.registros.iter().enumerate() {
            if j > 0 {
                s.push(',');
            }
            let _ = write!(
                s,
                "{{\"nombre\":{},\"antes\":{},\"despues\":{}}}",
                json(&r.nombre),
                r.antes,
                r.despues
            );
        }
        s.push_str("],\"banderas\":[");
        for (j, b) in p.banderas.iter().enumerate() {
            if j > 0 {
                s.push(',');
            }
            let _ = write!(
                s,
                "{{\"nombre\":{},\"antes\":{},\"despues\":{}}}",
                json(&b.nombre),
                b.antes,
                b.despues
            );
        }
        s.push_str("],\"escrituras\":[");
        for (j, e) in p.escrituras.iter().enumerate() {
            if j > 0 {
                s.push(',');
            }
            let _ = write!(
                s,
                "{{\"direccion\":{},\"antes\":{},\"despues\":{}}}",
                e.direccion, e.antes, e.despues
            );
        }
        s.push_str("]}");
    }
    s.push_str("]}\n");
    s
}

pub fn exportar(traza: &Traza, formato: Formato) -> String {
    match formato {
        Formato::Csv => a_csv(traza),
        Formato::Json => a_json(traza),
    }
}
