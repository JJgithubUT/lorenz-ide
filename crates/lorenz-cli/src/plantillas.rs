//! Las plantillas de proyecto que van dentro del programa (RF-IDE-007, RF-IDE-008).
//!
//! Una plantilla es un conjunto de archivos. En ellos, `__NOMBRE__` se sustituye por el nombre
//! del proyecto nuevo.

pub struct Plantilla {
    pub nombre: &'static str,
    pub descripcion: &'static str,
    pub archivos: &'static [(&'static str, &'static str)],
}

pub const PLANTILLAS: &[Plantilla] = &[
    Plantilla {
        nombre: "programa-simple",
        descripcion: "un programa en ensamblador sobre una maquina de 8 bits, con pruebas",
        archivos: &[
            (
                "proyecto.toml",
                include_str!("../plantillas/programa-simple/proyecto.toml"),
            ),
            (
                "maquina.toml",
                include_str!("../plantillas/programa-simple/maquina.toml"),
            ),
            (
                "pruebas.toml",
                include_str!("../plantillas/programa-simple/pruebas.toml"),
            ),
            (
                "programas/principal.s",
                include_str!("../plantillas/programa-simple/programas/principal.s"),
            ),
        ],
    },
    Plantilla {
        nombre: "so-minimo",
        descripcion: "Lorenz OS: un sistema operativo minimo en SGT con pantalla, teclado y reloj",
        archivos: &[
            (
                "proyecto.toml",
                include_str!("../plantillas/so-minimo/proyecto.toml"),
            ),
            (
                "maquina.toml",
                include_str!("../../../ejemplos/so-ejemplo/maquina.toml"),
            ),
            (
                "README.md",
                include_str!("../../../ejemplos/so-ejemplo/README.md"),
            ),
            (
                "arranque.sgt",
                include_str!("../../../ejemplos/so-ejemplo/arranque.sgt"),
            ),
            (
                "pantalla.sgt",
                include_str!("../../../ejemplos/so-ejemplo/pantalla.sgt"),
            ),
            (
                "reloj.sgt",
                include_str!("../../../ejemplos/so-ejemplo/reloj.sgt"),
            ),
            (
                "planificador.sgt",
                include_str!("../../../ejemplos/so-ejemplo/planificador.sgt"),
            ),
            (
                "teclado.sgt",
                include_str!("../../../ejemplos/so-ejemplo/teclado.sgt"),
            ),
            (
                "so.sgt",
                include_str!("../../../ejemplos/so-ejemplo/so.sgt"),
            ),
        ],
    },
    Plantilla {
        nombre: "arquitectura-nueva",
        descripcion: "una arquitectura minima comentada para empezar a disenar la tuya",
        archivos: &[
            (
                "proyecto.toml",
                include_str!("../plantillas/arquitectura-nueva/proyecto.toml"),
            ),
            (
                "maquina.toml",
                include_str!("../plantillas/arquitectura-nueva/maquina.toml"),
            ),
            (
                "programas/principal.s",
                include_str!("../plantillas/arquitectura-nueva/programas/principal.s"),
            ),
        ],
    },
];

pub fn buscar(nombre: &str) -> Option<&'static Plantilla> {
    PLANTILLAS.iter().find(|p| p.nombre == nombre)
}

/// Un nombre de proyecto aceptable a partir del nombre de una carpeta.
pub fn nombre_de_proyecto(carpeta: &str) -> String {
    let base = std::path::Path::new(carpeta)
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let limpio: String = base
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let limpio = limpio.trim_matches('-').to_string();
    if limpio.is_empty() {
        "proyecto".to_string()
    } else {
        limpio
    }
}

impl Plantilla {
    /// Los archivos con el nombre del proyecto ya puesto.
    pub fn con_nombre(&self, nombre: &str) -> Vec<(&'static str, String)> {
        self.archivos
            .iter()
            .map(|(ruta, texto)| {
                let t = texto.replace("__NOMBRE__", nombre);
                let t = if *ruta == "maquina.toml" {
                    crate::nuevo::renombrar_maquina(&t, nombre)
                } else {
                    t
                };
                (*ruta, t)
            })
            .collect()
    }
}
