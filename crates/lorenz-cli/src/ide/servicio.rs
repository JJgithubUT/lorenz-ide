//! `lorenz servir`: el IDE alojado, para usarlo desde el navegador sin instalar nada
//! (RF-ECO-030 a 033).
//!
//! Cada visitante recibe una galleta con un identificador al azar y un espacio de trabajo propio en
//! disco, con unos ejemplos. El espacio sobrevive al cierre del navegador y a los reinicios del
//! servidor (RF-ECO-031). Todo lo demas es el mismo IDE: la diferencia son los limites (cuotas,
//! rutas, dispositivos externos) y lo que no esta disponible, que la interfaz explica
//! (RF-ECO-033).

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use lorenz_web::{Json, Manejador, Peticion, Respuesta, Router};

use super::sesion::{Ide, Restricciones};

/// Lo que el IDE alojado no ofrece frente al escritorio: funcion y motivo (RF-ECO-033).
pub const LIMITACIONES: &[(&str, &str)] = &[
    (
        "Dispositivos externos",
        "no se lanzan procesos ni se abren conexiones desde el servidor",
    ),
    (
        "Archivos fuera del espacio",
        "las rutas absolutas y `..` en herencias, inclusiones e importaciones no se admiten",
    ),
    (
        "Cuota de espacio",
        "cada visitante tiene un maximo de archivos y de bytes",
    ),
    (
        "Terminal y orden `lorenz`",
        "solo esta la interfaz; para la linea de comandos instala Lorenz en tu equipo",
    ),
    (
        "Carpeta del proyecto en tu disco",
        "el trabajo vive en el servidor: exportalo a `.lpkg` para abrirlo en el escritorio",
    ),
    (
        "Instalar paquetes de la red",
        "`lorenz paquete instalar` se hace en el escritorio",
    ),
    (
        "Editar lo compartido",
        "lo que otros comparten se ve y se ejecuta, pero se edita en una copia propia",
    ),
];

const EJEMPLOS: &[(&str, &str)] = &[
    (
        "ejemplo-8/maquina.toml",
        include_str!("../../../../ejemplos/ejemplo-8/maquina.toml"),
    ),
    (
        "ejemplo-8/programas/suma.s",
        include_str!("../../../../ejemplos/ejemplo-8/programas/suma.s"),
    ),
    (
        "ejemplo-8/programas/bucle.s",
        include_str!("../../../../ejemplos/ejemplo-8/programas/bucle.s"),
    ),
    (
        "risc-16/maquina.toml",
        include_str!("../../../../ejemplos/risc-16/maquina.toml"),
    ),
    (
        "risc-16/programas/suma-1-a-10.s",
        include_str!("../../../../ejemplos/risc-16/programas/suma-1-a-10.s"),
    ),
    (
        "risc-16/programas/comparar.s",
        include_str!("../../../../ejemplos/risc-16/programas/comparar.s"),
    ),
];

#[derive(Debug, Clone)]
pub struct OpcionesServicio {
    /// Donde viven los espacios, lo compartido y la galeria.
    pub datos: PathBuf,
    /// Espacios en memoria a la vez; los menos usados se sueltan (su disco no se toca).
    pub max_visitantes: usize,
    pub restricciones: Restricciones,
    /// Peticiones por minuto y visitante.
    pub peticiones_por_minuto: u32,
    /// Un espacio sin usar mas de estos dias se borra al arrancar.
    pub caducidad_dias: u64,
}

impl OpcionesServicio {
    pub fn en(datos: impl Into<PathBuf>) -> Self {
        Self {
            datos: datos.into(),
            max_visitantes: 256,
            restricciones: Restricciones::default(),
            peticiones_por_minuto: 900,
            caducidad_dias: 90,
        }
    }
}

struct Visitante {
    ide: Arc<Ide>,
    router: Arc<Router>,
    visto: Instant,
    ventana: Instant,
    cuenta: u32,
}

pub struct Servicio {
    pub opciones: OpcionesServicio,
    visitantes: Mutex<HashMap<String, Visitante>>,
    /// Serializa las escrituras en lo compartido y la galeria.
    pub(super) comun: Mutex<()>,
}

static CONTADOR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 128 bits al azar en hexadecimal. Usa el generador del sistema; si no hay, mezcla lo que tenga.
fn token_nuevo() -> String {
    let mut b = [0u8; 16];
    if std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut b))
        .is_ok()
    {
        return lorenz_paq::sha256::hex(&b);
    }
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u64(CONTADOR.fetch_add(1, std::sync::atomic::Ordering::SeqCst));
    h.write_u128(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos()),
    );
    h.write_u32(std::process::id());
    let mut crudo = h.finish().to_le_bytes().to_vec();
    crudo.extend(std::process::id().to_le_bytes());
    lorenz_paq::sha256::sha256_hex(&crudo)[..32].to_string()
}

pub fn token_valido(t: &str) -> bool {
    t.len() == 32
        && t.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

impl Servicio {
    pub fn nuevo(opciones: OpcionesServicio) -> std::io::Result<Arc<Self>> {
        for d in ["visitantes", "compartidos", "galeria"] {
            std::fs::create_dir_all(opciones.datos.join(d))?;
        }
        let s = Arc::new(Self {
            opciones,
            visitantes: Mutex::new(HashMap::new()),
            comun: Mutex::new(()),
        });
        s.limpiar();
        Ok(s)
    }

    pub fn carpeta_de(&self, token: &str) -> PathBuf {
        self.opciones.datos.join("visitantes").join(token)
    }

    pub fn compartidos(&self) -> PathBuf {
        self.opciones.datos.join("compartidos")
    }

    pub fn galeria(&self) -> PathBuf {
        self.opciones.datos.join("galeria")
    }

    /// Borra los espacios que nadie toca desde hace `caducidad_dias`.
    fn limpiar(&self) {
        let limite = Duration::from_secs(self.opciones.caducidad_dias * 86_400);
        let Ok(rd) = std::fs::read_dir(self.opciones.datos.join("visitantes")) else {
            return;
        };
        for e in rd.filter_map(Result::ok) {
            let marca = e.path().join(".visto");
            let antiguo = std::fs::metadata(&marca)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| SystemTime::now().duration_since(t).ok())
                .is_some_and(|d| d > limite);
            if antiguo && token_valido(&e.file_name().to_string_lossy()) {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
    }

    /// Crea el espacio en disco de un visitante nuevo, con los ejemplos.
    fn sembrar(&self, token: &str) -> std::io::Result<()> {
        let dir = self.carpeta_de(token);
        for (rel, texto) in EJEMPLOS {
            let destino = dir.join(rel);
            if let Some(p) = destino.parent() {
                std::fs::create_dir_all(p)?;
            }
            std::fs::write(destino, texto)?;
        }
        std::fs::write(dir.join(".visto"), b"")
    }

    fn visitante(self: &Arc<Self>, token: &str) -> Result<(Arc<Ide>, Arc<Router>, bool), String> {
        let mut todos = self.visitantes.lock().map_err(|_| "estado corrupto")?;
        let ahora = Instant::now();
        if let Some(v) = todos.get_mut(token) {
            v.visto = ahora;
            if ahora.duration_since(v.ventana) >= Duration::from_secs(60) {
                v.ventana = ahora;
                v.cuenta = 0;
            }
            v.cuenta += 1;
            if v.cuenta > self.opciones.peticiones_por_minuto {
                return Err("demasiadas peticiones; espera un momento".into());
            }
            return Ok((v.ide.clone(), v.router.clone(), false));
        }
        let dir = self.carpeta_de(token);
        let nuevo = !dir.exists();
        if nuevo {
            self.sembrar(token)
                .map_err(|e| format!("no se pudo crear el espacio: {e}"))?;
        } else {
            let _ = std::fs::write(dir.join(".visto"), b"");
        }
        if todos.len() >= self.opciones.max_visitantes {
            if let Some(viejo) = todos
                .iter()
                .min_by_key(|(_, v)| v.visto)
                .map(|(k, _)| k.clone())
            {
                todos.remove(&viejo);
            }
        }
        let ide = Arc::new(Ide::alojado(&dir, self.opciones.restricciones.clone()));
        let router = Arc::new(super::compartir::rutas(
            super::router(ide.clone()),
            self.clone(),
            ide.clone(),
        ));
        todos.insert(
            token.to_string(),
            Visitante {
                ide: ide.clone(),
                router: router.clone(),
                visto: ahora,
                ventana: ahora,
                cuenta: 1,
            },
        );
        Ok((ide, router, nuevo))
    }

    /// El manejador que usa `Servidor::iniciar_con`.
    pub fn manejador(self: &Arc<Self>) -> Manejador {
        let s = self.clone();
        Arc::new(move |p| s.atender(p))
    }

    fn atender(self: &Arc<Self>, p: &Peticion) -> Respuesta {
        // Una peticion que cambia cosas solo vale si viene de la propia pagina.
        if p.metodo != "GET" && p.metodo != "HEAD" {
            if let (Some(origen), Some(host)) = (p.cabecera("origin"), p.cabecera("host")) {
                let o = origen.split_once("://").map_or(origen, |x| x.1);
                if o != host {
                    return Respuesta::texto(403, "origen no permitido");
                }
            }
        }
        let token_galleta = p.galleta("lorenz").filter(|t| token_valido(t));
        let token = token_galleta.map_or_else(token_nuevo, str::to_string);
        let (_, router, nuevo) = match self.visitante(&token) {
            Ok(x) => x,
            Err(e) if e.starts_with("demasiadas") => return Respuesta::texto(429, &e),
            Err(e) => return Respuesta::texto(500, &e),
        };
        let r = router.despachar(p);
        if token_galleta.is_none() || nuevo {
            r.con_cabecera(
                "Set-Cookie",
                &format!(
                    "lorenz={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
                    self.opciones.caducidad_dias * 86_400
                ),
            )
        } else {
            r
        }
    }
}

/// `GET /api/modo`: si el IDE esta alojado y que no ofrece frente al escritorio (RF-ECO-033).
pub fn modo(ide: &Ide) -> Json {
    let limitaciones: Vec<Json> = if ide.es_alojado() {
        LIMITACIONES
            .iter()
            .map(|(f, m)| Json::objeto([("funcion", (*f).into()), ("motivo", (*m).into())]))
            .collect()
    } else {
        Vec::new()
    };
    let cuota = ide.restricciones().map_or(Json::Nulo, |r| {
        Json::objeto([
            ("archivos", (r.max_archivos as u64).into()),
            ("bytes", r.max_bytes.into()),
            ("bytes_por_archivo", (r.max_archivo as u64).into()),
        ])
    });
    Json::objeto([
        ("alojado", ide.es_alojado().into()),
        ("limitaciones", limitaciones.into()),
        ("cuota", cuota),
    ])
}

/// Para `lorenz servir`: arranca el servicio y espera.
pub fn servir_cli(
    direccion: &str,
    datos: &str,
    segundos: Option<u64>,
    salida: &mut dyn std::io::Write,
    errores: &mut dyn std::io::Write,
) -> i32 {
    use crate::ordenes::{SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_OK};
    let servicio = match Servicio::nuevo(OpcionesServicio::en(datos)) {
        Ok(s) => s,
        Err(e) => {
            let _ = writeln!(
                errores,
                "{datos}: no se pudo preparar la carpeta de datos: {e}"
            );
            return SALIDA_ERROR_USO;
        }
    };
    let local = direccion
        .rsplit_once(':')
        .map_or(direccion, |x| x.0)
        .trim_matches(['[', ']'])
        .to_string();
    let loopback = matches!(local.as_str(), "127.0.0.1" | "localhost" | "::1");
    let limites = lorenz_web::Limites {
        solo_locales: loopback,
        ..lorenz_web::Limites::default()
    };
    let servidor = match lorenz_web::Servidor::iniciar_con(direccion, servicio.manejador(), limites)
    {
        Ok(s) => s,
        Err(e) => {
            let _ = writeln!(errores, "no se pudo abrir {direccion}: {e}");
            return SALIDA_ERROR_EJECUCION;
        }
    };
    let _ = writeln!(
        salida,
        "Lorenz alojado en http://{} (datos en {}); Ctrl-C para salir",
        servidor.direccion(),
        Path::new(datos).display()
    );
    if !loopback {
        let _ = writeln!(
            salida,
            "aviso: escucha fuera de este equipo; ponlo detras de HTTPS y en un entorno aislado (docs/WEB.md)"
        );
    }
    let _ = salida.flush();
    match segundos {
        Some(s) => std::thread::sleep(Duration::from_secs(s)),
        None => loop {
            std::thread::sleep(Duration::from_secs(3600));
        },
    }
    servidor.detener();
    SALIDA_OK
}
