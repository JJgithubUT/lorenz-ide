//! El archivo de proyecto `proyecto.toml` (RF-IDE-001 a RF-IDE-006).
//!
//! Un proyecto reune lo que hace falta para trabajar con una o varias arquitecturas: cuales son,
//! cual esta activa, los fuentes y su orden de compilacion, los dispositivos con su entrada y las
//! tareas con nombre. Este modulo solo lee y valida el archivo; ejecutar las tareas es de
//! [`crate::tareas`].

use std::collections::BTreeSet;

use serde::Deserialize;

pub const NOMBRE_ARCHIVO: &str = "proyecto.toml";
/// Carpeta de salida por omision, relativa al proyecto.
pub const SALIDA_POR_OMISION: &str = "construccion";
/// Pasos que existen sin declararse; una tarea no puede llamarse como ellos.
pub const PASOS_BASICOS: [&str; 3] = ["ensamblar", "ejecutar", "probar"];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatosProyecto {
    pub nombre: String,
    /// Arquitectura activa; puede omitirse si solo hay una.
    pub arquitectura: Option<String>,
    /// Carpeta donde se escriben los programas construidos.
    pub salida: Option<String>,
    /// Archivo de pruebas (`lorenz probar`).
    pub pruebas: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Arquitectura {
    pub nombre: String,
    pub ruta: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fuente {
    pub ruta: String,
}

/// Un dispositivo de la maquina y el texto que recibe al ejecutar.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispositivoProyecto {
    pub nombre: String,
    pub entrada: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tarea {
    pub nombre: String,
    pub descripcion: Option<String>,
    /// Pasos basicos (`ensamblar`, `ejecutar`, `probar`) o nombres de otras tareas.
    pub pasos: Vec<String>,
    /// Limite de instrucciones al ejecutar.
    pub limite: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proyecto {
    pub proyecto: DatosProyecto,
    #[serde(default, rename = "arquitectura")]
    pub arquitecturas: Vec<Arquitectura>,
    #[serde(default, rename = "fuente")]
    pub fuentes: Vec<Fuente>,
    #[serde(default, rename = "dispositivo")]
    pub dispositivos: Vec<DispositivoProyecto>,
    #[serde(default, rename = "tarea")]
    pub tareas: Vec<Tarea>,
}

/// Una ruta dentro del proyecto: relativa, sin `..` y sin `\`.
pub fn ruta_interna(r: &str) -> Result<(), String> {
    if r.is_empty() {
        return Err("la ruta esta vacia".into());
    }
    if r.starts_with('/') || r.contains('\\') || r.as_bytes().get(1) == Some(&b':') {
        return Err(format!("`{r}` debe ser una ruta relativa al proyecto"));
    }
    if r.split('/').any(|p| p == ".." || p.is_empty()) {
        return Err(format!(
            "`{r}` no puede salir del proyecto ni tener tramos vacios"
        ));
    }
    Ok(())
}

fn nombre_valido(n: &str) -> bool {
    !n.is_empty()
        && n.chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

impl Proyecto {
    /// Lee y valida el texto de un `proyecto.toml`.
    pub fn leer(texto: &str) -> Result<Self, String> {
        let p: Proyecto = toml::from_str(texto).map_err(|e| format!("{NOMBRE_ARCHIVO}: {e}"))?;
        p.validar()?;
        Ok(p)
    }

    fn validar(&self) -> Result<(), String> {
        let f = |m: String| format!("{NOMBRE_ARCHIVO}: {m}");
        if !nombre_valido(&self.proyecto.nombre) {
            return Err(f(format!(
                "el nombre del proyecto `{}` solo admite letras, digitos, `-` y `_`",
                self.proyecto.nombre
            )));
        }
        if self.arquitecturas.is_empty() {
            return Err(f("declara al menos una `[[arquitectura]]`".into()));
        }
        let mut vistos = BTreeSet::new();
        for a in &self.arquitecturas {
            if !nombre_valido(&a.nombre) {
                return Err(f(format!(
                    "el nombre de arquitectura `{}` no es valido",
                    a.nombre
                )));
            }
            if !vistos.insert(&a.nombre) {
                return Err(f(format!("la arquitectura `{}` esta repetida", a.nombre)));
            }
            ruta_interna(&a.ruta).map_err(|e| f(format!("arquitectura `{}`: {e}", a.nombre)))?;
        }
        self.activa(None)?;
        if let Some(s) = &self.proyecto.salida {
            ruta_interna(s).map_err(|e| f(format!("salida: {e}")))?;
        }
        if let Some(s) = &self.proyecto.pruebas {
            ruta_interna(s).map_err(|e| f(format!("pruebas: {e}")))?;
        }
        let mut vistos = BTreeSet::new();
        for fu in &self.fuentes {
            ruta_interna(&fu.ruta).map_err(|e| f(format!("fuente: {e}")))?;
            if !(fu.ruta.ends_with(".s") || fu.ruta.ends_with(".sgt")) {
                return Err(f(format!(
                    "fuente `{}`: se esperaba un archivo `.s` (ensamblador) o `.sgt` (SGT)",
                    fu.ruta
                )));
            }
            if !vistos.insert(&fu.ruta) {
                return Err(f(format!("la fuente `{}` esta repetida", fu.ruta)));
            }
        }
        let mut vistos = BTreeSet::new();
        for d in &self.dispositivos {
            if !vistos.insert(&d.nombre) {
                return Err(f(format!("el dispositivo `{}` esta repetido", d.nombre)));
            }
        }
        self.validar_tareas().map_err(f)
    }

    fn validar_tareas(&self) -> Result<(), String> {
        let mut nombres = BTreeSet::new();
        for t in &self.tareas {
            if !nombre_valido(&t.nombre) {
                return Err(format!("el nombre de tarea `{}` no es valido", t.nombre));
            }
            if PASOS_BASICOS.contains(&t.nombre.as_str()) {
                return Err(format!(
                    "la tarea no puede llamarse `{}`: es un paso basico",
                    t.nombre
                ));
            }
            if !nombres.insert(t.nombre.as_str()) {
                return Err(format!("la tarea `{}` esta repetida", t.nombre));
            }
            if t.pasos.is_empty() {
                return Err(format!("la tarea `{}` no tiene pasos", t.nombre));
            }
            if t.limite == Some(0) {
                return Err(format!(
                    "la tarea `{}`: el limite debe ser mayor que cero",
                    t.nombre
                ));
            }
        }
        for t in &self.tareas {
            for p in &t.pasos {
                if !PASOS_BASICOS.contains(&p.as_str()) && !nombres.contains(p.as_str()) {
                    return Err(format!(
                        "la tarea `{}` usa `{p}`, que no es un paso basico ni otra tarea",
                        t.nombre
                    ));
                }
            }
        }
        // Una tarea no puede llegar a si misma por sus pasos.
        for t in &self.tareas {
            let mut pila = vec![t.nombre.as_str()];
            self.buscar_ciclo(&t.nombre, &mut pila)?;
        }
        Ok(())
    }

    fn buscar_ciclo<'a>(&'a self, nombre: &str, pila: &mut Vec<&'a str>) -> Result<(), String> {
        let Some(t) = self.tarea(nombre) else {
            return Ok(());
        };
        for p in &t.pasos {
            let Some(sub) = self.tarea(p) else { continue };
            if pila.contains(&sub.nombre.as_str()) {
                let mut ciclo = pila.clone();
                ciclo.push(&sub.nombre);
                return Err(format!(
                    "las tareas forman un ciclo: {}",
                    ciclo.join(" -> ")
                ));
            }
            pila.push(&sub.nombre);
            self.buscar_ciclo(&sub.nombre, pila)?;
            pila.pop();
        }
        Ok(())
    }

    pub fn tarea(&self, nombre: &str) -> Option<&Tarea> {
        self.tareas.iter().find(|t| t.nombre == nombre)
    }

    /// La arquitectura activa: la pedida, la declarada en `[proyecto]` o la unica que hay.
    pub fn activa(&self, pedida: Option<&str>) -> Result<&Arquitectura, String> {
        let elegida = pedida.or(self.proyecto.arquitectura.as_deref());
        match elegida {
            Some(n) => self.arquitecturas.iter().find(|a| a.nombre == n).ok_or_else(|| {
                let hay: Vec<&str> = self.arquitecturas.iter().map(|a| a.nombre.as_str()).collect();
                format!(
                    "{NOMBRE_ARCHIVO}: la arquitectura `{n}` no esta declarada (hay: {})",
                    hay.join(", ")
                )
            }),
            None if self.arquitecturas.len() == 1 => Ok(&self.arquitecturas[0]),
            None => Err(format!(
                "{NOMBRE_ARCHIVO}: hay {} arquitecturas; indica cual es la activa con `arquitectura = \"...\"` en `[proyecto]`",
                self.arquitecturas.len()
            )),
        }
    }

    pub fn salida(&self) -> &str {
        self.proyecto
            .salida
            .as_deref()
            .unwrap_or(SALIDA_POR_OMISION)
    }
}

/// Un proyecto leido de una carpeta, listo para comprobarse y usarse.
#[derive(Debug, Clone)]
pub struct Abierto {
    pub carpeta: std::path::PathBuf,
    pub proyecto: Proyecto,
}

/// Resultado de comprobar un proyecto contra el disco y la maquina activa.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Comprobacion {
    pub avisos: Vec<String>,
}

impl Abierto {
    /// Abre `ruta`, que es una carpeta con `proyecto.toml` o el propio archivo.
    pub fn abrir(ruta: &str) -> Result<Self, String> {
        let r = std::path::Path::new(ruta);
        let archivo = if r.is_dir() {
            r.join(NOMBRE_ARCHIVO)
        } else {
            r.to_path_buf()
        };
        let texto = std::fs::read_to_string(&archivo)
            .map_err(|e| format!("{}: no se pudo leer: {e}", archivo.display()))?;
        let proyecto = Proyecto::leer(&texto)?;
        let carpeta = match archivo.parent() {
            Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
            _ => std::path::PathBuf::from("."),
        };
        Ok(Self { carpeta, proyecto })
    }

    pub fn ruta(&self, relativa: &str) -> std::path::PathBuf {
        self.carpeta.join(relativa)
    }

    /// Ruta de la maquina de la arquitectura activa (o de la pedida).
    pub fn ruta_maquina(&self, pedida: Option<&str>) -> Result<std::path::PathBuf, String> {
        Ok(self.ruta(&self.proyecto.activa(pedida)?.ruta))
    }

    /// Comprueba que los archivos existen, que la maquina activa carga y que los dispositivos
    /// declarados existen en ella y admiten entrada. Las demas arquitecturas que no cargan solo
    /// dan aviso: no impiden trabajar con la activa.
    pub fn comprobar(&self, pedida: Option<&str>) -> Result<Comprobacion, String> {
        let p = &self.proyecto;
        let activa = p.activa(pedida)?;
        let mut c = Comprobacion::default();
        let maquina =
            lorenz_arch::cargar_archivo(self.ruta(&activa.ruta).to_str().unwrap_or(""))
                .map_err(|e| format!("arquitectura `{}` ({}): {e}", activa.nombre, activa.ruta))?;
        for otra in p.arquitecturas.iter().filter(|a| a.nombre != activa.nombre) {
            let ruta = self.ruta(&otra.ruta);
            if let Err(e) = lorenz_arch::cargar_archivo(ruta.to_str().unwrap_or("")) {
                c.avisos.push(format!(
                    "arquitectura `{}` ({}): {e}",
                    otra.nombre, otra.ruta
                ));
            }
        }
        for f in &p.fuentes {
            if !self.ruta(&f.ruta).is_file() {
                return Err(format!("fuente `{}`: no existe", f.ruta));
            }
        }
        if let Some(pr) = &p.proyecto.pruebas {
            if !self.ruta(pr).is_file() {
                return Err(format!("pruebas `{pr}`: no existe"));
            }
        }
        for d in &p.dispositivos {
            let Some(real) = maquina.dispositivos.iter().find(|x| x.nombre == d.nombre) else {
                let hay: Vec<&str> = maquina
                    .dispositivos
                    .iter()
                    .map(|x| x.nombre.as_str())
                    .collect();
                return Err(format!(
                    "dispositivo `{}`: la maquina `{}` no lo tiene (tiene: {})",
                    d.nombre,
                    activa.nombre,
                    if hay.is_empty() {
                        "ninguno".to_string()
                    } else {
                        hay.join(", ")
                    }
                ));
            };
            if d.entrada.is_some() && entrada_con(&real.tipo).is_none() {
                return Err(format!(
                    "dispositivo `{}`: solo los teclados y las lineas serie reciben entrada",
                    d.nombre
                ));
            }
        }
        Ok(c)
    }
}

/// La opcion de `lorenz ejecutar` que da entrada a un dispositivo de ese tipo.
pub fn entrada_con(tipo: &lorenz_arch::TipoDispositivo) -> Option<&'static str> {
    match tipo {
        lorenz_arch::TipoDispositivo::Teclado => Some("--teclas"),
        lorenz_arch::TipoDispositivo::Uart => Some("--serie"),
        _ => None,
    }
}

fn mostrar(a: &Abierto, activa: &str, salida: &mut dyn std::io::Write) {
    let p = &a.proyecto;
    let _ = writeln!(salida, "proyecto {}", p.proyecto.nombre);
    let _ = writeln!(salida, "  arquitecturas:");
    for ar in &p.arquitecturas {
        let marca = if ar.nombre == activa { "*" } else { " " };
        let _ = writeln!(salida, "   {marca} {} ({})", ar.nombre, ar.ruta);
    }
    let _ = writeln!(salida, "  fuentes (en orden de compilacion):");
    for f in &p.fuentes {
        let _ = writeln!(salida, "    {}", f.ruta);
    }
    if !p.dispositivos.is_empty() {
        let _ = writeln!(salida, "  dispositivos:");
        for d in &p.dispositivos {
            match &d.entrada {
                Some(e) => {
                    let _ = writeln!(salida, "    {} (entrada: {e:?})", d.nombre);
                }
                None => {
                    let _ = writeln!(salida, "    {}", d.nombre);
                }
            }
        }
    }
    if !p.tareas.is_empty() {
        let _ = writeln!(salida, "  tareas:");
        for t in &p.tareas {
            let _ = writeln!(
                salida,
                "    {}: {}{}",
                t.nombre,
                t.pasos.join(" -> "),
                t.descripcion
                    .as_ref()
                    .map_or(String::new(), |d| format!("  # {d}"))
            );
        }
    }
}

/// `lorenz proyecto`: muestra el proyecto y lo comprueba.
pub fn proyecto_cli(
    ruta: &str,
    arquitectura: Option<&str>,
    salida: &mut dyn std::io::Write,
    errores: &mut dyn std::io::Write,
) -> i32 {
    use crate::ordenes::{SALIDA_ERROR_USO, SALIDA_OK};
    let r = Abierto::abrir(ruta).and_then(|a| {
        let activa = a.proyecto.activa(arquitectura)?.nombre.clone();
        let c = a.comprobar(arquitectura)?;
        Ok((a, c, activa))
    });
    match r {
        Ok((a, c, activa)) => {
            mostrar(&a, &activa, salida);
            for av in &c.avisos {
                let _ = writeln!(errores, "aviso: {av}");
            }
            SALIDA_OK
        }
        Err(e) => {
            let _ = writeln!(errores, "{e}");
            SALIDA_ERROR_USO
        }
    }
}

/// `lorenz tarea`: sin nombre lista las tareas; con nombre la ejecuta.
pub fn tarea_cli(
    ruta: &str,
    nombre: Option<&str>,
    arquitectura: Option<&str>,
    salida: &mut dyn std::io::Write,
    errores: &mut dyn std::io::Write,
) -> i32 {
    use crate::ordenes::{SALIDA_ERROR_USO, SALIDA_OK};
    let a = match Abierto::abrir(ruta) {
        Ok(a) => a,
        Err(e) => {
            let _ = writeln!(errores, "{e}");
            return SALIDA_ERROR_USO;
        }
    };
    match nombre {
        Some(n) => crate::tareas::ejecutar_tarea(&a, n, arquitectura, salida, errores),
        None => {
            if a.proyecto.tareas.is_empty() {
                let _ = writeln!(salida, "el proyecto no declara tareas");
            }
            for t in &a.proyecto.tareas {
                let _ = writeln!(
                    salida,
                    "{}: {}{}",
                    t.nombre,
                    t.pasos.join(" -> "),
                    t.descripcion
                        .as_ref()
                        .map_or(String::new(), |d| format!("  # {d}"))
                );
            }
            SALIDA_OK
        }
    }
}
