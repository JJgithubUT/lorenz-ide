//! Exportar, compartir, derivar y galeria del IDE alojado (RF-ECO-010 a 013, RF-ECO-032).
//!
//! Lo compartido es un lote de archivos (una maquina, un programa y lo que citan) con un
//! identificador que sale de su contenido: el mismo contenido da siempre el mismo enlace, y nadie
//! puede cambiar lo que un enlace muestra. Un enlace materializa el lote en una carpeta de solo
//! lectura del visitante, donde se ejecuta; para editarlo se deriva una copia propia.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use lorenz_paq::{sha256_hex, Manifiesto, Paquete};
use lorenz_web::{Json, Peticion, Respuesta, Router};

use super::api::{cuerpo, error, ok, texto_de};
use super::seguridad::ruta_fuera_del_espacio;
use super::servicio::Servicio;
use super::sesion::{Ide, CARPETA_COMPARTIDA};

const MAX_ARCHIVOS_LOTE: usize = 24;
const MAX_BYTES_LOTE: usize = 256 * 1024;

/// Un lote compartido.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lote {
    pub maquina: String,
    pub programa: String,
    /// Ruta relativa al espacio y contenido, ordenadas por ruta.
    pub archivos: BTreeMap<String, String>,
}

impl Lote {
    fn a_json(&self) -> Json {
        Json::objeto([
            ("v", 1u64.into()),
            ("maquina", self.maquina.as_str().into()),
            ("programa", self.programa.as_str().into()),
            (
                "archivos",
                self.archivos
                    .iter()
                    .map(|(r, t)| {
                        Json::objeto([("ruta", r.as_str().into()), ("texto", t.as_str().into())])
                    })
                    .collect::<Vec<_>>()
                    .into(),
            ),
        ])
    }

    /// El identificador: 20 caracteres hexadecimales de la suma SHA-256 del lote serializado. El
    /// serializador ordena las claves, asi que el mismo contenido da siempre el mismo texto.
    pub fn id(&self) -> String {
        sha256_hex(self.a_json().serializar().as_bytes())[..20].to_string()
    }

    fn de_json(j: &Json) -> Option<Lote> {
        let archivos = j
            .get("archivos")?
            .como_lista()?
            .iter()
            .map(|a| {
                Some((
                    a.get("ruta")?.como_texto()?.to_string(),
                    a.get("texto")?.como_texto()?.to_string(),
                ))
            })
            .collect::<Option<BTreeMap<_, _>>>()?;
        Some(Lote {
            maquina: j.get("maquina")?.como_texto()?.to_string(),
            programa: j.get("programa")?.como_texto()?.to_string(),
            archivos,
        })
    }
}

pub fn id_valido(id: &str) -> bool {
    id.len() == 20
        && id
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

/// Una ruta del espacio, ya normalizada (`a/../b` pasa a `b`); `None` si sale o es rara.
fn normalizar(ruta: &str) -> Option<String> {
    let mut partes: Vec<&str> = Vec::new();
    for p in ruta.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                partes.pop()?;
            }
            otro => partes.push(otro),
        }
    }
    let r = partes.join("/");
    (!r.is_empty() && !r.contains(['\\', ':', '\0'])).then_some(r)
}

/// Archivos que un texto cita entre comillas junto a `hereda`, `incluir` o `importar`.
fn citados(texto: &str) -> Vec<String> {
    let mut v = Vec::new();
    for linea in texto.lines() {
        let l = linea.split([';', '#']).next().unwrap_or("");
        let b = l.to_ascii_lowercase();
        if [
            "hereda",
            "incluir",
            "include",
            "importar",
            "import",
            "contenido",
        ]
        .iter()
        .any(|p| b.contains(p))
        {
            v.extend(
                l.split('"')
                    .skip(1)
                    .step_by(2)
                    .map(str::to_string)
                    .filter(|c| !c.is_empty()),
            );
        }
    }
    v
}

/// Reune la maquina, el programa y lo que citan, sin salirse del espacio.
fn reunir(ide: &Ide, maquina: &str, programa: &str) -> Result<Lote, String> {
    let mut pendientes = vec![
        normalizar(maquina).ok_or("ruta de maquina invalida")?,
        normalizar(programa).ok_or("ruta de programa invalida")?,
    ];
    let (maquina, programa) = (pendientes[0].clone(), pendientes[1].clone());
    let mut archivos = BTreeMap::new();
    let mut total = 0;
    while let Some(r) = pendientes.pop() {
        if archivos.contains_key(&r) {
            continue;
        }
        let disco = ide.resolver(&r)?;
        let texto = std::fs::read_to_string(&disco)
            .map_err(|e| format!("`{r}`: no se puede compartir ({e}); solo archivos de texto"))?;
        if let Some(f) = ruta_fuera_del_espacio(&texto) {
            // Las citas con `..` dentro del espacio se resuelven mas abajo; las absolutas no.
            if !f.contains("sale del espacio") || citados(&texto).iter().any(|c| c.starts_with('/'))
            {
                return Err(format!("`{r}`: {f}"));
            }
        }
        total += texto.len();
        if archivos.len() >= MAX_ARCHIVOS_LOTE || total > MAX_BYTES_LOTE {
            return Err(format!(
                "el lote es demasiado grande (maximo {MAX_ARCHIVOS_LOTE} archivos y {} KiB)",
                MAX_BYTES_LOTE / 1024
            ));
        }
        let dir = r.rsplit_once('/').map_or("", |x| x.0).to_string();
        for c in citados(&texto) {
            let unido = if dir.is_empty() {
                c.clone()
            } else {
                format!("{dir}/{c}")
            };
            let n = normalizar(&unido)
                .ok_or_else(|| format!("`{r}` cita `{c}`, que sale del espacio de trabajo"))?;
            if n.starts_with(&format!("{CARPETA_COMPARTIDA}/")) {
                return Err(format!("`{r}` cita `{c}`, que ya es de otra persona"));
            }
            pendientes.push(n);
        }
        archivos.insert(r, texto);
    }
    Ok(Lote {
        maquina,
        programa,
        archivos,
    })
}

fn leer_lote(servicio: &Servicio, id: &str) -> Option<Lote> {
    if !id_valido(id) {
        return None;
    }
    let t = std::fs::read_to_string(servicio.compartidos().join(format!("{id}.json"))).ok()?;
    let l = Lote::de_json(&Json::analizar(&t).ok()?)?;
    // El identificador es el contenido: si el archivo no coincide, esta danado o manipulado.
    (l.id() == id).then_some(l)
}

/// Escribe el lote bajo `prefijo/` en el espacio del visitante.
fn materializar(ide: &Ide, lote: &Lote, prefijo: &str, sobrescribir: bool) -> Result<(), String> {
    for (r, t) in &lote.archivos {
        let rel = format!("{prefijo}/{r}");
        let destino = ide.resolver(&rel)?;
        if destino.exists() && !sobrescribir {
            return Err(format!("`{rel}` ya existe"));
        }
        if prefijo.starts_with(CARPETA_COMPARTIDA) {
            // Lo compartido lo escribe el servicio, no el visitante: no pasa por la cuota.
            if ruta_fuera_del_espacio(t).is_some() && !t.contains("..") {
                return Err(format!("`{r}` no es seguro"));
            }
        } else {
            ide.permitir_escritura(&rel, t)?;
        }
        if let Some(p) = destino.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        std::fs::write(&destino, t).map_err(|e| format!("`{rel}`: {e}"))?;
    }
    Ok(())
}

fn nombre_de_carpeta_valido(n: &str) -> bool {
    (1..=40).contains(&n.len())
        && n.bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_')
        && !n.starts_with(['-', '_'])
        && n != CARPETA_COMPARTIDA
}

// --- exportar a .lpkg ------------------------------------------------------------------------

fn recoger(base: &Path, dir: &Path, salida: &mut BTreeMap<String, Vec<u8>>, total: &mut usize) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entradas: Vec<_> = rd.filter_map(Result::ok).collect();
    entradas.sort_by_key(|e| e.file_name());
    for e in entradas {
        let nombre = e.file_name().to_string_lossy().into_owned();
        let Ok(t) = e.file_type() else { continue };
        if nombre.starts_with('.') || t.is_symlink() {
            continue;
        }
        if t.is_dir() {
            recoger(base, &e.path(), salida, total);
        } else if let Ok(datos) = std::fs::read(e.path()) {
            let Ok(rel) = e.path().strip_prefix(base).map(|r| {
                r.components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/")
            }) else {
                continue;
            };
            if lorenz_paq::paquete::ruta_valida(&rel).is_ok() && !rel.ends_with(".lpkg") {
                *total += datos.len();
                salida.insert(rel, datos);
            }
        }
    }
}

/// El espacio de trabajo como un `.lpkg` para abrirlo en el escritorio (RF-ECO-032).
pub fn exportar(ide: &Ide) -> Result<Vec<u8>, String> {
    let mut archivos = BTreeMap::new();
    recoger(ide.raiz(), ide.raiz(), &mut archivos, &mut 0);
    if !archivos.contains_key(lorenz_paq::paquete::NOMBRE_MANIFIESTO) {
        archivos.insert(
            lorenz_paq::paquete::NOMBRE_MANIFIESTO.to_string(),
            "[paquete]\nnombre = \"proyecto-web\"\nversion = \"0.1.0\"\nautor = \"visitante\"\nlicencia = \"sin-especificar\"\ntipo = \"proyecto\"\ndescripcion = \"Exportado desde Lorenz web\"\n"
                .as_bytes()
                .to_vec(),
        );
    }
    let texto = String::from_utf8_lossy(&archivos[lorenz_paq::paquete::NOMBRE_MANIFIESTO]);
    let manifiesto = Manifiesto::leer(&texto).map_err(|e| e.to_string())?;
    Ok(Paquete {
        manifiesto,
        archivos,
    }
    .a_bytes())
}

// --- galeria -----------------------------------------------------------------------------------

/// Minusculas y sin acentos, para buscar sin que importen.
fn plano(t: &str) -> String {
    t.chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' | 'Á' => 'a',
            'é' | 'è' | 'ë' | 'ê' | 'É' => 'e',
            'í' | 'ì' | 'ï' | 'î' | 'Í' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'Ó' => 'o',
            'ú' | 'ù' | 'ü' | 'û' | 'Ú' => 'u',
            'ñ' | 'Ñ' => 'n',
            otro => otro.to_ascii_lowercase(),
        })
        .collect()
}

struct Entrada {
    id: String,
    titulo: String,
    descripcion: String,
    etiquetas: Vec<String>,
    maquina: String,
    fecha: u64,
    autor: String,
}

impl Entrada {
    fn a_json(&self) -> Json {
        Json::objeto([
            ("id", self.id.as_str().into()),
            ("titulo", self.titulo.as_str().into()),
            ("descripcion", self.descripcion.as_str().into()),
            (
                "etiquetas",
                self.etiquetas
                    .iter()
                    .map(|e| Json::from(e.as_str()))
                    .collect::<Vec<_>>()
                    .into(),
            ),
            ("maquina", self.maquina.as_str().into()),
            ("fecha", self.fecha.into()),
            ("autor", self.autor.as_str().into()),
        ])
    }

    fn de_json(j: &Json) -> Option<Self> {
        Some(Self {
            id: j.get("id")?.como_texto()?.to_string(),
            titulo: j.get("titulo")?.como_texto()?.to_string(),
            descripcion: j.get("descripcion")?.como_texto()?.to_string(),
            etiquetas: j
                .get("etiquetas")?
                .como_lista()?
                .iter()
                .filter_map(|e| e.como_texto().map(str::to_string))
                .collect(),
            maquina: j.get("maquina")?.como_texto()?.to_string(),
            fecha: j.get("fecha")?.como_entero()? as u64,
            autor: j.get("autor")?.como_texto()?.to_string(),
        })
    }

    /// Cuantos puntos suma la busqueda; 0 si alguna palabra no aparece.
    fn puntos(&self, palabras: &[String], etiqueta: Option<&str>) -> u32 {
        if let Some(e) = etiqueta {
            if !self.etiquetas.iter().any(|x| plano(x) == plano(e)) {
                return 0;
            }
        }
        let (t, d, m) = (
            plano(&self.titulo),
            plano(&self.descripcion),
            plano(&self.maquina),
        );
        let e: Vec<String> = self.etiquetas.iter().map(|x| plano(x)).collect();
        let mut total = 1;
        for p in palabras {
            let mut s = 0;
            if t.contains(p.as_str()) {
                s += 5;
            }
            if e.iter().any(|x| x.contains(p.as_str())) {
                s += 4;
            }
            if m.contains(p.as_str()) {
                s += 2;
            }
            if d.contains(p.as_str()) {
                s += 1;
            }
            if s == 0 {
                return 0;
            }
            total += s;
        }
        total
    }
}

fn entradas(servicio: &Servicio) -> Vec<Entrada> {
    let Ok(rd) = std::fs::read_dir(servicio.galeria()) else {
        return Vec::new();
    };
    rd.filter_map(Result::ok)
        .filter_map(|e| {
            let t = std::fs::read_to_string(e.path()).ok()?;
            Entrada::de_json(&Json::analizar(&t).ok()?)
        })
        .collect()
}

fn nombre_de_maquina(lote: &Lote) -> String {
    lote.archivos
        .get(&lote.maquina)
        .and_then(|t| {
            t.lines().find_map(|l| {
                let (k, v) = l.split_once('=')?;
                (k.trim() == "nombre").then(|| v.trim().trim_matches('"').to_string())
            })
        })
        .unwrap_or_default()
}

fn ahora() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Un texto sin saltos raros ni espacios de mas, o `None` si no cabe en `max` caracteres.
fn limpio(t: &str, min: usize, max: usize) -> Option<String> {
    let s: String = t.split_whitespace().collect::<Vec<_>>().join(" ");
    let n = s.chars().count();
    (min..=max).contains(&n).then_some(s)
}

// --- rutas --------------------------------------------------------------------------------------

fn lote_de(p: &Peticion, s: &Servicio) -> Result<Lote, Respuesta> {
    let id = p.parametro("id").unwrap_or("");
    leer_lote(s, id).ok_or_else(|| error(404, "no existe lo compartido con ese enlace"))
}

pub fn rutas(router: Router, servicio: Arc<Servicio>, ide: Arc<Ide>) -> Router {
    let (s1, s2, s3, s4, s5, s6, s7, s8) = (
        servicio.clone(),
        servicio.clone(),
        servicio.clone(),
        servicio.clone(),
        servicio.clone(),
        servicio.clone(),
        servicio.clone(),
        servicio,
    );
    let (i1, i2, i3, i4) = (ide.clone(), ide.clone(), ide.clone(), ide);
    router
        .get("/api/exportar", move |_| match exportar(&i1) {
            Ok(bytes) => Respuesta::nueva(200, "application/octet-stream", bytes).con_cabecera(
                "Content-Disposition",
                "attachment; filename=\"proyecto.lpkg\"",
            ),
            Err(e) => error(500, e),
        })
        .post("/api/compartir", move |p| {
            let j = match cuerpo(p) {
                Ok(j) => j,
                Err(r) => return r,
            };
            let (m, pr) = match (texto_de(&j, "maquina"), texto_de(&j, "programa")) {
                (Ok(m), Ok(pr)) => (m, pr),
                (Err(r), _) | (_, Err(r)) => return r,
            };
            let lote = match reunir(&i2, m, pr) {
                Ok(l) => l,
                Err(e) => return error(422, e),
            };
            // La maquina tiene que cargar: no se comparte algo que no funciona.
            let id = lote.id();
            let Ok(_g) = s1.comun.lock() else {
                return error(500, "estado corrupto");
            };
            let destino = s1.compartidos().join(format!("{id}.json"));
            if !destino.exists() {
                if let Err(e) = std::fs::write(&destino, lote.a_json().serializar()) {
                    return error(500, format!("no se pudo guardar: {e}"));
                }
            }
            ok(Json::objeto([
                ("id", id.as_str().into()),
                ("enlace", format!("/c/{id}").into()),
                ("archivos", (lote.archivos.len() as u64).into()),
                ("maquina", lote.maquina.as_str().into()),
                ("programa", lote.programa.as_str().into()),
            ]))
        })
        .get("/c/:id", move |p| {
            let lote = match lote_de(p, &s2) {
                Ok(l) => l,
                Err(r) => return r,
            };
            let id = lote.id();
            let prefijo = format!("{CARPETA_COMPARTIDA}/{id}");
            if !i3.resolver(&prefijo).is_ok_and(|d| d.exists()) {
                if let Err(e) = materializar(&i3, &lote, &prefijo, true) {
                    return error(500, e);
                }
            }
            let destino = format!(
                "/?abrir={prefijo}/{}&programa={prefijo}/{}",
                lote.maquina, lote.programa
            );
            Respuesta::vacia(302).con_cabecera("Location", &destino)
        })
        .get("/api/compartido/:id", move |p| match lote_de(p, &s3) {
            Ok(l) => ok(Json::objeto([
                ("id", l.id().into()),
                ("maquina", l.maquina.as_str().into()),
                ("programa", l.programa.as_str().into()),
                (
                    "archivos",
                    l.archivos
                        .keys()
                        .map(|k| Json::from(k.as_str()))
                        .collect::<Vec<_>>()
                        .into(),
                ),
            ])),
            Err(r) => r,
        })
        .post("/api/derivar", move |p| {
            let j = match cuerpo(p) {
                Ok(j) => j,
                Err(r) => return r,
            };
            let (id, nombre) = match (texto_de(&j, "id"), texto_de(&j, "nombre")) {
                (Ok(i), Ok(n)) => (i, n),
                (Err(r), _) | (_, Err(r)) => return r,
            };
            if !nombre_de_carpeta_valido(nombre) {
                return error(
                    400,
                    "el nombre de la copia usa minusculas, digitos, `-` y `_` (hasta 40)",
                );
            }
            let Some(lote) = leer_lote(&s4, id) else {
                return error(404, "no existe lo compartido con ese enlace");
            };
            if let Err(e) = materializar(&i4, &lote, nombre, false) {
                return error(422, e);
            }
            ok(Json::objeto([
                ("carpeta", nombre.into()),
                ("maquina", format!("{nombre}/{}", lote.maquina).into()),
                ("programa", format!("{nombre}/{}", lote.programa).into()),
            ]))
        })
        .post("/api/galeria", move |p| {
            let j = match cuerpo(p) {
                Ok(j) => j,
                Err(r) => return r,
            };
            let id = match texto_de(&j, "id") {
                Ok(i) => i,
                Err(r) => return r,
            };
            let Some(lote) = leer_lote(&s5, id) else {
                return error(404, "comparte primero la maquina para obtener su enlace");
            };
            let titulo = j
                .get("titulo")
                .and_then(Json::como_texto)
                .and_then(|t| limpio(t, 3, 80));
            let Some(titulo) = titulo else {
                return error(400, "el titulo debe tener de 3 a 80 caracteres");
            };
            let descripcion = j
                .get("descripcion")
                .and_then(Json::como_texto)
                .map_or(Some(String::new()), |t| limpio(t, 0, 500));
            let Some(descripcion) = descripcion else {
                return error(400, "la descripcion no puede pasar de 500 caracteres");
            };
            let mut etiquetas = Vec::new();
            for e in j.get("etiquetas").and_then(Json::como_lista).unwrap_or(&[]) {
                match e.como_texto().and_then(|t| limpio(t, 1, 24)) {
                    Some(t) => etiquetas.push(t),
                    None => return error(400, "cada etiqueta debe tener de 1 a 24 caracteres"),
                }
            }
            if etiquetas.len() > 8 {
                return error(400, "como maximo 8 etiquetas");
            }
            let autor = p.galleta("lorenz").map_or(String::new(), |t| {
                sha256_hex(t.as_bytes())[..16].to_string()
            });
            let Ok(_g) = s5.comun.lock() else {
                return error(500, "estado corrupto");
            };
            let propias = entradas(&s5).iter().filter(|e| e.autor == autor).count();
            if propias >= 20 {
                return error(
                    429,
                    "ya publicaste 20 maquinas; retira alguna antes de publicar otra",
                );
            }
            let e = Entrada {
                id: id.to_string(),
                titulo,
                descripcion,
                etiquetas,
                maquina: nombre_de_maquina(&lote),
                fecha: ahora(),
                autor,
            };
            if let Err(er) = std::fs::write(
                s5.galeria().join(format!("{id}.json")),
                e.a_json().serializar(),
            ) {
                return error(500, format!("no se pudo publicar: {er}"));
            }
            ok(e.a_json())
        })
        .get("/api/galeria", move |p| {
            let palabras: Vec<String> = p
                .consulta("q")
                .unwrap_or("")
                .split_whitespace()
                .map(plano)
                .collect();
            let etiqueta = p.consulta("etiqueta");
            let mi_autor = p.galleta("lorenz").map_or(String::new(), |t| {
                sha256_hex(t.as_bytes())[..16].to_string()
            });
            let mut hallados: Vec<(u32, Entrada)> = entradas(&s6)
                .into_iter()
                .filter_map(|e| {
                    let n = e.puntos(&palabras, etiqueta);
                    (n > 0).then_some((n, e))
                })
                .collect();
            hallados.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.fecha.cmp(&a.1.fecha)));
            hallados.truncate(50);
            ok(Json::objeto([(
                "resultados",
                hallados
                    .iter()
                    .map(|(_, e)| {
                        let mut j = e.a_json();
                        if let Json::Objeto(m) = &mut j {
                            m.remove("autor");
                            m.insert("mia".into(), (e.autor == mi_autor).into());
                            m.insert("enlace".into(), format!("/c/{}", e.id).into());
                        }
                        j
                    })
                    .collect::<Vec<_>>()
                    .into(),
            )]))
        })
        .delete("/api/galeria/:id", move |p| {
            let id = p.parametro("id").unwrap_or("");
            if !id_valido(id) {
                return error(400, "enlace invalido");
            }
            let autor = p.galleta("lorenz").map_or(String::new(), |t| {
                sha256_hex(t.as_bytes())[..16].to_string()
            });
            let Ok(_g) = s7.comun.lock() else {
                return error(500, "estado corrupto");
            };
            let ruta = s7.galeria().join(format!("{id}.json"));
            let Some(e) = std::fs::read_to_string(&ruta)
                .ok()
                .and_then(|t| Json::analizar(&t).ok())
                .and_then(|j| Entrada::de_json(&j))
            else {
                return error(404, "no esta en la galeria");
            };
            if e.autor != autor {
                return error(403, "solo quien publico una maquina puede retirarla");
            }
            match std::fs::remove_file(ruta) {
                Ok(()) => ok(Json::objeto([("id", id.into())])),
                Err(er) => error(500, er.to_string()),
            }
        })
        .get("/api/galeria/etiquetas", move |_| {
            // Las etiquetas en uso, para filtrar sin escribir.
            let mut cuenta: BTreeMap<String, u64> = BTreeMap::new();
            for e in entradas(&s8) {
                for t in e.etiquetas {
                    *cuenta.entry(plano(&t)).or_insert(0) += 1;
                }
            }
            ok(Json::objeto([(
                "etiquetas",
                cuenta
                    .into_iter()
                    .map(|(k, n)| Json::objeto([("nombre", k.into()), ("n", n.into())]))
                    .collect::<Vec<_>>()
                    .into(),
            )]))
        })
}
