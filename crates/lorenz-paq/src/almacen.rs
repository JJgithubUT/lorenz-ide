//! El almacen local de paquetes instalados y la resolucion de dependencias
//! (RF-ECO-003, RF-ECO-005).
//!
//! Cada paquete se instala en `<raiz>/<nombre>/<version>/`. Las dependencias que faltan se buscan
//! entre los `.lpkg` de las carpetas de repositorio que se indiquen, y se elige siempre la version
//! mas alta que cumple el requisito (D-110).

use std::path::{Path, PathBuf};

use crate::paquete::{ErrorPaquete, Manifiesto, Paquete, NOMBRE_MANIFIESTO};
use crate::version::{Requisito, Version};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Informe {
    /// Paquetes que se instalaron ahora, en el orden en que se instalaron (dependencias primero).
    pub instalados: Vec<(String, Version)>,
    /// Paquetes que ya estaban y cumplian.
    pub ya_estaban: Vec<(String, Version)>,
    pub avisos: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Almacen {
    raiz: PathBuf,
}

fn io(p: &Path, e: std::io::Error) -> ErrorPaquete {
    ErrorPaquete::Entrada(format!("{}: {e}", p.display()))
}

impl Almacen {
    pub fn nuevo(raiz: impl Into<PathBuf>) -> Self {
        Self { raiz: raiz.into() }
    }

    pub fn raiz(&self) -> &Path {
        &self.raiz
    }

    pub fn carpeta_de(&self, nombre: &str, v: Version) -> PathBuf {
        self.raiz.join(nombre).join(v.to_string())
    }

    /// Paquetes instalados con su manifiesto, por nombre y version.
    pub fn instalados(&self) -> Vec<Manifiesto> {
        let mut v = Vec::new();
        let Ok(nombres) = std::fs::read_dir(&self.raiz) else {
            return v;
        };
        for n in nombres.flatten() {
            if n.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            let Ok(versiones) = std::fs::read_dir(n.path()) else {
                continue;
            };
            for ver in versiones.flatten() {
                if let Ok(t) = std::fs::read_to_string(ver.path().join(NOMBRE_MANIFIESTO)) {
                    if let Ok(m) = Manifiesto::leer(&t) {
                        v.push(m);
                    }
                }
            }
        }
        v.sort_by(|a, b| (&a.nombre, a.version).cmp(&(&b.nombre, b.version)));
        v
    }

    /// La version instalada mas alta de `nombre` que cumple `req`.
    pub fn buscar(&self, nombre: &str, req: &Requisito) -> Option<Version> {
        self.instalados()
            .into_iter()
            .filter(|m| m.nombre == nombre && req.cumple(m.version))
            .map(|m| m.version)
            .max()
    }

    /// Instala `paquete` y, antes, sus dependencias (RF-ECO-003, RF-ECO-005). Si algo falla no
    /// queda nada a medias del paquete que fallo; las dependencias ya instaladas se conservan.
    pub fn instalar(
        &self,
        paquete: &Paquete,
        repositorios: &[PathBuf],
    ) -> Result<Informe, ErrorPaquete> {
        let mut informe = Informe::default();
        let mut pila = Vec::new();
        self.instalar_con(paquete, repositorios, &mut pila, &mut informe)?;
        Ok(informe)
    }

    fn instalar_con(
        &self,
        paquete: &Paquete,
        repos: &[PathBuf],
        pila: &mut Vec<String>,
        informe: &mut Informe,
    ) -> Result<(), ErrorPaquete> {
        let m = &paquete.manifiesto;
        if pila.contains(&m.nombre) {
            let mut ciclo = pila.clone();
            ciclo.push(m.nombre.clone());
            return Err(ErrorPaquete::Dependencia(format!(
                "las dependencias forman un ciclo: {}",
                ciclo.join(" -> ")
            )));
        }
        pila.push(m.nombre.clone());
        for a in paquete.avisos() {
            if !informe.avisos.contains(&a) {
                informe.avisos.push(a);
            }
        }
        for (dep, req) in &m.dependencias {
            if let Some(v) = self.buscar(dep, req) {
                if !informe.ya_estaban.contains(&(dep.clone(), v))
                    && !informe.instalados.contains(&(dep.clone(), v))
                {
                    informe.ya_estaban.push((dep.clone(), v));
                }
                continue;
            }
            let candidato = buscar_en_repositorios(dep, req, repos)?.ok_or_else(|| {
                ErrorPaquete::Dependencia(format!(
                    "falta `{dep}` ({req}), que exige `{}`; no esta instalado ni en los repositorios dados",
                    m.nombre
                ))
            })?;
            self.instalar_con(&candidato, repos, pila, informe)?;
        }
        pila.pop();
        let destino = self.carpeta_de(&m.nombre, m.version);
        if destino.join(NOMBRE_MANIFIESTO).exists() {
            let v = (m.nombre.clone(), m.version);
            if !informe.ya_estaban.contains(&v) && !informe.instalados.contains(&v) {
                informe.ya_estaban.push(v);
            }
            return Ok(());
        }
        self.escribir(paquete, &destino)?;
        informe.instalados.push((m.nombre.clone(), m.version));
        Ok(())
    }

    /// Escribe en una carpeta temporal y la renombra, para que no quede un paquete a medias.
    fn escribir(&self, p: &Paquete, destino: &Path) -> Result<(), ErrorPaquete> {
        let tmp = self.raiz.join(format!(
            ".instalando-{}-{}",
            p.manifiesto.nombre,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let r = (|| -> Result<(), ErrorPaquete> {
            for (ruta, datos) in &p.archivos {
                let f = tmp.join(ruta);
                if let Some(d) = f.parent() {
                    std::fs::create_dir_all(d).map_err(|e| io(d, e))?;
                }
                std::fs::write(&f, datos).map_err(|e| io(&f, e))?;
            }
            if let Some(d) = destino.parent() {
                std::fs::create_dir_all(d).map_err(|e| io(d, e))?;
            }
            std::fs::rename(&tmp, destino).map_err(|e| io(destino, e))
        })();
        if r.is_err() {
            let _ = std::fs::remove_dir_all(&tmp);
        }
        r
    }
}

/// El paquete `nombre` de version mas alta que cumple `req` entre los `.lpkg` de las carpetas.
/// Un `.lpkg` danado se ignora con error solo si es el unico candidato: aqui se omite.
fn buscar_en_repositorios(
    nombre: &str,
    req: &Requisito,
    repos: &[PathBuf],
) -> Result<Option<Paquete>, ErrorPaquete> {
    let mut mejor: Option<Paquete> = None;
    for r in repos {
        let Ok(entradas) = std::fs::read_dir(r) else {
            continue;
        };
        let mut rutas: Vec<PathBuf> = entradas
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "lpkg"))
            .collect();
        rutas.sort();
        for ruta in rutas {
            let Ok(bytes) = std::fs::read(&ruta) else {
                continue;
            };
            let Ok(p) = Paquete::desde_bytes(&bytes) else {
                continue;
            };
            if p.manifiesto.nombre == nombre
                && req.cumple(p.manifiesto.version)
                && mejor
                    .as_ref()
                    .map_or(true, |m| p.manifiesto.version > m.manifiesto.version)
            {
                mejor = Some(p);
            }
        }
    }
    Ok(mejor)
}
