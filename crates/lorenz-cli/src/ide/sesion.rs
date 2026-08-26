//! Estado del IDE: la carpeta del proyecto y las sesiones de ejecucion abiertas.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use lorenz_dbg::Depurador;
use lorenz_emu::Emulador;

impl std::ops::Deref for SesionIde {
    type Target = lorenz_dbg::comandos::Sesion;
    fn deref(&self) -> &Self::Target {
        &self.sesion
    }
}

impl std::ops::DerefMut for SesionIde {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sesion
    }
}

/// Archivos y bytes que ocupa una carpeta.
fn uso(dir: &Path) -> (usize, u64) {
    let mut total = (0usize, 0u64);
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.filter_map(Result::ok) {
            let Ok(t) = e.file_type() else { continue };
            if t.is_symlink() || e.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if t.is_dir() {
                let (a, b) = uso(&e.path());
                total = (total.0 + a, total.1 + b);
            } else if let Ok(m) = e.metadata() {
                total = (total.0 + 1, total.1 + m.len());
            }
        }
    }
    total
}

pub const MAX_SESIONES: usize = 16;

/// Una ejecucion en curso, con su depurador.
pub struct SesionIde {
    pub sesion: lorenz_dbg::comandos::Sesion,
    pub maquina: String,
    pub programa: String,
}

/// Limites de un espacio de trabajo alojado en un servidor (`lorenz servir`).
#[derive(Debug, Clone)]
pub struct Restricciones {
    /// Archivos como maximo en el espacio.
    pub max_archivos: usize,
    /// Bytes como maximo entre todos los archivos.
    pub max_bytes: u64,
    /// Bytes como maximo de un archivo.
    pub max_archivo: usize,
}

impl Default for Restricciones {
    fn default() -> Self {
        Self {
            max_archivos: 300,
            max_bytes: 4 * 1024 * 1024,
            max_archivo: 512 * 1024,
        }
    }
}

/// Carpeta de lo compartido por otros: se lee y se ejecuta, pero no se edita (se deriva una copia).
pub const CARPETA_COMPARTIDA: &str = "compartido";

pub struct Ide {
    raiz: PathBuf,
    restricciones: Option<Restricciones>,
    sesiones: Mutex<BTreeMap<u32, Arc<Mutex<SesionIde>>>>,
    siguiente: Mutex<u32>,
}

impl Ide {
    pub fn nuevo(raiz: impl Into<PathBuf>) -> Self {
        Self {
            raiz: raiz.into(),
            restricciones: None,
            sesiones: Mutex::new(BTreeMap::new()),
            siguiente: Mutex::new(1),
        }
    }

    /// Un espacio de trabajo en un servidor: con cuotas, sin dispositivos externos y sin rutas que
    /// salgan de la carpeta.
    pub fn alojado(raiz: impl Into<PathBuf>, restricciones: Restricciones) -> Self {
        let mut ide = Self::nuevo(raiz);
        ide.restricciones = Some(restricciones);
        ide
    }

    pub fn es_alojado(&self) -> bool {
        self.restricciones.is_some()
    }

    pub fn restricciones(&self) -> Option<&Restricciones> {
        self.restricciones.as_ref()
    }

    /// Comprueba que se pueda escribir `texto` en `rel`. Sin restricciones, siempre se puede.
    pub fn permitir_escritura(&self, rel: &str, texto: &str) -> Result<(), String> {
        let Some(r) = &self.restricciones else {
            return Ok(());
        };
        let limpia = rel.trim_start_matches("./");
        if limpia == CARPETA_COMPARTIDA || limpia.starts_with(&format!("{CARPETA_COMPARTIDA}/")) {
            return Err(format!(
                "`{CARPETA_COMPARTIDA}/` es de solo lectura: deriva una copia propia para editarla"
            ));
        }
        if texto.len() > r.max_archivo {
            return Err(format!(
                "`{rel}` pesa {} bytes y el maximo por archivo es {}",
                texto.len(),
                r.max_archivo
            ));
        }
        if let Some(fallo) = crate::ide::seguridad::ruta_fuera_del_espacio(texto) {
            return Err(format!("`{rel}`: {fallo}"));
        }
        let destino = self.resolver(rel)?;
        let (archivos, bytes) = uso(&self.raiz);
        let (era_archivo, era_bytes) = std::fs::metadata(&destino)
            .map(|m| (m.is_file(), m.len()))
            .unwrap_or((false, 0));
        let archivos = archivos + usize::from(!era_archivo);
        let bytes = bytes - era_bytes + texto.len() as u64;
        if archivos > r.max_archivos {
            return Err(format!(
                "el espacio ya tiene el maximo de {} archivos",
                r.max_archivos
            ));
        }
        if bytes > r.max_bytes {
            return Err(format!(
                "el espacio superaria {} bytes en total (cuota {})",
                bytes, r.max_bytes
            ));
        }
        Ok(())
    }

    /// Comprueba que se pueda borrar `rel`.
    pub fn permitir_borrado(&self, rel: &str) -> Result<(), String> {
        if self.restricciones.is_some() {
            let limpia = rel.trim_start_matches("./");
            if limpia.starts_with(&format!("{CARPETA_COMPARTIDA}/")) {
                return Err(format!("`{CARPETA_COMPARTIDA}/` es de solo lectura"));
            }
        }
        Ok(())
    }

    /// Una maquina que se va a ejecutar: en un servidor no se admiten dispositivos que lancen
    /// procesos o abran conexiones.
    pub fn permitir_maquina(&self, m: &lorenz_arch::Maquina) -> Result<(), String> {
        if self.restricciones.is_some()
            && m.dispositivos
                .iter()
                .any(|d| matches!(d.tipo, lorenz_arch::TipoDispositivo::Externo { .. }))
        {
            return Err("los dispositivos externos no estan disponibles en la version web".into());
        }
        Ok(())
    }

    pub fn raiz(&self) -> &Path {
        &self.raiz
    }

    /// Resuelve una ruta del proyecto; rechaza lo que salga de la carpeta, tambien por enlaces.
    pub fn resolver(&self, rel: &str) -> Result<PathBuf, String> {
        crate::proyecto::ruta_interna(rel)?;
        let ruta = self.raiz.join(rel);
        let base = self
            .raiz
            .canonicalize()
            .map_err(|e| format!("la carpeta del proyecto no existe: {e}"))?;
        // Se comprueba el ancestro existente mas cercano (el archivo puede no existir aun).
        let mut a = ruta.as_path();
        let existente = loop {
            if a.exists() {
                break a.canonicalize().map_err(|e| e.to_string())?;
            }
            a = a.parent().ok_or("ruta invalida")?;
        };
        if existente.starts_with(&base) {
            Ok(ruta)
        } else {
            Err(format!("`{rel}` sale del proyecto"))
        }
    }

    pub fn ruta_texto(&self, rel: &str) -> Result<String, String> {
        Ok(self.resolver(rel)?.to_string_lossy().into_owned())
    }

    /// Abre una sesion: ensambla el programa, lo carga y deja el PC en el arranque.
    pub fn abrir(&self, maquina_rel: &str, programa_rel: &str) -> Result<u32, String> {
        if self.sesiones.lock().map_err(|_| "estado corrupto")?.len() >= MAX_SESIONES {
            return Err(format!(
                "hay demasiadas sesiones abiertas (maximo {MAX_SESIONES}); cierra alguna"
            ));
        }
        let rm = self.ruta_texto(maquina_rel)?;
        let rp = self.ruta_texto(programa_rel)?;
        let maquina =
            lorenz_arch::cargar_archivo(&rm).map_err(|e| format!("{maquina_rel}: {e}"))?;
        let inicio = maquina.arranque;
        let mut objeto = None;
        let segmentos = if crate::ensamblar::es_fuente(&rp) {
            let (o, fuentes) = crate::ensamblar::ensamblar_ejecutable(&maquina, &rp)?;
            let s = o
                .segmentos
                .iter()
                .map(|s| (s.direccion, s.palabras.clone()))
                .collect::<Vec<_>>();
            objeto = Some((o, fuentes));
            s
        } else {
            crate::ensamblar::cargar_segmentos(&maquina, &rp, inicio)?
        };
        self.permitir_maquina(&maquina)?;
        let mut emulador = Emulador::nuevo(maquina).map_err(|e| format!("{maquina_rel}: {e}"))?;
        lorenz_emu::programa::precargar_regiones(&mut emulador, &crate::ordenes::carpeta_de(&rm))
            .map_err(|e| format!("{maquina_rel}: {e}"))?;
        for (desde, palabras) in &segmentos {
            emulador
                .cargar_programa(*desde, palabras)
                .map_err(|e| format!("{programa_rel}: {e}"))?;
        }
        emulador.estado_mut().fijar_pc(inicio);
        let mut depurador = Depurador::nuevo(emulador);
        if let Some((o, fuentes)) = &objeto {
            let pares: Vec<(String, String)> = fuentes
                .iter()
                .map(|f| (f.nombre.clone(), f.texto.clone()))
                .collect();
            depurador.con_fuente(o, &pares);
        }
        let mut sig = self.siguiente.lock().map_err(|_| "estado corrupto")?;
        let id = *sig;
        *sig += 1;
        self.sesiones.lock().map_err(|_| "estado corrupto")?.insert(
            id,
            Arc::new(Mutex::new(SesionIde {
                sesion: lorenz_dbg::comandos::Sesion::nueva(depurador),
                maquina: maquina_rel.into(),
                programa: programa_rel.into(),
            })),
        );
        Ok(id)
    }

    pub fn sesion(&self, id: u32) -> Option<Arc<Mutex<SesionIde>>> {
        self.sesiones.lock().ok()?.get(&id).cloned()
    }

    pub fn cerrar(&self, id: u32) -> bool {
        self.sesiones
            .lock()
            .map(|mut s| s.remove(&id).is_some())
            .unwrap_or(false)
    }

    pub fn ids(&self) -> Vec<u32> {
        self.sesiones
            .lock()
            .map(|s| s.keys().copied().collect())
            .unwrap_or_default()
    }
}
