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

// --- bancos de memoria conmutables por puerto (RF-ARQ-033) ---

fn con_region(extra: &str) -> String {
    base("").replace(
        "[[region]]\nnombre = \"ram\"\ndesde = 0x0000\nhasta = 0xFFFF\nacceso = \"rw\"",
        &format!(
            "[[region]]\nnombre = \"ram\"\ndesde = 0x0000\nhasta = 0x7FFF\nacceso = \"rw\"\n\n[[region]]\nnombre = \"ventana\"\ndesde = 0x8000\nhasta = 0x8FFF\nacceso = \"rw\"\n{extra}\n"
        ),
    )
}

#[test]
fn acepta_una_region_con_bancos_y_puerto() {
    let m = cargar(&con_region("bancos = 4\nselector_puerto = 0x10")).unwrap();
    let v = m.regiones.iter().find(|r| r.nombre == "ventana").unwrap();
    assert_eq!((v.bancos, v.selector_puerto), (4, Some(0x10)));
    let r = m.regiones.iter().find(|r| r.nombre == "ram").unwrap();
    assert_eq!((r.bancos, r.selector_puerto), (1, None));
}

#[test]
fn varios_bancos_sin_puerto_selector_no_se_aceptan() {
    let c = codigos(&con_region("bancos = 4"));
    assert_eq!(c, vec![Regla::BancoMemoriaInvalido.codigo()]);
}

#[test]
fn un_puerto_sin_bancos_tampoco() {
    let c = codigos(&con_region("selector_puerto = 0x10"));
    assert_eq!(c, vec![Regla::BancoMemoriaInvalido.codigo()]);
}

#[test]
fn cero_bancos_no_tiene_sentido() {
    assert!(!codigos(&con_region("bancos = 0")).is_empty());
}

#[test]
fn dos_regiones_no_comparten_el_puerto_selector() {
    let t = con_region("bancos = 2\nselector_puerto = 0x10").replace(
        "nombre = \"ram\"\ndesde = 0x0000\nhasta = 0x7FFF\nacceso = \"rw\"",
        "nombre = \"ram\"\ndesde = 0x0000\nhasta = 0x7FFF\nacceso = \"rw\"\nbancos = 2\nselector_puerto = 0x10",
    );
    let c = codigos(&t);
    assert_eq!(c, vec![Regla::BancoMemoriaInvalido.codigo()]);
}

#[test]
fn el_error_de_memoria_cita_su_requisito() {
    assert_eq!(Regla::BancoMemoriaInvalido.requisito(), "RF-ARQ-033");
}
