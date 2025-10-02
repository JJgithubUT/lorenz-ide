//! El interprete de ordenes del depurador, en español (RF-DEP-001, RF-DEP-017, RF-DEP-018).
//!
//! [`Sesion::ejecutar`] recibe una linea y devuelve el texto a mostrar; no toca la terminal.

use crate::depurador::{Depurador, Parada, Vigilar};
use crate::error::ErrorDepurador;
use crate::{numero, vistas};

/// Cuantas instrucciones corre `continuar` como maximo si no se indica otra cosa.
pub const LIMITE_POR_DEFECTO: u64 = 10_000_000;

/// Resultado de una orden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Respuesta {
    pub texto: String,
    /// `true` si la sesion debe terminar.
    pub salir: bool,
}

impl Respuesta {
    fn texto(t: impl Into<String>) -> Self {
        Respuesta {
            texto: t.into(),
            salir: false,
        }
    }
}

/// Una sesion de depuracion: el depurador y la ultima orden, para repetirla con una linea vacia.
pub struct Sesion {
    pub depurador: Depurador,
    ultima: Option<String>,
}

const AYUDA: &str = "\
Ordenes (entre parentesis, la forma corta):
  paso [n] (s)            ejecuta n instrucciones (1 por omision)
  continuar [n] (c)       ejecuta hasta una parada; n limita las instrucciones
  hasta UBICACION         ejecuta hasta llegar a la ubicacion
  salir                   ejecuta hasta volver de la subrutina actual
  registros (r)           muestra registros y banderas
  volcar DIR [n] (v)      muestra n celdas desde DIR
  desensamblar [DIR] [n] (d)  muestra el codigo, con la linea de fuente
  pila [n]                muestra la cima de la pila
  fijar NOMBRE VALOR      cambia un registro o una bandera
  escribir DIR VALOR      cambia una celda de memoria
  puerto N [VALOR]        lee el puerto N, o le escribe VALOR
  entrada N VALOR         deja VALOR en la cola de entrada del puerto N
  imprimir EXPR (p)       evalua una expresion con registros, simbolos y mem[...]
  ruptura UBIC [si COND] (b)  pone un punto de ruptura
  observar DIR [lectura|escritura|acceso]  detiene al acceder a una celda
  puntos                  lista rupturas y observaciones
  borrar N                quita un punto
  activar N / desactivar N
  ayuda (h)               esta ayuda
  fin (q)                 termina
UBICACION es una direccion, una etiqueta o archivo:linea. Una linea vacia repite la ultima orden.";

fn pedir<'a>(
    partes: &[&'a str],
    i: usize,
    orden: &'static str,
    falta: &'static str,
) -> Result<&'a str, ErrorDepurador> {
    partes
        .get(i)
        .copied()
        .ok_or(ErrorDepurador::FaltaArgumento { orden, falta })
}

fn sin_mas(partes: &[&str], desde: usize, orden: &'static str) -> Result<(), ErrorDepurador> {
    match partes.get(desde) {
        Some(extra) => Err(ErrorDepurador::SobraArgumento {
            orden,
            sobra: (*extra).to_string(),
        }),
        None => Ok(()),
    }
}

impl Sesion {
    pub fn nueva(depurador: Depurador) -> Self {
        Sesion {
            depurador,
            ultima: None,
        }
    }

    /// Ejecuta una linea. Los errores se devuelven como texto; la sesion sigue viva.
    pub fn ejecutar(&mut self, linea: &str) -> Respuesta {
        let linea = linea.trim();
        let linea = if linea.is_empty() {
            match &self.ultima {
                Some(u) => u.clone(),
                None => return Respuesta::texto(""),
            }
        } else {
            linea.to_string()
        };
        match self.orden(&linea) {
            Ok(r) => {
                self.ultima = Some(linea);
                r
            }
            Err(e) => Respuesta::texto(format!("error: {e}")),
        }
    }

    fn orden(&mut self, linea: &str) -> Result<Respuesta, ErrorDepurador> {
        // La condicion de `ruptura` conserva sus espacios, asi que se separa antes.
        let (cabeza, condicion) = match linea.split_once(" si ") {
            Some((a, b)) => (a, Some(b.trim())),
            None => (linea, None),
        };
        let partes: Vec<&str> = cabeza.split_whitespace().collect();
        let Some(&nombre) = partes.first() else {
            return Ok(Respuesta::texto(""));
        };
        if condicion.is_some() && !matches!(nombre, "ruptura" | "b") {
            return Err(ErrorDepurador::SobraArgumento {
                orden: "orden",
                sobra: format!("si {}", condicion.unwrap_or_default()),
            });
        }
        let d = &mut self.depurador;
        match nombre {
            "paso" | "s" => {
                sin_mas(&partes, 2, "paso")?;
                let n = match partes.get(1) {
                    Some(t) => numero::analizar(t)?,
                    None => 1,
                };
                let p = d.paso(n);
                Ok(Respuesta::texto(Self::tras_parada(d, &p)))
            }
            "continuar" | "c" => {
                sin_mas(&partes, 2, "continuar")?;
                let n = match partes.get(1) {
                    Some(t) => numero::analizar(t)?,
                    None => LIMITE_POR_DEFECTO,
                };
                let p = d.continuar(n);
                Ok(Respuesta::texto(Self::tras_parada(d, &p)))
            }
            "hasta" => {
                sin_mas(&partes, 2, "hasta")?;
                let dir = d.resolver(pedir(&partes, 1, "hasta", "la ubicacion")?)?;
                let p = d.hasta(dir, LIMITE_POR_DEFECTO);
                Ok(Respuesta::texto(Self::tras_parada(d, &p)))
            }
            "salir" => {
                sin_mas(&partes, 1, "salir")?;
                let p = d.salir(LIMITE_POR_DEFECTO)?;
                Ok(Respuesta::texto(Self::tras_parada(d, &p)))
            }
            "registros" | "r" => {
                sin_mas(&partes, 1, "registros")?;
                Ok(Respuesta::texto(vistas::registros(d)))
            }
            "volcar" | "v" => {
                sin_mas(&partes, 3, "volcar")?;
                let dir = d.resolver(pedir(&partes, 1, "volcar", "la direccion")?)?;
                let n = match partes.get(2) {
                    Some(t) => numero::analizar(t)? as usize,
                    None => 64,
                };
                Ok(Respuesta::texto(vistas::volcar(d, dir, n)?))
            }
            "desensamblar" | "d" => {
                sin_mas(&partes, 3, "desensamblar")?;
                let dir = match partes.get(1) {
                    Some(t) => d.resolver(t)?,
                    None => d.emulador().estado().pc(),
                };
                let n = match partes.get(2) {
                    Some(t) => numero::analizar(t)? as usize,
                    None => 8,
                };
                Ok(Respuesta::texto(vistas::listar(d, dir, n)))
            }
            "pila" => {
                sin_mas(&partes, 2, "pila")?;
                let n = match partes.get(1) {
                    Some(t) => numero::analizar(t)? as usize,
                    None => 8,
                };
                Ok(Respuesta::texto(vistas::pila(d, n)?))
            }
            "fijar" => {
                sin_mas(&partes, 3, "fijar")?;
                let nombre = pedir(&partes, 1, "fijar", "el nombre")?;
                let valor = numero::analizar(pedir(&partes, 2, "fijar", "el valor")?)?;
                if d.emulador().estado().tiene_bandera(nombre) {
                    d.fijar_bandera(nombre, valor != 0)?;
                } else {
                    d.fijar_registro(nombre, valor)?;
                }
                Ok(Respuesta::texto(format!("{nombre} = {valor:#x}")))
            }
            "escribir" => {
                sin_mas(&partes, 3, "escribir")?;
                let dir = d.resolver(pedir(&partes, 1, "escribir", "la direccion")?)?;
                let valor = numero::analizar(pedir(&partes, 2, "escribir", "el valor")?)?;
                d.emulador_mut()
                    .memoria_mut()
                    .escribir_privilegiado(dir, valor)?;
                Ok(Respuesta::texto(format!("[{dir:#x}] = {valor:#x}")))
            }
            "puerto" => {
                sin_mas(&partes, 3, "puerto")?;
                let n = numero::analizar(pedir(&partes, 1, "puerto", "el numero")?)?;
                match partes.get(2) {
                    Some(t) => {
                        let v = numero::analizar(t)?;
                        d.escribir_puerto(n, v)?;
                        Ok(Respuesta::texto(format!("puerto {n:#x} <- {v:#x}")))
                    }
                    None => {
                        let v = d.leer_puerto(n)?;
                        Ok(Respuesta::texto(format!("puerto {n:#x} -> {v:#x}")))
                    }
                }
            }
            "entrada" => {
                sin_mas(&partes, 3, "entrada")?;
                let n = numero::analizar(pedir(&partes, 1, "entrada", "el numero")?)?;
                let v = numero::analizar(pedir(&partes, 2, "entrada", "el valor")?)?;
                d.encolar_entrada(n, v)?;
                Ok(Respuesta::texto(format!("puerto {n:#x}: {v:#x} en cola")))
            }
            "imprimir" | "p" => {
                let expr = cabeza[nombre.len()..].trim();
                if expr.is_empty() {
                    return Err(ErrorDepurador::FaltaArgumento {
                        orden: "imprimir",
                        falta: "la expresion",
                    });
                }
                let v = d.evaluar(expr)?;
                Ok(Respuesta::texto(format!("{v} ({v:#x})")))
            }
            "ruptura" | "b" => {
                sin_mas(&partes, 2, "ruptura")?;
                let dir = d.resolver(pedir(&partes, 1, "ruptura", "la ubicacion")?)?;
                let id = d.poner_ruptura(dir, condicion)?;
                Ok(Respuesta::texto(format!("ruptura {id} en {dir:#x}")))
            }
            "observar" => {
                sin_mas(&partes, 3, "observar")?;
                let dir = d.resolver(pedir(&partes, 1, "observar", "la direccion")?)?;
                let tipo = match partes.get(2).copied() {
                    None | Some("acceso") => Vigilar::Acceso,
                    Some("lectura") => Vigilar::Lectura,
                    Some("escritura") => Vigilar::Escritura,
                    Some(otro) => return Err(ErrorDepurador::OrdenDesconocida(otro.to_string())),
                };
                let id = d.observar(dir, tipo)?;
                Ok(Respuesta::texto(format!(
                    "observacion {id} de {} en {dir:#x}",
                    tipo.nombre()
                )))
            }
            "puntos" => {
                sin_mas(&partes, 1, "puntos")?;
                Ok(Respuesta::texto(Self::puntos(d)))
            }
            "borrar" | "activar" | "desactivar" => {
                sin_mas(&partes, 2, "borrar")?;
                let id = numero::analizar(pedir(&partes, 1, "borrar", "el numero")?)? as u32;
                match nombre {
                    "borrar" => d.borrar(id)?,
                    "activar" => d.activar(id, true)?,
                    _ => d.activar(id, false)?,
                }
                Ok(Respuesta::texto(format!("{nombre}: {id}")))
            }
            "ayuda" | "h" => Ok(Respuesta::texto(AYUDA)),
            "fin" | "q" => Ok(Respuesta {
                texto: String::new(),
                salir: true,
            }),
            otra => Err(ErrorDepurador::OrdenDesconocida(otra.to_string())),
        }
    }

    /// Texto de una parada seguido de la instruccion que toca ejecutar (RF-DEP-016).
    fn tras_parada(d: &Depurador, p: &Parada) -> String {
        let mut s = vistas::parada(d, p);
        let pc = d.emulador().estado().pc();
        if !matches!(p, Parada::Detenida | Parada::Fallo { .. }) {
            if let Some(i) = vistas::instruccion_en(d, pc) {
                s.push_str(&format!("\n=> {pc:#x}  {i}"));
            }
        }
        s
    }

    fn puntos(d: &Depurador) -> String {
        let mut s = String::new();
        for r in d.rupturas() {
            s.push_str(&format!(
                "{} ruptura en {:#x}{}{} ({} aciertos)\n",
                r.id,
                r.direccion,
                r.condicion
                    .as_ref()
                    .map(|c| format!(" si {}", c.texto()))
                    .unwrap_or_default(),
                if r.activa { "" } else { " [desactivada]" },
                r.aciertos
            ));
        }
        for o in d.observaciones() {
            s.push_str(&format!(
                "{} observacion de {} en {:#x} ({} aciertos)\n",
                o.id,
                o.tipo.nombre(),
                o.direccion,
                o.aciertos
            ));
        }
        if s.is_empty() {
            s.push_str("no hay puntos\n");
        }
        s
    }
}
