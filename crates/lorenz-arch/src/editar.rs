//! Edicion de descripciones de arquitectura que conserva comentarios y formato (RF-ARQ-040).
//!
//! El editor visual no regenera el archivo: cada cambio del diagrama se aplica sobre el texto
//! original con `toml_edit`, de modo que los comentarios, el orden y el aspecto de lo que no se
//! toco quedan como estaban (decision D-114). Lo que el diagrama muestra (RF-ARQ-039) sale del
//! mismo texto.

use std::str::FromStr;

use toml_edit::{ArrayOfTables, DocumentMut, InlineTable, Item, Table, Value};

/// Un paso en la ruta hasta un elemento del documento.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seg {
    Clave(String),
    Indice(usize),
}

/// Un valor escalar que el diagrama sabe escribir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Valor {
    Entero(i64),
    Texto(String),
    Bool(bool),
}

/// Lo que representa un bloque del diagrama.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clase {
    Maquina,
    Interrupciones,
    Bandera,
    Registro,
    Region,
    Dispositivo,
    Instruccion,
    Otro,
}

impl Clase {
    pub fn nombre(self) -> &'static str {
        match self {
            Clase::Maquina => "maquina",
            Clase::Interrupciones => "interrupciones",
            Clase::Bandera => "bandera",
            Clase::Registro => "registro",
            Clase::Region => "region",
            Clase::Dispositivo => "dispositivo",
            Clase::Instruccion => "instruccion",
            Clase::Otro => "otro",
        }
    }
}

/// Un campo de un bloque: su valor si es escalar, y siempre el texto tal como esta escrito.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Campo {
    pub clave: String,
    pub valor: Option<Valor>,
    pub texto: String,
}

/// Un bloque del diagrama: una tabla de la descripcion con su lugar en el archivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bloque {
    pub clase: Clase,
    pub nombre: String,
    pub ruta: Vec<Seg>,
    /// Primera y ultima linea del bloque en el texto (desde 1).
    pub linea: usize,
    pub fin: usize,
    pub campos: Vec<Campo>,
}

/// Un cambio pedido desde el diagrama.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cambio {
    /// Escribe un valor en `ruta` (la ultima clave se crea si falta).
    Fijar { ruta: Vec<Seg>, valor: Valor },
    /// Quita la clave o el elemento que `ruta` senala.
    Quitar { ruta: Vec<Seg> },
    /// Anade un elemento a una lista de tablas (`registro`, `region`, `dispositivo`) o una bandera.
    Anadir {
        lista: String,
        campos: Vec<(String, Valor)>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorEdicion(pub String);

impl std::fmt::Display for ErrorEdicion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ErrorEdicion {}

fn err<T>(m: impl Into<String>) -> Result<T, ErrorEdicion> {
    Err(ErrorEdicion(m.into()))
}

/// Listas de tablas que el diagrama puede ampliar.
const LISTAS: &[&str] = &["registro", "region", "dispositivo"];

pub struct Documento {
    texto: String,
    doc: DocumentMut,
}

impl Documento {
    pub fn analizar(texto: &str) -> Result<Self, ErrorEdicion> {
        let doc = DocumentMut::from_str(texto)
            .map_err(|e| ErrorEdicion(format!("el texto no es TOML valido: {e}")))?;
        Ok(Self {
            texto: texto.to_string(),
            doc,
        })
    }

    /// El texto del documento tal como esta ahora (con los cambios aplicados).
    pub fn texto(&self) -> String {
        self.doc.to_string()
    }

    /// Los bloques del diagrama, en el orden del archivo.
    pub fn bloques(&self) -> Vec<Bloque> {
        let lineas: Vec<&str> = self.texto.lines().collect();
        let cabeceras = cabeceras(&lineas);
        let mut contador: std::collections::HashMap<String, usize> = Default::default();
        let mut v = Vec::new();
        for (i, (nombre, es_lista, linea)) in cabeceras.iter().enumerate() {
            let fin = cabeceras
                .get(i + 1)
                .map_or(lineas.len(), |c| c.2 - 1)
                .max(*linea);
            // Las lineas en blanco y comentarios finales pertenecen al siguiente bloque.
            let fin = (*linea..=fin)
                .rev()
                .find(|l| {
                    let t = lineas[l - 1].trim();
                    !t.is_empty() && !t.starts_with('#')
                })
                .unwrap_or(*linea);
            let ruta: Vec<Seg> = if *es_lista {
                let n = contador.entry(nombre.clone()).or_insert(0);
                let r = vec![Seg::Clave(nombre.clone()), Seg::Indice(*n)];
                *n += 1;
                r
            } else {
                nombre
                    .split('.')
                    .map(|s| Seg::Clave(s.to_string()))
                    .collect()
            };
            if nombre == "flags" {
                v.extend(self.banderas(&lineas, *linea, fin));
                continue;
            }
            let Some(item) = navegar(self.doc.as_item(), &ruta) else {
                continue;
            };
            let campos = campos_de(item);
            let clase = match nombre.as_str() {
                "maquina" => Clase::Maquina,
                "maquina.interrupciones" => Clase::Interrupciones,
                "registro" => Clase::Registro,
                "region" => Clase::Region,
                "dispositivo" => Clase::Dispositivo,
                "instruccion" => Clase::Instruccion,
                _ => Clase::Otro,
            };
            let nombre_bloque = ["nombre", "mnemonico"]
                .iter()
                .find_map(|k| {
                    campos
                        .iter()
                        .find(|c| c.clave == *k)
                        .and_then(|c| match &c.valor {
                            Some(Valor::Texto(t)) => Some(t.clone()),
                            _ => None,
                        })
                })
                .unwrap_or_else(|| nombre.clone());
            v.push(Bloque {
                clase,
                nombre: nombre_bloque,
                ruta,
                linea: *linea,
                fin,
                campos,
            });
        }
        v
    }

    fn banderas(&self, lineas: &[&str], desde: usize, hasta: usize) -> Vec<Bloque> {
        let Some(t) = self.doc.get("flags").and_then(Item::as_table_like) else {
            return Vec::new();
        };
        t.iter()
            .map(|(k, item)| {
                let linea = (desde..=hasta)
                    .find(|l| {
                        let s = lineas[l - 1].trim_start();
                        s.starts_with(k) && s[k.len()..].trim_start().starts_with('=')
                    })
                    .unwrap_or(desde);
                Bloque {
                    clase: Clase::Bandera,
                    nombre: k.to_string(),
                    ruta: vec![Seg::Clave("flags".into()), Seg::Clave(k.to_string())],
                    linea,
                    fin: linea,
                    campos: campos_de(item),
                }
            })
            .collect()
    }

    /// Aplica un cambio. Si falla, el documento queda como estaba.
    pub fn aplicar(&mut self, cambio: &Cambio) -> Result<(), ErrorEdicion> {
        let mut copia = self.doc.clone();
        match cambio {
            Cambio::Fijar { ruta, valor } => fijar(&mut copia, ruta, valor)?,
            Cambio::Quitar { ruta } => quitar(&mut copia, ruta)?,
            Cambio::Anadir { lista, campos } => anadir(&mut copia, lista, campos)?,
        }
        self.doc = copia;
        // El nuevo texto pasa a ser el de referencia para las lineas.
        self.texto = self.doc.to_string();
        Ok(())
    }
}

/// `(nombre, es lista de tablas, linea)` de cada cabecera `[x]` o `[[x]]`.
fn cabeceras(lineas: &[&str]) -> Vec<(String, bool, usize)> {
    let mut v = Vec::new();
    for (i, l) in lineas.iter().enumerate() {
        let t = l.trim();
        if let Some(r) = t.strip_prefix("[[") {
            if let Some(n) = r.split("]]").next() {
                v.push((n.trim().to_string(), true, i + 1));
            }
        } else if let Some(r) = t.strip_prefix('[') {
            if let Some(n) = r.split(']').next() {
                v.push((n.trim().to_string(), false, i + 1));
            }
        }
    }
    v
}

fn navegar<'a>(raiz: &'a Item, ruta: &[Seg]) -> Option<&'a Item> {
    let mut actual = raiz;
    for s in ruta {
        actual = match s {
            Seg::Clave(k) => actual.get(k.as_str())?,
            Seg::Indice(i) => actual.get(*i)?,
        };
    }
    Some(actual)
}

fn navegar_mut<'a>(raiz: &'a mut Item, ruta: &[Seg]) -> Result<&'a mut Item, ErrorEdicion> {
    let mut actual = raiz;
    for s in ruta {
        actual = match s {
            Seg::Clave(k) => actual.get_mut(k.as_str()),
            Seg::Indice(i) => actual.get_mut(*i),
        }
        .ok_or_else(|| ErrorEdicion(format!("no existe {}", mostrar(ruta))))?;
    }
    Ok(actual)
}

fn mostrar(ruta: &[Seg]) -> String {
    ruta.iter()
        .map(|s| match s {
            Seg::Clave(k) => k.clone(),
            Seg::Indice(i) => format!("[{i}]"),
        })
        .collect::<Vec<_>>()
        .join(".")
}

fn campos_de(item: &Item) -> Vec<Campo> {
    let Some(t) = item.as_table_like() else {
        return Vec::new();
    };
    t.iter()
        .filter(|(_, i)| !i.is_table() && !i.is_array_of_tables())
        .map(|(k, i)| {
            let (valor, texto) = match i.as_value() {
                Some(v) => (
                    match v {
                        Value::Integer(n) => Some(Valor::Entero(*n.value())),
                        Value::String(s) => Some(Valor::Texto(s.value().clone())),
                        Value::Boolean(b) => Some(Valor::Bool(*b.value())),
                        _ => None,
                    },
                    repr(v),
                ),
                None => (None, String::new()),
            };
            Campo {
                clave: k.to_string(),
                valor,
                texto,
            }
        })
        .collect()
}

/// El valor escrito, sin los espacios ni el comentario que lo rodean.
fn repr(v: &Value) -> String {
    let mut c = v.clone();
    c.decor_mut().clear();
    c.to_string()
}

/// Un valor nuevo que conserva el aspecto del anterior: los comentarios de la linea y, para los
/// enteros, la base (`0x1F` sigue siendo hexadecimal).
fn valor_toml(nuevo: &Valor, viejo: Option<&Value>) -> Value {
    let mut v = match nuevo {
        Valor::Entero(n) => {
            let hex = viejo.is_some_and(|o| repr(o).starts_with("0x"));
            if hex && *n >= 0 {
                let digitos = viejo
                    .map(|o| repr(o).trim_start_matches("0x").len())
                    .unwrap_or(1);
                Value::from_str(&format!("0x{:0w$X}", n, w = digitos))
                    .unwrap_or_else(|_| Value::from(*n))
            } else {
                Value::from(*n)
            }
        }
        Valor::Texto(t) => Value::from(t.as_str()),
        Valor::Bool(b) => Value::from(*b),
    };
    if let Some(o) = viejo {
        *v.decor_mut() = o.decor().clone();
    }
    v
}

fn fijar(doc: &mut DocumentMut, ruta: &[Seg], valor: &Valor) -> Result<(), ErrorEdicion> {
    let Some((Seg::Clave(clave), padre)) = ruta.split_last().map(|(u, p)| (u.clone(), p)) else {
        return err("la ruta debe terminar en una clave");
    };
    let item = navegar_mut(doc.as_item_mut(), padre)?;
    let Some(tabla) = item.as_table_like_mut() else {
        return err(format!("{} no es una tabla", mostrar(padre)));
    };
    let viejo = tabla.get(&clave).and_then(Item::as_value).cloned();
    if viejo.is_none() && tabla.contains_key(&clave) {
        return err(format!("{} no es un valor simple", mostrar(ruta)));
    }
    tabla.insert(&clave, Item::Value(valor_toml(valor, viejo.as_ref())));
    Ok(())
}

fn quitar(doc: &mut DocumentMut, ruta: &[Seg]) -> Result<(), ErrorEdicion> {
    let Some((ultimo, padre)) = ruta.split_last() else {
        return err("la ruta esta vacia");
    };
    let item = navegar_mut(doc.as_item_mut(), padre)?;
    match ultimo {
        Seg::Indice(i) => {
            let Some(lista) = item.as_array_of_tables_mut() else {
                return err(format!("{} no es una lista de tablas", mostrar(padre)));
            };
            if *i >= lista.len() {
                return err(format!("no existe {}", mostrar(ruta)));
            }
            lista.remove(*i);
            if lista.is_empty() {
                // Una lista vacia no se escribe; se quita la clave para no dejar restos.
                if let Some((Seg::Clave(k), p2)) = padre.split_last().map(|(u, p)| (u.clone(), p)) {
                    if let Some(t) = navegar_mut(doc.as_item_mut(), p2)?.as_table_like_mut() {
                        t.remove(&k);
                    }
                }
            }
            Ok(())
        }
        Seg::Clave(k) => {
            let Some(t) = item.as_table_like_mut() else {
                return err(format!("{} no es una tabla", mostrar(padre)));
            };
            if t.remove(k).is_none() {
                return err(format!("no existe {}", mostrar(ruta)));
            }
            Ok(())
        }
    }
}

fn anadir(
    doc: &mut DocumentMut,
    lista: &str,
    campos: &[(String, Valor)],
) -> Result<(), ErrorEdicion> {
    if lista == "flags" {
        let [(nombre, Valor::Texto(n)), resto @ ..] = campos else {
            return err("una bandera necesita `nombre` como primer campo");
        };
        if nombre != "nombre" {
            return err("una bandera necesita `nombre` como primer campo");
        }
        let mut fila = InlineTable::new();
        for (k, v) in resto {
            fila.insert(k, valor_toml(v, None));
        }
        let t = doc
            .entry("flags")
            .or_insert_with(|| Item::Table(Table::new()))
            .as_table_mut()
            .ok_or_else(|| ErrorEdicion("`flags` no es una tabla".into()))?;
        if t.contains_key(n) {
            return err(format!("ya existe la bandera {n}"));
        }
        t.insert(n, Item::Value(Value::InlineTable(fila)));
        return Ok(());
    }
    if !LISTAS.contains(&lista) {
        return err(format!(
            "no se pueden anadir elementos a `{lista}` (solo {})",
            LISTAS.join(", ")
        ));
    }
    let mut nueva = Table::new();
    for (k, v) in campos {
        nueva.insert(k, Item::Value(valor_toml(v, None)));
    }
    let item = doc
        .entry(lista)
        .or_insert_with(|| Item::ArrayOfTables(ArrayOfTables::new()));
    let Some(l) = item.as_array_of_tables_mut() else {
        return err(format!("`{lista}` no es una lista de tablas"));
    };
    // Detras del ultimo elemento del mismo tipo, no al final del archivo.
    if let Some(p) = l.iter().filter_map(Table::position).max() {
        nueva.set_position(p + 1);
    }
    l.push(nueva);
    Ok(())
}

/// Un fallo de la descripcion situado en el texto, listo para senalarlo en el diagrama (RF-ARQ-041).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostico {
    pub linea: usize,
    pub columna: usize,
    pub codigo: String,
    pub mensaje: String,
}

fn linea_de(texto: &str, desplazamiento: usize) -> (usize, usize) {
    let d = desplazamiento.min(texto.len());
    let antes = &texto.as_bytes()[..d];
    let linea = antes.iter().filter(|b| **b == b'\n').count() + 1;
    let inicio = antes.iter().rposition(|b| *b == b'\n').map_or(0, |p| p + 1);
    (linea, d - inicio + 1)
}

/// Primera linea que contiene `aguja`, o la 1.
fn buscar_linea(texto: &str, aguja: &str) -> usize {
    texto
        .lines()
        .position(|l| l.contains(aguja))
        .map_or(1, |i| i + 1)
}

/// Pasa el texto por el cargador y devuelve todos sus fallos con linea. Vacio si es valido.
pub fn diagnosticos(
    texto: &str,
    carpeta: Option<&std::path::Path>,
    propia: Option<&std::path::Path>,
) -> Vec<Diagnostico> {
    use crate::error::ErrorArquitectura as E;
    let d = |linea, columna, codigo: &str, mensaje: String| Diagnostico {
        linea,
        columna,
        codigo: codigo.to_string(),
        mensaje,
    };
    match crate::carga::cargar_texto_en(texto, carpeta, propia) {
        Ok(_) => Vec::new(),
        Err(E::Validacion(fallos)) => fallos
            .into_iter()
            .map(|f| {
                let p = f
                    .posicion
                    .unwrap_or_else(|| crate::error::Posicion::nueva(1, 1));
                d(p.linea, p.columna, f.regla.codigo(), f.detalle)
            })
            .collect(),
        Err(E::Sintaxis(e)) => {
            let (l, c) = e.span().map_or((1, 1), |s| linea_de(texto, s.start));
            vec![d(l, c, "sintaxis", e.message().to_string())]
        }
        Err(E::Codificacion { mnemonico, causa }) => vec![d(
            buscar_linea(texto, &format!("\"{mnemonico}\"")),
            1,
            "codificacion",
            format!("codificacion invalida en {mnemonico}: {causa}"),
        )],
        Err(E::VersionFormato { pedida, soportada }) => vec![d(
            buscar_linea(texto, "formato"),
            1,
            "version",
            format!("la descripcion pide el formato {pedida}; se entiende {soportada}"),
        )],
        Err(E::Herencia(m)) => vec![d(buscar_linea(texto, "hereda"), 1, "herencia", m)],
        Err(otro) => vec![d(1, 1, "lectura", otro.to_string())],
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    const BASE: &str = "# Mi maquina\n\n[maquina]\nnombre = \"m\"   # el nombre\nformato = \"0.1\"\nbits_palabra = 8\nbits_direccion = 16\nendianness = \"little\"\n\n[flags]\nZ = { bit = 0, descripcion = \"cero\" }\n\n# registro A\n[[registro]]\nnombre = \"A\"\nbits = 8\nrol = \"general\"\ninicial = 0x00  # arranca a cero\n\n[[registro]]\nnombre = \"SP\"\nbits = 16\nrol = \"sp\"\ninicial = 0xEFFF\n\n[[region]]\nnombre = \"ram\"\ndesde = 0x0000\nhasta = 0xEFFF\nacceso = \"rw\"\n\n[[instruccion]]\nmnemonico = \"NOP\"\n";

    fn doc() -> Documento {
        Documento::analizar(BASE).unwrap()
    }

    #[test]
    fn sin_cambios_el_texto_es_identico() {
        assert_eq!(doc().texto(), BASE);
    }

    #[test]
    fn los_bloques_dicen_donde_estan() {
        let b = doc().bloques();
        let nombres: Vec<_> = b
            .iter()
            .map(|b| (b.clase.nombre(), b.nombre.as_str()))
            .collect();
        assert_eq!(
            nombres,
            [
                ("maquina", "m"),
                ("bandera", "Z"),
                ("registro", "A"),
                ("registro", "SP"),
                ("region", "ram"),
                ("instruccion", "NOP")
            ]
        );
        let a = &b[2];
        assert_eq!(BASE.lines().nth(a.linea - 1).unwrap(), "[[registro]]");
        assert_eq!(
            BASE.lines().nth(a.fin - 1).unwrap(),
            "inicial = 0x00  # arranca a cero"
        );
        assert_eq!(
            BASE.lines().nth(b[1].linea - 1).unwrap().chars().next(),
            Some('Z')
        );
        assert_eq!(a.ruta, [Seg::Clave("registro".into()), Seg::Indice(0)]);
    }

    #[test]
    fn fijar_conserva_comentarios_y_base() {
        let mut d = doc();
        d.aplicar(&Cambio::Fijar {
            ruta: vec![
                Seg::Clave("registro".into()),
                Seg::Indice(0),
                Seg::Clave("inicial".into()),
            ],
            valor: Valor::Entero(255),
        })
        .unwrap();
        let t = d.texto();
        assert!(t.contains("inicial = 0xFF  # arranca a cero"), "{t}");
        assert!(t.contains("# Mi maquina") && t.contains("# registro A"));
        d.aplicar(&Cambio::Fijar {
            ruta: vec![Seg::Clave("maquina".into()), Seg::Clave("nombre".into())],
            valor: Valor::Texto("otra".into()),
        })
        .unwrap();
        assert!(
            d.texto().contains("nombre = \"otra\"   # el nombre"),
            "{}",
            d.texto()
        );
    }

    #[test]
    fn anadir_registro_va_junto_a_los_demas() {
        let mut d = doc();
        d.aplicar(&Cambio::Anadir {
            lista: "registro".into(),
            campos: vec![
                ("nombre".into(), Valor::Texto("B".into())),
                ("bits".into(), Valor::Entero(8)),
                ("rol".into(), Valor::Texto("general".into())),
                ("inicial".into(), Valor::Entero(0)),
            ],
        })
        .unwrap();
        let t = d.texto();
        let pos_b = t.find("nombre = \"B\"").unwrap();
        assert!(t.find("nombre = \"SP\"").unwrap() < pos_b);
        assert!(pos_b < t.find("[[region]]").unwrap(), "{t}");
        // Y sigue siendo una descripcion que se puede releer.
        assert_eq!(Documento::analizar(&t).unwrap().bloques().len(), 7);
    }

    #[test]
    fn anadir_bandera_y_quitar() {
        let mut d = doc();
        d.aplicar(&Cambio::Anadir {
            lista: "flags".into(),
            campos: vec![
                ("nombre".into(), Valor::Texto("C".into())),
                ("bit".into(), Valor::Entero(1)),
                ("descripcion".into(), Valor::Texto("acarreo".into())),
            ],
        })
        .unwrap();
        assert!(
            d.texto()
                .contains("C = { bit = 1, descripcion = \"acarreo\" }"),
            "{}",
            d.texto()
        );
        d.aplicar(&Cambio::Quitar {
            ruta: vec![Seg::Clave("registro".into()), Seg::Indice(0)],
        })
        .unwrap();
        let t = d.texto();
        assert!(
            !t.contains("nombre = \"A\"") && t.contains("nombre = \"SP\""),
            "{t}"
        );
        d.aplicar(&Cambio::Quitar {
            ruta: vec![Seg::Clave("flags".into()), Seg::Clave("C".into())],
        })
        .unwrap();
        assert!(!d.texto().contains("acarreo"));
    }

    #[test]
    fn un_cambio_invalido_no_toca_el_documento() {
        let mut d = doc();
        assert!(d
            .aplicar(&Cambio::Quitar {
                ruta: vec![Seg::Clave("nada".into())]
            })
            .is_err());
        assert!(d
            .aplicar(&Cambio::Anadir {
                lista: "maquina".into(),
                campos: vec![]
            })
            .is_err());
        assert!(d
            .aplicar(&Cambio::Fijar {
                ruta: vec![
                    Seg::Clave("registro".into()),
                    Seg::Indice(9),
                    Seg::Clave("bits".into())
                ],
                valor: Valor::Entero(1)
            })
            .is_err());
        assert_eq!(d.texto(), BASE);
    }

    #[test]
    fn los_fallos_del_cargador_traen_linea() {
        let ok = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../ejemplos/ejemplo-8/maquina.toml"
        ))
        .unwrap();
        assert!(diagnosticos(&ok, None, None).is_empty());
        // Un registro con el mismo nombre que otro: se senala la segunda declaracion.
        let roto = ok.replacen("nombre = \"B\"", "nombre = \"A\"", 1);
        let ds = diagnosticos(&roto, None, None);
        let dup = ds
            .iter()
            .find(|d| d.codigo == "E-DUPLICADO")
            .expect("{ds:?}");
        let l = roto.lines().nth(dup.linea - 1).unwrap();
        assert!(l.contains("\"A\""), "{dup:?} -> {l}");
        // Sintaxis rota: linea de la rotura.
        let ds = diagnosticos("[maquina]\nnombre = \n", None, None);
        assert_eq!(ds[0].codigo, "sintaxis");
        assert!(ds[0].linea >= 2);
    }

    #[test]
    fn texto_que_no_es_toml_se_rechaza() {
        assert!(Documento::analizar("[maquina\nx = ").is_err());
    }
}
