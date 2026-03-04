//! Bancos de registros conmutables (RF-ARQ-012).

use lorenz_arch::{cargar, ErrorArquitectura, Regla};

fn base(banco: &str) -> String {
    format!(
        r#"
[maquina]
nombre = "prueba"
formato = "0.1"
bits_palabra = 8
bits_direccion = 16
arranque = 0x0000

[[registro]]
nombre = "R1"
bits = 8

[[registro]]
nombre = "R2"
bits = 8

[[registro]]
nombre = "SEL"
bits = 8

[[registro]]
nombre = "PC"
bits = 16
rol = "pc"

{banco}

[[instruccion]]
mnemonico = "HLT"
codigo = "1111 ...."
semantica = "detener"

[[region]]
nombre = "ram"
desde = 0x0000
hasta = 0xFFFF
acceso = "rw"
"#
    )
}

fn codigos(texto: &str) -> Vec<&'static str> {
    match cargar(texto) {
        Ok(_) => Vec::new(),
        Err(ErrorArquitectura::Validacion(f)) => f.iter().map(|f| f.regla.codigo()).collect(),
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn acepta_un_banco_con_selector_copias_y_registros() {
    let m = cargar(&base(
        "[[banco]]\nselector = \"SEL\"\nregistros = [\"R1\", \"R2\"]\ncopias = 4\n",
    ))
    .unwrap();
    assert_eq!(m.bancos_registros.len(), 1);
    assert_eq!(m.bancos_registros[0].copias, 4);
}

#[test]
fn sin_bancos_no_hay_ninguno() {
    assert!(cargar(&base("")).unwrap().bancos_registros.is_empty());
}

#[test]
fn rechaza_menos_de_dos_copias() {
    let c = codigos(&base(
        "[[banco]]\nselector = \"SEL\"\nregistros = [\"R1\"]\ncopias = 1\n",
    ));
    assert_eq!(c, vec![Regla::BancoInvalido.codigo()]);
}

#[test]
fn rechaza_un_selector_o_un_registro_que_no_existen() {
    assert!(!codigos(&base(
        "[[banco]]\nselector = \"NO\"\nregistros = [\"R1\"]\ncopias = 2\n"
    ))
    .is_empty());
    assert!(!codigos(&base(
        "[[banco]]\nselector = \"SEL\"\nregistros = [\"NO\"]\ncopias = 2\n"
    ))
    .is_empty());
}

#[test]
fn rechaza_registros_de_rol_especial_el_selector_dentro_y_los_repetidos() {
    let ban = |regs: &str| {
        codigos(&base(&format!(
            "[[banco]]\nselector = \"SEL\"\nregistros = [{regs}]\ncopias = 2\n"
        )))
    };
    assert!(!ban("\"PC\"").is_empty(), "el pc no se banca");
    assert!(!ban("\"SEL\"").is_empty(), "el selector no va en su banco");
    assert!(!ban("").is_empty(), "un banco sin registros");
    let dos = base(
        "[[banco]]\nselector = \"SEL\"\nregistros = [\"R1\"]\ncopias = 2\n\n[[banco]]\nselector = \"SEL\"\nregistros = [\"R1\"]\ncopias = 2\n",
    );
    assert!(!codigos(&dos).is_empty(), "un registro en dos bancos");
}

#[test]
fn el_error_cita_el_requisito() {
    assert_eq!(Regla::BancoInvalido.requisito(), "RF-ARQ-012");
}
