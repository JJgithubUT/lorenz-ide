//! Exportaciones a Verilog, PDF y Logisim (RF-ECO-024, RF-ECO-034 a RF-ECO-036).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use lorenz_cli::{ejecutar_cli, SALIDA_ERROR_USO, SALIDA_OK};

static N: AtomicUsize = AtomicUsize::new(0);

fn temporal() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "lorenz-exp-{}-{}",
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
const EJEMPLO8: &str = "../../ejemplos/ejemplo-8/maquina.toml";

fn hay_iverilog() -> bool {
    Command::new("iverilog").arg("-V").output().is_ok()
}

/// Registros que imprime `lorenz ejecutar` y el banco de pruebas, como `NOMBRE=0xVALOR`.
fn registros(texto: &str, linea_con: Option<&str>) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for l in texto.lines() {
        let l = match linea_con {
            Some(p) => match l.strip_prefix(p) {
                Some(r) => r.to_string(),
                None => continue,
            },
            None => l.to_string(),
        };
        v.extend(
            l.split_whitespace()
                .filter(|t| t.contains("=0x"))
                .map(String::from),
        );
    }
    v.sort();
    v
}

fn comparar_con_el_emulador(maquina: &str, programa: &Path) {
    let dir = temporal();
    let (c, _, e) = correr(&[
        "exportar",
        "verilog",
        maquina,
        programa.to_str().unwrap(),
        "-o",
        dir.to_str().unwrap(),
    ]);
    assert_eq!(c, SALIDA_OK, "{e}");
    let fuentes: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "v"))
        .collect();
    let sim = dir.join("sim");
    let r = Command::new("iverilog")
        .arg("-o")
        .arg(&sim)
        .args(&fuentes)
        .output()
        .unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    let r = Command::new("vvp")
        .arg(&sim)
        .current_dir(&dir)
        .output()
        .unwrap();
    let verilog = String::from_utf8_lossy(&r.stdout).to_string();
    let (_, emu, _) = correr(&["ejecutar", maquina, programa.to_str().unwrap()]);
    // El emulador escribe `PC=0x..` y el banco `PC=0x..`; las banderas no estan en el banco.
    let de_emulador = registros(&emu, Some("registros: "));
    let de_verilog = registros(&verilog, None);
    assert_eq!(
        de_verilog, de_emulador,
        "\nverilog:\n{verilog}\nemulador:\n{emu}"
    );
}

#[test]
fn el_modelo_verilog_coincide_con_el_emulador() {
    if !hay_iverilog() {
        eprintln!("iverilog no esta instalado: se omite la comparacion");
        return;
    }
    let t = temporal();
    let casos: Vec<(&str, PathBuf)> = vec![
        (
            RISC,
            PathBuf::from("../../ejemplos/risc-16/programas/suma-1-a-10.s"),
        ),
        (
            RISC,
            PathBuf::from("../../ejercicios/4-fibonacci/referencia.s"),
        ),
        (RISC, {
            let p = t.join("memoria.s");
            std::fs::write(&p, "LI R6, 0x100\nADD R6, R6, R6\nADD R6, R6, R6\nADD R6, R6, R6\nADD R6, R6, R6\nADD R6, R6, R6\nADD R6, R6, R6\nLI R1, 77\nSW R1, [R6+3]\nLW R2, [R6+3]\nXOR R3, R1, R2\nHLT\n").unwrap();
            p
        }),
        (EJEMPLO8, {
            let p = t.join("e8.s");
            std::fs::write(
                &p,
                "LDI A, 200\nLDI B, 100\nADD A, B\nST [0x2000], A\nLD C, [0x2000]\nMOV D, C\nHLT\n",
            )
            .unwrap();
            p
        }),
    ];
    for (m, p) in casos {
        comparar_con_el_emulador(m, &p);
    }
}

#[test]
fn verilog_escribe_modulo_banco_y_programa() {
    let dir = temporal();
    let (c, s, _) = correr(&[
        "exportar",
        "verilog",
        RISC,
        "../../ejemplos/risc-16/programas/suma-1-a-10.s",
        "-o",
        dir.to_str().unwrap(),
    ]);
    assert_eq!(c, SALIDA_OK);
    assert!(s.contains("risc_16_cpu.v"));
    let cpu = std::fs::read_to_string(dir.join("risc_16_cpu.v")).unwrap();
    assert!(cpu.contains("module risc_16_cpu") && cpu.contains("endmodule"));
    assert!(cpu.contains("// ADD {rd}, {rs}, {rt}"));
    let banco = std::fs::read_to_string(dir.join("risc_16_tb.v")).unwrap();
    assert!(banco.contains("$readmemh(\"programa.mem\""));
    let mem = std::fs::read_to_string(dir.join("programa.mem")).unwrap();
    assert!(mem.contains("@0\n"));
}

#[test]
fn verilog_avisa_de_lo_que_no_modela() {
    let dir = temporal();
    let (c, _, e) = correr(&["exportar", "verilog", EJEMPLO8, "-o", dir.to_str().unwrap()]);
    assert_eq!(c, SALIDA_OK);
    assert!(
        e.contains("interrupciones") && e.contains("dispositivos"),
        "{e}"
    );
}

#[test]
fn pdf_es_un_documento_bien_formado() {
    let dir = temporal();
    let destino = dir.join("r.pdf");
    let (c, _, e) = correr(&["exportar", "pdf", RISC, "-o", destino.to_str().unwrap()]);
    assert_eq!(c, SALIDA_OK, "{e}");
    let b = std::fs::read(&destino).unwrap();
    assert!(b.starts_with(b"%PDF-1.4"));
    assert!(b.ends_with(b"%%EOF\n"));
    let t = String::from_utf8_lossy(&b);
    // Cada entrada de la tabla de referencias cruzadas apunta a un `N 0 obj`.
    let inicio: usize = t
        .rsplit("startxref\n")
        .next()
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(t[inicio..].starts_with("xref"));
    let entradas: Vec<usize> = t[inicio..]
        .lines()
        .filter(|l| l.ends_with(" n "))
        .map(|l| l[..10].parse().unwrap())
        .collect();
    assert!(entradas.len() > 8);
    for (i, p) in entradas.iter().enumerate() {
        assert!(
            t[*p..].starts_with(&format!("{} 0 obj", i + 1)),
            "objeto {}",
            i + 1
        );
    }
    assert!(t.contains("/Count "));
}

#[test]
fn pdf_pagina_los_documentos_largos_y_codifica_latin1() {
    let md: String = (0..300)
        .map(|i| format!("Linea numero {i} con acentos: canción, año.\n\n"))
        .collect();
    let pdf = lorenz_cli::pdf::markdown_a_pdf(&md, "Prueba (con) parentesis");
    let t = String::from_utf8_lossy(&pdf).to_string();
    let paginas: usize = t
        .split("/Count ")
        .nth(1)
        .unwrap()
        .split(' ')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(paginas >= 5, "{paginas}");
    assert!(t.contains("Prueba \\(con\\) parentesis"));
    assert!(
        pdf.windows(2).any(|w| w == b"ci") && pdf.contains(&0xF3),
        "la o con tilde va como byte Latin-1"
    );
}

#[test]
fn logisim_genera_un_xml_con_los_bloques() {
    let dir = temporal();
    let destino = dir.join("m.circ");
    let (c, _, e) = correr(&[
        "exportar",
        "logisim",
        RISC,
        "../../ejemplos/risc-16/programas/suma-1-a-10.s",
        "-o",
        destino.to_str().unwrap(),
    ]);
    assert_eq!(c, SALIDA_OK, "{e}");
    let t = std::fs::read_to_string(&destino).unwrap();
    assert!(t.starts_with("<?xml"));
    assert!(
        t.contains("name=\"ROM\"") && t.contains("name=\"RAM\"") && t.contains("name=\"Clock\"")
    );
    assert_eq!(t.matches("name=\"Register\"").count(), 9);
    assert!(t.contains("addr/data: 16 16"));
    assert!(t.contains("ADD {rd}, {rs}, {rt}"));
    assert_eq!(
        t.matches("<comp").count(),
        t.matches("</comp>").count() + t.matches("name=\"Clock\"/>").count()
    );
}

#[test]
fn errores_de_uso_de_exportar() {
    assert_eq!(correr(&["exportar"]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["exportar", "svg", RISC]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["exportar", "pdf"]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["exportar", "pdf", RISC]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["exportar", "verilog", RISC]).0, SALIDA_ERROR_USO);
    assert_eq!(
        correr(&["exportar", "pdf", "no-existe.toml", "-o", "x"]).0,
        SALIDA_ERROR_USO
    );
    assert_eq!(
        correr(&["exportar", "pdf", RISC, "a", "b", "-o", "x"]).0,
        SALIDA_ERROR_USO
    );
    assert_eq!(
        correr(&["exportar", "pdf", RISC, "--raro"]).0,
        SALIDA_ERROR_USO
    );
}
