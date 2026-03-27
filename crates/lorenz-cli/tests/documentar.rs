//! La orden `documentar` (RF-ECO-017, RF-ECO-026, RF-ARQ-036).

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use lorenz_cli::documentar::{Pasada, Vigilante};
use lorenz_cli::ejecutar_cli;

static N: AtomicUsize = AtomicUsize::new(0);

fn ejemplo(nombre: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../ejemplos")
        .join(nombre)
        .join("maquina.toml")
        .to_string_lossy()
        .into_owned()
}

fn carpeta() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "lorenz-documentar-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn correr(args: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (mut s, mut e) = (Vec::new(), Vec::new());
    let c = ejecutar_cli(&args, &mut s, &mut e);
    (
        c,
        String::from_utf8(s).unwrap(),
        String::from_utf8(e).unwrap(),
    )
}

#[test]
fn documenta_cada_ejemplo_por_la_salida_estandar() {
    for e in ["ejemplo-8", "ejemplo-8-serie", "risc-16", "pila-8"] {
        let (c, s, _) = correr(&["documentar", &ejemplo(e)]);
        assert_eq!(c, 0, "{e}");
        assert!(s.contains("## Codigos de operacion"), "{e}");
        assert!(s.contains("## Mapa de memoria"), "{e}");
    }
}

#[test]
fn escribe_en_un_archivo_con_la_opcion_o() {
    let destino = carpeta().join("isa.md");
    let (c, s, _) = correr(&[
        "documentar",
        &ejemplo("pila-8"),
        "-o",
        &destino.to_string_lossy(),
    ]);
    assert_eq!(c, 0);
    assert!(s.is_empty());
    let doc = std::fs::read_to_string(destino).unwrap();
    assert!(doc.starts_with("# pila-8"));
    // El comentario con que abre el archivo llega al documento.
    assert!(doc.contains("Maquina de pila de 8 bits"));
}

#[test]
fn una_descripcion_invalida_da_error_de_uso() {
    let d = carpeta().join("mala.toml");
    std::fs::write(&d, "no es toml").unwrap();
    let (c, _, e) = correr(&["documentar", &d.to_string_lossy()]);
    assert_eq!(c, 2);
    assert!(e.contains("mala.toml"));
}

#[test]
fn vigilar_exige_archivo_de_salida() {
    let (c, _, e) = correr(&["documentar", &ejemplo("pila-8"), "--vigilar"]);
    assert_eq!(c, 2);
    assert!(e.contains("-o"));
}

#[test]
fn el_vigilante_regenera_solo_cuando_la_descripcion_cambia() {
    let dir = carpeta();
    let maquina = dir.join("m.toml");
    let salida = dir.join("m.md");
    let original = std::fs::read_to_string(ejemplo("pila-8")).unwrap();
    std::fs::write(&maquina, &original).unwrap();
    let mut v = Vigilante::nuevo(&maquina.to_string_lossy(), &salida.to_string_lossy());

    assert_eq!(v.pasar(), Pasada::Regenerada);
    assert_eq!(v.pasar(), Pasada::SinCambios);

    // Se renombra la maquina: la documentacion se rehace sola, sin otra orden.
    std::fs::write(&maquina, original.replace("\"pila-8\"", "\"pila-9\"")).unwrap();
    assert_eq!(v.pasar(), Pasada::Regenerada);
    assert!(std::fs::read_to_string(&salida)
        .unwrap()
        .starts_with("# pila-9"));
}

#[test]
fn una_descripcion_rota_no_borra_la_documentacion_ni_repite_el_error() {
    let dir = carpeta();
    let maquina = dir.join("m.toml");
    let salida = dir.join("m.md");
    std::fs::write(
        &maquina,
        std::fs::read_to_string(ejemplo("pila-8")).unwrap(),
    )
    .unwrap();
    let mut v = Vigilante::nuevo(&maquina.to_string_lossy(), &salida.to_string_lossy());
    assert_eq!(v.pasar(), Pasada::Regenerada);

    std::fs::write(&maquina, "[maquina").unwrap();
    assert!(matches!(v.pasar(), Pasada::Fallo(_)));
    assert_eq!(v.pasar(), Pasada::SinCambios);
    assert!(std::fs::read_to_string(&salida)
        .unwrap()
        .starts_with("# pila-8"));
}

#[test]
fn si_la_descripcion_desaparece_se_avisa() {
    let dir = carpeta();
    let mut v = Vigilante::nuevo(
        &dir.join("no-existe.toml").to_string_lossy(),
        &dir.join("x.md").to_string_lossy(),
    );
    assert!(matches!(v.pasar(), Pasada::Fallo(_)));
}
