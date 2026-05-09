//! El archivo `proyecto.toml` (RF-IDE-001 a RF-IDE-006).

use lorenz_cli::proyecto::Proyecto;

const BASE: &str = r#"
[proyecto]
nombre = "hola"
arquitectura = "a"

[[arquitectura]]
nombre = "a"
ruta = "maquina.toml"

[[arquitectura]]
nombre = "b"
ruta = "otras/maquina-b.toml"

[[fuente]]
ruta = "programas/uno.s"

[[fuente]]
ruta = "programas/dos.s"

[[dispositivo]]
nombre = "teclado0"
entrada = "hola"

[[tarea]]
nombre = "construir"
pasos = ["ensamblar"]

[[tarea]]
nombre = "correr"
descripcion = "construye y ejecuta"
pasos = ["construir", "ejecutar"]
limite = 5000
"#;

fn error(texto: &str) -> String {
    Proyecto::leer(texto).unwrap_err()
}

#[test]
fn lee_un_proyecto_completo() {
    let p = Proyecto::leer(BASE).unwrap();
    assert_eq!(p.proyecto.nombre, "hola");
    assert_eq!(p.arquitecturas.len(), 2);
    assert_eq!(p.activa(None).unwrap().ruta, "maquina.toml");
    assert_eq!(p.activa(Some("b")).unwrap().ruta, "otras/maquina-b.toml");
    // El orden de los fuentes es el de aparicion.
    let rutas: Vec<_> = p.fuentes.iter().map(|f| f.ruta.as_str()).collect();
    assert_eq!(rutas, ["programas/uno.s", "programas/dos.s"]);
    assert_eq!(p.dispositivos[0].entrada.as_deref(), Some("hola"));
    assert_eq!(p.tarea("correr").unwrap().limite, Some(5000));
    assert_eq!(p.salida(), "construccion");
}

#[test]
fn la_unica_arquitectura_es_la_activa() {
    let t = "[proyecto]\nnombre = \"x\"\n[[arquitectura]]\nnombre = \"solo\"\nruta = \"m.toml\"\n";
    assert_eq!(
        Proyecto::leer(t).unwrap().activa(None).unwrap().nombre,
        "solo"
    );
}

#[test]
fn con_varias_hay_que_elegir_una() {
    let t = BASE.replace("arquitectura = \"a\"\n", "");
    let e = error(&t);
    assert!(e.contains("2 arquitecturas") && e.contains("activa"), "{e}");
}

#[test]
fn rechaza_lo_incorrecto() {
    for (de, a, frag) in [
        (
            "nombre = \"hola\"",
            "nombre = \"ho la\"",
            "nombre del proyecto",
        ),
        (
            "arquitectura = \"a\"",
            "arquitectura = \"z\"",
            "`z` no esta declarada",
        ),
        (
            "ruta = \"maquina.toml\"",
            "ruta = \"../fuera.toml\"",
            "no puede salir",
        ),
        (
            "ruta = \"maquina.toml\"",
            "ruta = \"/abs.toml\"",
            "relativa",
        ),
        ("programas/dos.s", "programas/uno.s", "repetida"),
        ("programas/dos.s", "programas/dos.txt", ".s"),
        (
            "nombre = \"correr\"",
            "nombre = \"ensamblar\"",
            "paso basico",
        ),
        (
            "pasos = [\"construir\", \"ejecutar\"]",
            "pasos = []",
            "no tiene pasos",
        ),
        (
            "pasos = [\"construir\", \"ejecutar\"]",
            "pasos = [\"volar\"]",
            "volar",
        ),
        ("limite = 5000", "limite = 0", "mayor que cero"),
        ("[proyecto]\n", "[proyecto]\nextra = 1\n", "extra"),
    ] {
        let e = error(&BASE.replace(de, a));
        assert!(e.contains(frag), "{de} -> {a}: {e}");
    }
}

#[test]
fn rechaza_nombres_repetidos() {
    let e = error(&format!("{BASE}\n[[dispositivo]]\nnombre = \"teclado0\"\n"));
    assert!(e.contains("dispositivo `teclado0` esta repetido"), "{e}");
    let e = error(&format!(
        "{BASE}\n[[tarea]]\nnombre = \"construir\"\npasos = [\"ejecutar\"]\n"
    ));
    assert!(e.contains("tarea `construir` esta repetida"), "{e}");
    let e = error(&BASE.replace("nombre = \"b\"", "nombre = \"a\""));
    assert!(e.contains("arquitectura `a` esta repetida"), "{e}");
}

#[test]
fn sin_arquitectura_no_hay_proyecto() {
    let e = error("[proyecto]\nnombre = \"x\"\n");
    assert!(e.contains("al menos una"), "{e}");
}

#[test]
fn detecta_ciclos_entre_tareas() {
    let t = BASE.replace("pasos = [\"ensamblar\"]", "pasos = [\"correr\"]");
    let e = error(&t);
    assert!(
        e.contains("ciclo") && e.contains("construir -> correr -> construir"),
        "{e}"
    );
    // Una tarea que se llama a si misma.
    let t = format!("{BASE}\n[[tarea]]\nnombre = \"bucle\"\npasos = [\"bucle\"]\n");
    assert!(error(&t).contains("bucle -> bucle"));
}

#[test]
fn una_tarea_puede_repetirse_en_pasos_sin_ser_ciclo() {
    let t = BASE.replace(
        "pasos = [\"construir\", \"ejecutar\"]",
        "pasos = [\"construir\", \"construir\", \"ejecutar\"]",
    );
    assert!(Proyecto::leer(&t).is_ok());
}
