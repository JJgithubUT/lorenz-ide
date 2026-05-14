//! Ejecuta las tareas de un proyecto (RF-IDE-006).
//!
//! Una tarea se aplana a pasos basicos (`ensamblar`, `ejecutar`, `probar`) y cada paso llama a la
//! misma orden de la linea de comandos que llamaria el usuario, de modo que una tarea nunca hace
//! algo distinto de lo que haria escribiendo la orden a mano.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::ordenes::{ejecutar_cli, SALIDA_ERROR_USO, SALIDA_LIMITE, SALIDA_OK};
use crate::proyecto::{entrada_con, Abierto, Proyecto, PASOS_BASICOS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Paso {
    Ensamblar,
    Ejecutar {
        limite: Option<u64>,
        hasta_el_limite: bool,
    },
    Probar,
}

impl Paso {
    pub fn nombre(&self) -> &'static str {
        match self {
            Paso::Ensamblar => "ensamblar",
            Paso::Ejecutar { .. } => "ejecutar",
            Paso::Probar => "probar",
        }
    }
}

/// Los pasos basicos de una tarea, con las tareas anidadas ya expandidas.
pub fn aplanar(p: &Proyecto, tarea: &str) -> Result<Vec<Paso>, String> {
    let mut fuera = Vec::new();
    expandir(p, tarea, &mut fuera)?;
    Ok(fuera)
}

fn expandir(p: &Proyecto, nombre: &str, fuera: &mut Vec<Paso>) -> Result<(), String> {
    let t = p.tarea(nombre).ok_or_else(|| {
        let hay: Vec<&str> = p.tareas.iter().map(|t| t.nombre.as_str()).collect();
        format!(
            "la tarea `{nombre}` no existe (hay: {})",
            if hay.is_empty() {
                "ninguna".to_string()
            } else {
                hay.join(", ")
            }
        )
    })?;
    for paso in &t.pasos {
        match paso.as_str() {
            "ensamblar" => fuera.push(Paso::Ensamblar),
            "ejecutar" => fuera.push(Paso::Ejecutar {
                limite: t.limite,
                hasta_el_limite: t.hasta_el_limite,
            }),
            "probar" => fuera.push(Paso::Probar),
            otra if PASOS_BASICOS.contains(&otra) => unreachable!(),
            otra => expandir(p, otra, fuera)?,
        }
    }
    Ok(())
}

fn texto(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// El programa que produce `ensamblar`.
pub fn programa(a: &Abierto) -> PathBuf {
    a.ruta(a.proyecto.salida())
        .join(format!("{}.hex", a.proyecto.proyecto.nombre))
}

struct Marcha<'a> {
    a: &'a Abierto,
    maquina: PathBuf,
}

impl Marcha<'_> {
    fn construir(&self, salida: &mut dyn Write, errores: &mut dyn Write) -> i32 {
        let p = &self.a.proyecto;
        let carpeta = self.a.ruta(p.salida());
        if let Err(e) = std::fs::create_dir_all(&carpeta) {
            let _ = writeln!(errores, "{}: {e}", carpeta.display());
            return SALIDA_ERROR_USO;
        }
        let hex = programa(self.a);
        let m = texto(&self.maquina);
        let hay_s = p.fuentes.iter().any(|f| f.ruta.ends_with(".s"));
        let hay_sgt = p.fuentes.iter().any(|f| f.ruta.ends_with(".sgt"));
        if p.fuentes.is_empty() {
            let _ = writeln!(errores, "el proyecto no declara ninguna `[[fuente]]`");
            return SALIDA_ERROR_USO;
        }
        if hay_s && hay_sgt {
            let _ = writeln!(
                errores,
                "los fuentes mezclan `.s` y `.sgt`; usa un solo lenguaje por proyecto"
            );
            return SALIDA_ERROR_USO;
        }
        if hay_sgt {
            // Los modulos se resuelven por importacion; el ultimo fuente es el principal.
            let entrada = &p.fuentes[p.fuentes.len() - 1].ruta;
            return ejecutar_cli(
                &[
                    "compilar".into(),
                    m,
                    texto(&self.a.ruta(entrada)),
                    "-o".into(),
                    texto(&hex),
                ],
                salida,
                errores,
            );
        }
        if p.fuentes.len() == 1 {
            return ejecutar_cli(
                &[
                    "ensamblar".into(),
                    m,
                    texto(&self.a.ruta(&p.fuentes[0].ruta)),
                    "-o".into(),
                    texto(&hex),
                ],
                salida,
                errores,
            );
        }
        let mut objetos = Vec::new();
        for (i, f) in p.fuentes.iter().enumerate() {
            let base = Path::new(&f.ruta)
                .file_stem()
                .map_or_else(|| "fuente".into(), |s| s.to_string_lossy().into_owned());
            let objeto = carpeta.join(format!("{i:02}-{base}.o"));
            let parcial = carpeta.join(format!("{i:02}-{base}.hex"));
            let c = ejecutar_cli(
                &[
                    "ensamblar".into(),
                    m.clone(),
                    texto(&self.a.ruta(&f.ruta)),
                    "--objeto".into(),
                    texto(&objeto),
                    "-o".into(),
                    texto(&parcial),
                ],
                salida,
                errores,
            );
            if c != SALIDA_OK {
                return c;
            }
            objetos.push(texto(&objeto));
        }
        let mut v = vec!["enlazar".to_string(), m];
        v.extend(objetos);
        v.extend(["-o".into(), texto(&hex)]);
        ejecutar_cli(&v, salida, errores)
    }

    fn ejecutar(
        &self,
        limite: Option<u64>,
        salida: &mut dyn Write,
        errores: &mut dyn Write,
    ) -> i32 {
        let hex = programa(self.a);
        if !hex.is_file() {
            let _ = writeln!(
                errores,
                "{}: no existe; pon `ensamblar` antes de `ejecutar` en la tarea",
                hex.display()
            );
            return SALIDA_ERROR_USO;
        }
        let mut v = vec!["ejecutar".to_string(), texto(&self.maquina), texto(&hex)];
        if let Some(l) = limite {
            v.extend(["--limite".into(), l.to_string()]);
        }
        // La entrada de cada dispositivo viaja con la opcion que le corresponde.
        let maquina = lorenz_arch::cargar_archivo(texto(&self.maquina)).ok();
        for d in &self.a.proyecto.dispositivos {
            let Some(entrada) = &d.entrada else { continue };
            let tipo = maquina
                .as_ref()
                .and_then(|m| m.dispositivos.iter().find(|x| x.nombre == d.nombre))
                .map(|x| &x.tipo);
            if let Some(opcion) = tipo.and_then(entrada_con) {
                v.extend([opcion.into(), entrada.clone()]);
            }
        }
        ejecutar_cli(&v, salida, errores)
    }

    fn probar(&self, salida: &mut dyn Write, errores: &mut dyn Write) -> i32 {
        let Some(pr) = &self.a.proyecto.proyecto.pruebas else {
            let _ = writeln!(errores, "el proyecto no declara `pruebas` en `[proyecto]`");
            return SALIDA_ERROR_USO;
        };
        ejecutar_cli(&["probar".into(), texto(&self.a.ruta(pr))], salida, errores)
    }
}

/// Ejecuta una tarea. Se detiene en el primer paso que falla y devuelve su codigo de salida.
pub fn ejecutar_tarea(
    a: &Abierto,
    nombre: &str,
    arquitectura: Option<&str>,
    salida: &mut dyn Write,
    errores: &mut dyn Write,
) -> i32 {
    let (pasos, maquina) =
        match aplanar(&a.proyecto, nombre).and_then(|p| Ok((p, a.ruta_maquina(arquitectura)?))) {
            Ok(r) => r,
            Err(e) => {
                let _ = writeln!(errores, "{e}");
                return SALIDA_ERROR_USO;
            }
        };
    if let Err(e) = a.comprobar(arquitectura) {
        let _ = writeln!(errores, "{e}");
        return SALIDA_ERROR_USO;
    }
    let marcha = Marcha { a, maquina };
    for paso in pasos {
        let _ = writeln!(salida, "> {}", paso.nombre());
        let c = match paso {
            Paso::Ensamblar => marcha.construir(salida, errores),
            Paso::Ejecutar {
                limite,
                hasta_el_limite,
            } => {
                let c = marcha.ejecutar(limite, salida, errores);
                if c == SALIDA_LIMITE && hasta_el_limite {
                    let _ = writeln!(salida, "(se alcanzo el limite, como se esperaba)");
                    SALIDA_OK
                } else {
                    c
                }
            }
            Paso::Probar => marcha.probar(salida, errores),
        };
        if c != SALIDA_OK {
            let _ = writeln!(
                errores,
                "la tarea `{nombre}` se detuvo en `{}`",
                paso.nombre()
            );
            return c;
        }
    }
    SALIDA_OK
}
