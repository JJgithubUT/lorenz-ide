//! Cliente HTTP minimo para bajar paquetes (RF-ECO-004).
//!
//! Solo `http://`: Lorenz no trae TLS (decision D-115). La integridad la da el SHA-256 del paquete,
//! que quien instala fija con `--sha256`; sin esa huella o sobre una huella distinta, el paquete
//! no se acepta. Soporta `Content-Length`, `Transfer-Encoding: chunked`, el cierre de la conexion y
//! hasta cinco redirecciones a otras direcciones `http://`.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// Tamano maximo que se acepta de un paquete.
pub const MAXIMO: usize = 32 * 1024 * 1024;
const ESPERA: Duration = Duration::from_secs(10);
const REDIRECCIONES: usize = 5;

/// Una direccion `http://host[:puerto]/ruta` ya separada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    pub host: String,
    pub puerto: u16,
    pub ruta: String,
}

pub fn es_url(texto: &str) -> bool {
    let t = texto.to_ascii_lowercase();
    t.starts_with("http://") || t.starts_with("https://")
}

impl Url {
    pub fn analizar(texto: &str) -> Result<Self, String> {
        let minus = texto.to_ascii_lowercase();
        if minus.starts_with("https://") {
            return Err(format!(
                "{texto}: Lorenz no trae HTTPS; baja el archivo con otro programa e instalalo desde el disco, o usa una direccion http:// con --sha256"
            ));
        }
        let resto = minus
            .starts_with("http://")
            .then(|| &texto["http://".len()..])
            .ok_or_else(|| format!("{texto}: no es una direccion http://"))?;
        let (autoridad, ruta) = match resto.find('/') {
            Some(i) => (&resto[..i], &resto[i..]),
            None => (resto, "/"),
        };
        if autoridad.is_empty() || autoridad.contains('@') {
            return Err(format!("{texto}: direccion invalida"));
        }
        let (host, puerto) = match autoridad.rsplit_once(':') {
            Some((h, p)) => (
                h,
                p.parse::<u16>()
                    .map_err(|_| format!("{texto}: puerto invalido"))?,
            ),
            None => (autoridad, 80),
        };
        if host.is_empty() || ruta.contains(['\r', '\n', ' ']) {
            return Err(format!("{texto}: direccion invalida"));
        }
        Ok(Self {
            host: host.into(),
            puerto,
            ruta: ruta.into(),
        })
    }

    fn a_texto(&self) -> String {
        format!("http://{}:{}{}", self.host, self.puerto, self.ruta)
    }
}

fn leer_linea(flujo: &mut impl Read) -> Result<String, String> {
    let mut l = Vec::new();
    let mut b = [0u8; 1];
    loop {
        match flujo.read(&mut b) {
            Ok(0) => break,
            Ok(_) => {
                if b[0] == b'\n' {
                    break;
                }
                l.push(b[0]);
                if l.len() > 16 * 1024 {
                    return Err("encabezado demasiado largo".into());
                }
            }
            Err(e) => return Err(format!("lectura: {e}")),
        }
    }
    Ok(String::from_utf8_lossy(&l)
        .trim_end_matches('\r')
        .to_string())
}

fn leer_cuerpo(
    flujo: &mut impl Read,
    longitud: Option<usize>,
    troceado: bool,
    maximo: usize,
) -> Result<Vec<u8>, String> {
    let mut cuerpo = Vec::new();
    if troceado {
        loop {
            let linea = leer_linea(flujo)?;
            let tamano = usize::from_str_radix(linea.split(';').next().unwrap_or("").trim(), 16)
                .map_err(|_| format!("tamano de trozo invalido: `{linea}`"))?;
            if tamano == 0 {
                break;
            }
            if cuerpo.len() + tamano > maximo {
                return Err(format!("el paquete pasa del maximo de {maximo} bytes"));
            }
            let inicio = cuerpo.len();
            cuerpo.resize(inicio + tamano, 0);
            flujo
                .read_exact(&mut cuerpo[inicio..])
                .map_err(|e| format!("trozo incompleto: {e}"))?;
            let _ = leer_linea(flujo)?;
        }
        return Ok(cuerpo);
    }
    if let Some(n) = longitud {
        if n > maximo {
            return Err(format!("el paquete mide {n} bytes y el maximo es {maximo}"));
        }
        cuerpo.resize(n, 0);
        flujo
            .read_exact(&mut cuerpo)
            .map_err(|e| format!("respuesta incompleta: {e}"))?;
        return Ok(cuerpo);
    }
    flujo
        .take(maximo as u64 + 1)
        .read_to_end(&mut cuerpo)
        .map_err(|e| format!("lectura: {e}"))?;
    if cuerpo.len() > maximo {
        return Err(format!("el paquete pasa del maximo de {maximo} bytes"));
    }
    Ok(cuerpo)
}

fn pedir(url: &Url, maximo: usize) -> Result<(u16, Option<String>, Vec<u8>), String> {
    let destino = (url.host.as_str(), url.puerto)
        .to_socket_addrs()
        .map_err(|e| format!("{}: no se resuelve: {e}", url.host))?
        .next()
        .ok_or_else(|| format!("{}: no se resuelve", url.host))?;
    let mut flujo = TcpStream::connect_timeout(&destino, ESPERA)
        .map_err(|e| format!("{}: no se pudo conectar: {e}", url.a_texto()))?;
    flujo
        .set_read_timeout(Some(ESPERA))
        .map_err(|e| e.to_string())?;
    flujo
        .set_write_timeout(Some(ESPERA))
        .map_err(|e| e.to_string())?;
    write!(
        flujo,
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: lorenz/{}\r\nAccept: */*\r\nConnection: close\r\n\r\n",
        url.ruta,
        url.host,
        env!("CARGO_PKG_VERSION")
    )
    .map_err(|e| format!("no se pudo enviar la peticion: {e}"))?;
    let estado = leer_linea(&mut flujo)?;
    let codigo: u16 = estado
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .ok_or_else(|| format!("respuesta invalida: `{estado}`"))?;
    let (mut longitud, mut troceado, mut ubicacion) = (None, false, None);
    loop {
        let l = leer_linea(&mut flujo)?;
        if l.is_empty() {
            break;
        }
        if let Some((k, v)) = l.split_once(':') {
            let v = v.trim();
            match k.trim().to_ascii_lowercase().as_str() {
                "content-length" => longitud = v.parse().ok(),
                "transfer-encoding" => troceado = v.to_ascii_lowercase().contains("chunked"),
                "location" => ubicacion = Some(v.to_string()),
                _ => {}
            }
        }
    }
    if (300..400).contains(&codigo) {
        return Ok((codigo, ubicacion, Vec::new()));
    }
    let cuerpo = leer_cuerpo(&mut flujo, longitud, troceado, maximo)?;
    Ok((codigo, ubicacion, cuerpo))
}

/// Baja el contenido de una direccion `http://`.
pub fn descargar(direccion: &str) -> Result<Vec<u8>, String> {
    let mut url = Url::analizar(direccion)?;
    for _ in 0..=REDIRECCIONES {
        let (codigo, ubicacion, cuerpo) = pedir(&url, MAXIMO)?;
        match codigo {
            200 => return Ok(cuerpo),
            301 | 302 | 303 | 307 | 308 => {
                let l = ubicacion.ok_or("redireccion sin Location")?;
                url = if es_url(&l) {
                    Url::analizar(&l)?
                } else if l.starts_with('/') {
                    Url { ruta: l, ..url }
                } else {
                    return Err(format!("redireccion a `{l}`, que no se admite"));
                };
            }
            otro => return Err(format!("{}: el servidor respondio {otro}", url.a_texto())),
        }
    }
    Err("demasiadas redirecciones".into())
}
