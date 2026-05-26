//! La orden `explicar` (RF-ECO-044) y la marca de texto generado (RF-ECO-046).

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use lorenz_cli::{ejecutar_cli, SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_OK};

static N: AtomicUsize = AtomicUsize::new(0);

fn temporal() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "lorenz-expl-{}-{}",
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

const RISC: &str = "../../ejemplos/risc-16/maquina.toml";
const SUMA: &str = "../../ejemplos/risc-16/programas/suma-1-a-10.s";

#[test]
fn explica_cada_instruccion_con_lo_que_cambia() {
    let (c, s, e) = correr(&["explicar", RISC, SUMA, "--limite", "100"]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.contains("1. 0000  LI R1, 1"), "{s}");
    assert!(s.contains("R1 pasa de 0x0 a 0x1."), "{s}");
    assert!(
        s.contains("Salta: el contador de programa pasa a 0003."),
        "{s}"
    );
    assert!(s.contains("Escribe 0x37 en la direccion 0x4000."), "{s}");
    assert!(s.contains("La maquina se detiene."), "{s}");
    assert!(
        s.contains("La maquina se detuvo tras 42 instrucciones"),
        "{s}"
    );
}

#[test]
fn la_explicacion_es_siempre_la_misma() {
    let a = correr(&["explicar", RISC, SUMA]).1;
    let b = correr(&["explicar", RISC, SUMA]).1;
    assert_eq!(a, b);
}

#[test]
fn lleva_la_marca_de_generado() {
    let (_, s, _) = correr(&["explicar", RISC, SUMA]);
    assert!(s.contains("Generado por Lorenz"), "{s}");
    assert!(s.contains("`lorenz explicar`"));
}

#[test]
fn el_limite_corta_y_lo_dice() {
    let (c, s, _) = correr(&["explicar", RISC, SUMA, "--limite", "3"]);
    assert_eq!(c, SALIDA_OK);
    assert!(s.contains("3. 0002"));
    assert!(!s.contains("4. 0003"));
    assert!(
        s.contains("Se explican las primeras 3 instrucciones"),
        "{s}"
    );
}

#[test]
fn cuenta_los_cambios_de_banderas_y_los_accesos_a_memoria() {
    let d = temporal();
    let p = d.join("p.s");
    std::fs::write(
        &p,
        "LDI A, 200\nLDI B, 100\nADD A, B\nST [0x2000], A\nLD C, [0x2000]\nHLT\n",
    )
    .unwrap();
    let (c, s, e) = correr(&[
        "explicar",
        "../../ejemplos/ejemplo-8/maquina.toml",
        p.to_str().unwrap(),
    ]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.contains("La bandera C pasa de 0 a 1."), "{s}");
    assert!(s.contains("Escribe 0x2c en la direccion 0x2000."), "{s}");
    assert!(s.contains("Lee 0x2c de la direccion 0x2000."), "{s}");
}

#[test]
fn avisa_cuando_una_instruccion_no_cambia_nada() {
    let d = temporal();
    let p = d.join("p.s");
    std::fs::write(&p, "NOP\nHLT\n").unwrap();
    let (_, s, _) = correr(&["explicar", RISC, p.to_str().unwrap()]);
    assert!(
        s.contains("No cambia ningun registro ni la memoria."),
        "{s}"
    );
}

#[test]
fn un_error_de_ejecucion_se_reporta() {
    let d = temporal();
    let p = d.join("p.s");
    // R1 = 0x4000 - 1: fuera de toda region al escribir en la ROM.
    std::fs::write(&p, "LI R1, 1\nSW R1, [R0+5]\nHLT\n").unwrap();
    let (c, s, e) = correr(&["explicar", RISC, p.to_str().unwrap()]);
    assert_eq!(c, SALIDA_ERROR_EJECUCION, "{s}{e}");
    assert!(e.contains("solo lectura"), "{e}");
}

#[test]
fn errores_de_uso() {
    assert_eq!(correr(&["explicar"]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["explicar", RISC]).0, SALIDA_ERROR_USO);
    assert_eq!(
        correr(&["explicar", RISC, SUMA, "extra"]).0,
        SALIDA_ERROR_USO
    );
    assert_eq!(
        correr(&["explicar", RISC, SUMA, "--limite"]).0,
        SALIDA_ERROR_USO
    );
    assert_eq!(
        correr(&["explicar", RISC, SUMA, "--raro"]).0,
        SALIDA_ERROR_USO
    );
    assert_eq!(
        correr(&["explicar", "no-existe.toml", SUMA]).0,
        SALIDA_ERROR_USO
    );
}

#[test]
fn los_textos_generados_llevan_la_marca() {
    let (_, doc, _) = correr(&["documentar", RISC]);
    assert!(doc.contains("Generado por Lorenz") && doc.contains("`lorenz documentar`"));
    let d = temporal();
    let v = d.join("v");
    correr(&["exportar", "verilog", RISC, "-o", v.to_str().unwrap()]);
    for f in ["risc_16_cpu.v", "risc_16_tb.v"] {
        assert!(
            std::fs::read_to_string(v.join(f))
                .unwrap()
                .contains("Generado por Lorenz"),
            "{f}"
        );
    }
    let c = d.join("m.circ");
    correr(&["exportar", "logisim", RISC, "-o", c.to_str().unwrap()]);
    assert!(std::fs::read_to_string(&c)
        .unwrap()
        .contains("Generado por Lorenz"));
    let pdf = d.join("m.pdf");
    correr(&["exportar", "pdf", RISC, "-o", pdf.to_str().unwrap()]);
    assert!(String::from_utf8_lossy(&std::fs::read(&pdf).unwrap()).contains("Generado por Lorenz"));
    let (_, t, _) = correr(&["importar", "../../importadores/jdh-8.tabla"]);
    assert!(t.contains("Generado por Lorenz"));
}
