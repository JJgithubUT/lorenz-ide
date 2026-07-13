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

pub const MAX_SESIONES: usize = 16;

/// Una ejecucion en curso, con su depurador.
pub struct SesionIde {
    pub sesion: lorenz_dbg::comandos::Sesion,
    pub maquina: String,
    pub programa: String,
}

pub struct Ide {
    raiz: PathBuf,
    sesiones: Mutex<BTreeMap<u32, Arc<Mutex<SesionIde>>>>,
    siguiente: Mutex<u32>,
}

impl Ide {
    pub fn nuevo(raiz: impl Into<PathBuf>) -> Self {
        Self {
            raiz: raiz.into(),
            sesiones: Mutex::new(BTreeMap::new()),
            siguiente: Mutex::new(1),
        }
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
