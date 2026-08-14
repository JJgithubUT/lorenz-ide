//! Filtros para el IDE alojado: lo que un visitante escribe no puede apuntar fuera de su espacio.
//!
//! Los cargadores (herencia de arquitecturas, `.incluir`, `importar`, contenido inicial de una
//! region) leen archivos por la ruta que les dice el propio texto. En un servidor eso permitiria
//! leer cualquier archivo, asi que al escribir se rechaza lo que nombre una ruta absoluta o suba
//! con `..`, y los dispositivos externos.

/// Palabras que introducen una ruta de archivo en los formatos de Lorenz.
const INTRODUCEN_RUTA: &[&str] = &[
    "hereda",
    "contenido",
    "incluir",
    "include",
    "importar",
    "import",
    "archivo",
    "ruta",
    "script",
    "entrada",
    "salida",
];

fn ruta_sospechosa(r: &str) -> bool {
    let t = r.trim();
    t.starts_with('/')
        || t.starts_with('\\')
        || t.starts_with('~')
        || t.split(['/', '\\']).any(|p| p == "..")
        || t.as_bytes().get(1) == Some(&b':')
        || t.starts_with("file:")
}

/// Los textos entre comillas dobles de una linea.
fn entrecomillados(linea: &str) -> Vec<&str> {
    linea.split('"').skip(1).step_by(2).collect()
}

/// Por que el texto no se puede guardar en un servidor, si no se puede.
pub fn ruta_fuera_del_espacio(texto: &str) -> Option<String> {
    for (i, linea) in texto.lines().enumerate() {
        let sin_comentario = linea.split(';').next().unwrap_or("");
        let sin_comentario = sin_comentario.split('#').next().unwrap_or("");
        let l = sin_comentario.to_ascii_lowercase();
        let cadenas = entrecomillados(sin_comentario);
        if l.contains("\"externo\"") || l.contains("'externo'") {
            return Some(format!(
                "linea {}: los dispositivos externos no estan disponibles en la version web",
                i + 1
            ));
        }
        if !INTRODUCEN_RUTA.iter().any(|p| l.contains(p)) {
            continue;
        }
        if let Some(c) = cadenas.iter().find(|c| ruta_sospechosa(c)) {
            return Some(format!(
                "linea {}: la ruta `{c}` sale del espacio de trabajo (nada de rutas absolutas ni `..`)",
                i + 1
            ));
        }
    }
    None
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn rechaza_rutas_que_salen() {
        for malo in [
            "hereda = \"../otra/maquina.toml\"",
            "hereda = \"/etc/passwd\"",
            ".incluir \"../../secreto.s\"",
            "contenido = \"C:\\\\windows\\\\x\"",
            "importar \"~/x.sgt\"",
            "tipo = \"externo\"",
        ] {
            assert!(ruta_fuera_del_espacio(malo).is_some(), "{malo}");
        }
    }

    #[test]
    fn admite_lo_normal() {
        for bueno in [
            "hereda = \"../risc-16/maquina.toml\""
                .replace("../", "")
                .as_str(),
            ".incluir \"constantes.inc\"",
            "nombre = \"ejemplo\"  # ../no cuenta, es un comentario",
            "        LI R1, 1   ; incluir \"/etc\" tampoco cuenta",
            "descripcion = \"sube con .. y /\"",
        ] {
            assert_eq!(ruta_fuera_del_espacio(bueno), None, "{bueno}");
        }
    }
}
