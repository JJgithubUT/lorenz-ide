//! Lectura y analisis de una peticion HTTP/1.x.

use std::collections::HashMap;
use std::io::Read;

use crate::servidor::Limites;

/// Una peticion ya analizada.
#[derive(Debug, Clone, Default)]
pub struct Peticion {
    pub metodo: String,
    /// Ruta decodificada, sin la consulta.
    pub ruta: String,
    pub consulta: Vec<(String, String)>,
    /// Cabeceras con el nombre en minusculas.
    pub cabeceras: Vec<(String, String)>,
    pub cuerpo: Vec<u8>,
    /// Parametros de la ruta (`/api/:id`), rellenados por el enrutador.
    pub parametros: HashMap<String, String>,
}

/// Por que no se pudo leer una peticion; cada caso tiene su respuesta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorPeticion {
    /// La conexion se cerro antes de enviar nada.
    Vacia,
    Mal(String),
    CabecerasGrandes,
    CuerpoGrande,
    SinLongitud,
    NoSoportado(String),
    Espera,
}

impl Peticion {
    pub fn cabecera(&self, nombre: &str) -> Option<&str> {
        let n = nombre.to_ascii_lowercase();
        self.cabeceras
            .iter()
            .find(|(k, _)| *k == n)
            .map(|(_, v)| v.as_str())
    }

    pub fn consulta(&self, nombre: &str) -> Option<&str> {
        self.consulta
            .iter()
            .find(|(k, _)| k == nombre)
            .map(|(_, v)| v.as_str())
    }

    pub fn parametro(&self, nombre: &str) -> Option<&str> {
        self.parametros.get(nombre).map(String::as_str)
    }

    pub fn cuerpo_texto(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.cuerpo)
    }
}

/// Decodifica `%XX` y, si `mas`, `+` como espacio. Falla si el resultado no es UTF-8.
pub fn decodificar(texto: &str, mas: bool) -> Option<String> {
    let b = texto.as_bytes();
    let mut o = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' => {
                let h = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
                o.push(u8::from_str_radix(h, 16).ok()?);
                i += 3;
            }
            b'+' if mas => {
                o.push(b' ');
                i += 1;
            }
            c => {
                o.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8(o).ok()
}

fn analizar_consulta(q: &str) -> Option<Vec<(String, String)>> {
    let mut v = Vec::new();
    for par in q.split('&').filter(|p| !p.is_empty()) {
        let (k, val) = par.split_once('=').unwrap_or((par, ""));
        v.push((decodificar(k, true)?, decodificar(val, true)?));
    }
    Some(v)
}

/// Lee una peticion completa del flujo.
pub fn leer(flujo: &mut impl Read, limites: &Limites) -> Result<Peticion, ErrorPeticion> {
    let mut cab = Vec::with_capacity(1024);
    let mut resto: Vec<u8> = Vec::new();
    let mut buf = [0u8; 4096];
    let fin = loop {
        if let Some(p) = cab.windows(4).position(|w| w == b"\r\n\r\n") {
            break p;
        }
        if cab.len() > limites.cabeceras_max {
            return Err(ErrorPeticion::CabecerasGrandes);
        }
        match flujo.read(&mut buf) {
            Ok(0) => {
                return Err(if cab.is_empty() {
                    ErrorPeticion::Vacia
                } else {
                    ErrorPeticion::Mal("conexion cerrada a medias".into())
                });
            }
            Ok(n) => cab.extend_from_slice(&buf[..n]),
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                return Err(ErrorPeticion::Espera)
            }
            Err(e) => return Err(ErrorPeticion::Mal(e.to_string())),
        }
    };
    if fin > limites.cabeceras_max {
        return Err(ErrorPeticion::CabecerasGrandes);
    }
    resto.extend_from_slice(&cab[fin + 4..]);
    let texto = std::str::from_utf8(&cab[..fin])
        .map_err(|_| ErrorPeticion::Mal("cabeceras que no son UTF-8".into()))?;
    let mut lineas = texto.split("\r\n");
    let primera = lineas.next().unwrap_or("");
    let mut partes = primera.split(' ');
    let (metodo, objetivo, version) = (
        partes.next().unwrap_or(""),
        partes.next().unwrap_or(""),
        partes.next().unwrap_or(""),
    );
    if partes.next().is_some()
        || metodo.is_empty()
        || !metodo.bytes().all(|c| c.is_ascii_uppercase())
        || !version.starts_with("HTTP/1.")
    {
        return Err(ErrorPeticion::Mal(format!(
            "linea de peticion invalida: `{primera}`"
        )));
    }
    let (ruta_cruda, q) = objetivo.split_once('?').unwrap_or((objetivo, ""));
    if !ruta_cruda.starts_with('/') {
        return Err(ErrorPeticion::Mal("la ruta debe empezar por /".into()));
    }
    let ruta = decodificar(ruta_cruda, false)
        .ok_or_else(|| ErrorPeticion::Mal("ruta mal codificada".into()))?;
    if ruta.contains('\0') {
        return Err(ErrorPeticion::Mal("ruta con caracteres nulos".into()));
    }
    let consulta =
        analizar_consulta(q).ok_or_else(|| ErrorPeticion::Mal("consulta mal codificada".into()))?;
    let mut cabeceras = Vec::new();
    for l in lineas {
        let (k, v) = l
            .split_once(':')
            .ok_or_else(|| ErrorPeticion::Mal(format!("cabecera invalida: `{l}`")))?;
        cabeceras.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
    }
    let mut p = Peticion {
        metodo: metodo.into(),
        ruta,
        consulta,
        cabeceras,
        cuerpo: Vec::new(),
        parametros: HashMap::new(),
    };
    if p.cabecera("transfer-encoding").is_some() {
        return Err(ErrorPeticion::NoSoportado("Transfer-Encoding".into()));
    }
    let largo = match p.cabecera("content-length") {
        Some(l) => Some(
            l.parse::<usize>()
                .map_err(|_| ErrorPeticion::Mal("Content-Length invalido".into()))?,
        ),
        None => None,
    };
    match largo {
        None if matches!(p.metodo.as_str(), "POST" | "PUT" | "PATCH") => {
            return Err(ErrorPeticion::SinLongitud)
        }
        None | Some(0) => {}
        Some(n) if n > limites.cuerpo_max => return Err(ErrorPeticion::CuerpoGrande),
        Some(n) => {
            let mut cuerpo = resto;
            cuerpo.truncate(n);
            while cuerpo.len() < n {
                match flujo.read(&mut buf) {
                    Ok(0) => return Err(ErrorPeticion::Mal("cuerpo incompleto".into())),
                    Ok(k) => {
                        let faltan = n - cuerpo.len();
                        cuerpo.extend_from_slice(&buf[..k.min(faltan)]);
                    }
                    Err(e)
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) =>
                    {
                        return Err(ErrorPeticion::Espera)
                    }
                    Err(e) => return Err(ErrorPeticion::Mal(e.to_string())),
                }
            }
            p.cuerpo = cuerpo;
        }
    }
    Ok(p)
}
