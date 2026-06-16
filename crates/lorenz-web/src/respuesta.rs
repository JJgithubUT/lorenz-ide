//! Respuestas HTTP.

/// Una respuesta lista para escribir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Respuesta {
    pub estado: u16,
    pub cabeceras: Vec<(String, String)>,
    pub cuerpo: Vec<u8>,
}

pub fn motivo(estado: u16) -> &'static str {
    match estado {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        411 => "Length Required",
        413 => "Payload Too Large",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        _ => "OK",
    }
}

impl Respuesta {
    pub fn nueva(estado: u16, tipo: &str, cuerpo: impl Into<Vec<u8>>) -> Self {
        Self {
            estado,
            cabeceras: vec![("Content-Type".into(), tipo.into())],
            cuerpo: cuerpo.into(),
        }
    }

    pub fn texto(estado: u16, texto: &str) -> Self {
        Self::nueva(estado, "text/plain; charset=utf-8", texto)
    }

    pub fn html(texto: &str) -> Self {
        Self::nueva(200, "text/html; charset=utf-8", texto)
    }

    /// JSON ya serializado.
    pub fn json(estado: u16, texto: &str) -> Self {
        Self::nueva(estado, "application/json; charset=utf-8", texto)
    }

    pub fn vacia(estado: u16) -> Self {
        Self {
            estado,
            cabeceras: Vec::new(),
            cuerpo: Vec::new(),
        }
    }

    pub fn con_cabecera(mut self, nombre: &str, valor: &str) -> Self {
        self.cabeceras.push((nombre.into(), valor.into()));
        self
    }

    /// Los bytes que se escriben en la conexion.
    pub fn a_bytes(&self) -> Vec<u8> {
        let mut s = format!("HTTP/1.1 {} {}\r\n", self.estado, motivo(self.estado));
        for (k, v) in &self.cabeceras {
            // Una cabecera con saltos de linea permitiria inyectar otras.
            let limpio = |t: &str| t.replace(['\r', '\n'], " ");
            s.push_str(&format!("{}: {}\r\n", limpio(k), limpio(v)));
        }
        s.push_str(&format!(
            "Content-Length: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\n\r\n",
            self.cuerpo.len()
        ));
        let mut b = s.into_bytes();
        b.extend_from_slice(&self.cuerpo);
        b
    }
}
