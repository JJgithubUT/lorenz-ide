//! Enrutador: metodo y ruta con parametros (`/api/sesion/:id/paso`) hacia un manejador.

use crate::{Peticion, Respuesta};

pub type Manejador = Box<dyn Fn(&Peticion) -> Respuesta + Send + Sync>;

enum Segmento {
    Fijo(String),
    Parametro(String),
    /// `*resto`: lo que quede de la ruta, con barras.
    Resto(String),
}

struct Ruta {
    metodo: &'static str,
    segmentos: Vec<Segmento>,
    manejador: Manejador,
}

#[derive(Default)]
pub struct Router {
    rutas: Vec<Ruta>,
}

fn analizar(patron: &str) -> Vec<Segmento> {
    patron
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| {
            if let Some(n) = s.strip_prefix(':') {
                Segmento::Parametro(n.into())
            } else if let Some(n) = s.strip_prefix('*') {
                Segmento::Resto(n.into())
            } else {
                Segmento::Fijo(s.into())
            }
        })
        .collect()
}

fn encaja(segmentos: &[Segmento], ruta: &str) -> Option<Vec<(String, String)>> {
    let partes: Vec<&str> = ruta
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    let mut params = Vec::new();
    for (i, s) in segmentos.iter().enumerate() {
        match s {
            Segmento::Resto(n) => {
                params.push((n.clone(), partes.get(i..)?.join("/")));
                return Some(params);
            }
            Segmento::Fijo(f) => {
                if partes.get(i) != Some(&f.as_str()) {
                    return None;
                }
            }
            Segmento::Parametro(n) => params.push((n.clone(), (*partes.get(i)?).to_string())),
        }
    }
    (partes.len() == segmentos.len()).then_some(params)
}

impl Router {
    pub fn nuevo() -> Self {
        Self::default()
    }

    pub fn ruta(
        mut self,
        metodo: &'static str,
        patron: &str,
        f: impl Fn(&Peticion) -> Respuesta + Send + Sync + 'static,
    ) -> Self {
        self.rutas.push(Ruta {
            metodo,
            segmentos: analizar(patron),
            manejador: Box::new(f),
        });
        self
    }

    pub fn get(
        self,
        patron: &str,
        f: impl Fn(&Peticion) -> Respuesta + Send + Sync + 'static,
    ) -> Self {
        self.ruta("GET", patron, f)
    }

    pub fn post(
        self,
        patron: &str,
        f: impl Fn(&Peticion) -> Respuesta + Send + Sync + 'static,
    ) -> Self {
        self.ruta("POST", patron, f)
    }

    pub fn put(
        self,
        patron: &str,
        f: impl Fn(&Peticion) -> Respuesta + Send + Sync + 'static,
    ) -> Self {
        self.ruta("PUT", patron, f)
    }

    pub fn delete(
        self,
        patron: &str,
        f: impl Fn(&Peticion) -> Respuesta + Send + Sync + 'static,
    ) -> Self {
        self.ruta("DELETE", patron, f)
    }

    /// Atiende una peticion: 404 si ninguna ruta encaja, 405 con `Allow` si encaja con otro metodo.
    /// `HEAD` se trata como `GET` sin cuerpo.
    pub fn despachar(&self, p: &Peticion) -> Respuesta {
        let metodo = if p.metodo == "HEAD" {
            "GET"
        } else {
            p.metodo.as_str()
        };
        let mut permitidos: Vec<&str> = Vec::new();
        for r in &self.rutas {
            if let Some(params) = encaja(&r.segmentos, &p.ruta) {
                if r.metodo == metodo {
                    let mut con = p.clone();
                    con.parametros = params.into_iter().collect();
                    let mut respuesta = (r.manejador)(&con);
                    if p.metodo == "HEAD" {
                        respuesta
                            .cabeceras
                            .push(("X-Head-Longitud".into(), respuesta.cuerpo.len().to_string()));
                        respuesta.cuerpo.clear();
                    }
                    return respuesta;
                }
                if !permitidos.contains(&r.metodo) {
                    permitidos.push(r.metodo);
                }
            }
        }
        if permitidos.is_empty() {
            Respuesta::texto(404, "no existe")
        } else {
            Respuesta::texto(405, "metodo no permitido")
                .con_cabecera("Allow", &permitidos.join(", "))
        }
    }
}
