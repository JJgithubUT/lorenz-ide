//! La orden `nucleos` (RF-CIC-023).

use std::path::PathBuf;

use lorenz_cli::{ejecutar_cli, SALIDA_ERROR_USO, SALIDA_LIMITE, SALIDA_OK};

fn ruta(r: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(r)
        .to_string_lossy()
        .into_owned()
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

fn temporal(nombre: &str, texto: &str) -> String {
    let d = std::env::temp_dir().join(format!("lorenz-nucleos-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let r = d.join(nombre);
    std::fs::write(&r, texto).unwrap();
    r.to_string_lossy().into_owned()
}

fn maquina(nombre: &str, extra: &str) -> String {
    let base = std::fs::read_to_string(ruta("ejemplos/ejemplo-8-ciclos/maquina.toml")).unwrap();
    temporal(nombre, &format!("{base}\n{extra}\n"))
}

const DOS: &str = "[nucleos]\ncantidad = 2\ncompartida = [\"ram\"]\n";

fn escritor() -> String {
    temporal("escritor.s", "LDI A, 42\nST [0x2000], A\nHLT\n")
}

fn lector() -> String {
    temporal(
        "lector.s",
        &format!("{}LD B, [0x2000]\nHLT\n", "NOP\n".repeat(12)),
    )
}

#[test]
fn dos_nucleos_comparten_memoria_y_se_listan_por_separado() {
    let m = maquina("dos.toml", DOS);
    let (c, s, e) = correr(&["nucleos", &m, &escritor(), &lector()]);
    assert_eq!(c, SALIDA_OK, "{e}{s}");
    assert!(s.starts_with("2 nucleos"), "{s}");
    assert!(s.contains("palabras compartidas copiadas: 1"), "{s}");
    assert!(s.contains("total:"), "{s}");
    let filas = s
        .lines()
        .filter(|l| l.starts_with("0 ") || l.starts_with("1 "))
        .count();
    assert_eq!(filas, 2, "{s}");
}

#[test]
fn un_solo_programa_corre_en_todos_los_nucleos() {
    let m = maquina("uno.toml", DOS);
    let (c, s, e) = correr(&["nucleos", &m, &escritor()]);
    assert_eq!(c, SALIDA_OK, "{e}{s}");
    assert!(s.contains("palabras compartidas copiadas: 2"), "{s}");
}

#[test]
fn el_numero_de_programas_debe_cuadrar() {
    let m = maquina(
        "tres.toml",
        "[nucleos]\ncantidad = 3\ncompartida = [\"ram\"]\n",
    );
    let (c, _, e) = correr(&["nucleos", &m, &escritor(), &lector()]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("3 nucleos") && e.contains("no 2"), "{e}");
}

#[test]
fn una_maquina_sin_nucleos_funciona_como_de_uno() {
    let m = maquina("sin.toml", "");
    let (c, s, e) = correr(&["nucleos", &m, &escritor()]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.starts_with("1 nucleo\n"), "{s}");
}

#[test]
fn el_limite_corta_y_sale_con_su_codigo() {
    let m = maquina("limite.toml", DOS);
    let (c, s, _) = correr(&[
        "nucleos",
        &m,
        &ruta("ejemplos/ejemplo-8-ciclos/programas/bucle.s"),
        "--limite",
        "40",
    ]);
    assert_eq!(c, SALIDA_LIMITE);
    assert!(s.contains("limite de 40"), "{s}");
}

#[test]
fn un_bus_comun_se_nota_en_las_esperas() {
    let con = maquina("bus.toml", &format!("{DOS}[bus]\ncompartido = true\n"));
    let (c, s, e) = correr(&["nucleos", &con, &lector()]);
    assert_eq!(c, SALIDA_OK, "{e}{s}");
    assert!(s.contains("bus comun ocupado"), "{s}");
}

#[test]
fn los_argumentos_que_faltan_o_sobran_se_rechazan() {
    let (c, _, e) = correr(&["nucleos", "m.toml"]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("programa"), "{e}");
    let (c, _, _) = correr(&["nucleos", "m.toml", "p.s", "--nada"]);
    assert_eq!(c, SALIDA_ERROR_USO);
}
