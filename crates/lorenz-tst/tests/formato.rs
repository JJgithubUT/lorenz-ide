//! Lectura y validacion del archivo de pruebas (RF-TST-001, RF-TST-007 a RF-TST-010, RF-TST-012).

use std::path::Path;

use lorenz_tst::error::ErrorPruebas;
use lorenz_tst::formato::{Evento, Momento, Proyecto, LIMITE_POR_OMISION};

const BASE: &str = "../..";

fn leer(cuerpo: &str) -> Result<Proyecto, ErrorPruebas> {
    let texto = format!(
        "formato = \"lorenz-pruebas-0.1\"\nmaquina = \"ejemplos/ejemplo-8/maquina.toml\"\n{cuerpo}"
    );
    Proyecto::desde_texto(&texto, "pruebas.toml", Path::new(BASE))
}

fn error(cuerpo: &str) -> String {
    leer(cuerpo).expect_err("debia fallar").to_string()
}

#[test]
fn una_prueba_minima() {
    let p = leer("[[prueba]]\nnombre = \"a\"\nprograma = \"x.s\"\n").unwrap();
    assert_eq!(p.pruebas.len(), 1);
    let t = &p.pruebas[0];
    assert_eq!(t.nombre(), "a");
    assert_eq!(t.limite(), LIMITE_POR_OMISION);
    assert!(t.programa.ends_with("x.s"));
    assert_eq!(p.semilla, 0);
}

#[test]
fn sin_pruebas_es_valido() {
    assert!(leer("").unwrap().pruebas.is_empty());
}

#[test]
fn la_semilla_se_lee() {
    assert_eq!(leer("semilla = 77\n").unwrap().semilla, 77);
}

#[test]
fn estado_inicial_y_expectativas() {
    let p = leer(
        r#"
[[prueba]]
nombre = "completa"
programa = "x.s"
limite = 50
[prueba.inicial]
registros = { A = 5, B = "0xFF" }
banderas = { Z = true }
memoria = [ { direccion = 0x2000, palabras = [1, 2, "0x03"] } ]
[prueba.espera]
detenida = true
pc = 4
registros = { A = 10 }
banderas = { C = false }
memoria = [ { direccion = 0x2000, palabras = [9] } ]
ciclos = { valor = 20, tolerancia = 2 }
"#,
    )
    .unwrap();
    let t = &p.pruebas[0].def;
    assert_eq!(t.inicial.registros["B"].0, 255);
    assert_eq!(t.inicial.memoria[0].palabras[2].0, 3);
    assert_eq!(t.espera.ciclos.as_ref().unwrap().tolerancia, 2);
    assert_eq!(p.pruebas[0].limite(), 50);
}

#[test]
fn un_numero_de_64_bits_cabe_como_texto() {
    let p = leer("[[prueba]]\nnombre=\"a\"\nprograma=\"x\"\n[prueba.espera]\npc = \"0xFFFF\"\n")
        .unwrap();
    assert_eq!(p.pruebas[0].def.espera.pc.unwrap().0, 0xFFFF);
}

#[test]
fn un_numero_negativo_se_rechaza() {
    assert!(error(
        "[[prueba]]\nnombre=\"a\"\nprograma=\"x\"\n[prueba.inicial]\nregistros = { A = -1 }\n"
    )
    .contains("negativo"));
}

#[test]
fn un_campo_mal_escrito_se_avisa() {
    let e = error("[[prueba]]\nnombre=\"a\"\nprograma=\"x\"\nlimit = 3\n");
    assert!(e.contains("limit"), "{e}");
}

#[test]
fn formato_desconocido() {
    let t = "formato = \"otro\"\nmaquina = \"m.toml\"\n";
    let e = Proyecto::desde_texto(t, "p.toml", Path::new(BASE)).unwrap_err();
    assert!(matches!(e, ErrorPruebas::Formato { .. }));
}

#[test]
fn maquina_que_no_existe() {
    let t = "formato = \"lorenz-pruebas-0.1\"\nmaquina = \"nada.toml\"\n";
    let e = Proyecto::desde_texto(t, "p.toml", Path::new(BASE)).unwrap_err();
    assert!(matches!(e, ErrorPruebas::Maquina { .. }));
}

#[test]
fn registro_inexistente() {
    let e =
        error("[[prueba]]\nnombre=\"a\"\nprograma=\"x\"\n[prueba.espera]\nregistros = { Q = 1 }\n");
    assert!(e.contains("registro llamado `Q`"), "{e}");
    assert!(e.contains("prueba `a`"), "{e}");
}

#[test]
fn bandera_inexistente() {
    let e = error(
        "[[prueba]]\nnombre=\"a\"\nprograma=\"x\"\n[prueba.inicial]\nbanderas = { X = true }\n",
    );
    assert!(e.contains("bandera llamada `X`"), "{e}");
}

#[test]
fn nombres_repetidos() {
    let e = error(
        "[[prueba]]\nnombre=\"a\"\nprograma=\"x\"\n[[prueba]]\nnombre=\"a\"\nprograma=\"y\"\n",
    );
    assert!(e.contains("dos pruebas con el nombre `a`"), "{e}");
}

#[test]
fn limite_cero() {
    assert!(
        error("[[prueba]]\nnombre=\"a\"\nprograma=\"x\"\nlimite = 0\n").contains("mayor que cero")
    );
}

#[test]
fn tramo_fuera_del_espacio() {
    let e = error("[[prueba]]\nnombre=\"a\"\nprograma=\"x\"\n[prueba.inicial]\nmemoria = [ { direccion = 0xFFFF, palabras = [1, 2] } ]\n");
    assert!(e.contains("se sale"), "{e}");
}

#[test]
fn tramo_vacio() {
    let e = error("[[prueba]]\nnombre=\"a\"\nprograma=\"x\"\n[prueba.inicial]\nmemoria = [ { direccion = 0, palabras = [] } ]\n");
    assert!(e.contains("no tiene palabras"), "{e}");
}

#[test]
fn entrada_como_pulsaciones_del_teclado() {
    let p = leer(
        r#"
[[prueba]]
nombre = "teclas"
programa = "x"
[[prueba.entrada]]
dispositivo = "teclado"
texto = "hi"
en_instruccion = 3
"#,
    )
    .unwrap();
    let e = &p.pruebas[0].eventos;
    assert_eq!(e.len(), 2);
    assert_eq!(e[0].0, Momento::Instruccion(3));
    assert_eq!(
        e[0].1,
        Evento::Tecla {
            dispositivo: "teclado".into(),
            codigo: u64::from(b'h')
        }
    );
    assert_eq!(
        e[1].1,
        Evento::Tecla {
            dispositivo: "teclado".into(),
            codigo: u64::from(b'i')
        }
    );
}

#[test]
fn entrada_sin_dispositivo_usa_el_primero_que_sirve() {
    let p = leer("[[prueba]]\nnombre=\"a\"\nprograma=\"x\"\n[[prueba.entrada]]\ncodigos = [65]\n")
        .unwrap();
    assert!(
        matches!(&p.pruebas[0].eventos[0].1, Evento::Tecla { dispositivo, .. } if dispositivo == "teclado")
    );
    assert_eq!(p.pruebas[0].eventos[0].0, Momento::Instruccion(0));
}

#[test]
fn entrada_necesita_texto_o_codigos() {
    assert!(
        error("[[prueba]]\nnombre=\"a\"\nprograma=\"x\"\n[[prueba.entrada]]\n").contains("texto")
    );
    assert!(error(
        "[[prueba]]\nnombre=\"a\"\nprograma=\"x\"\n[[prueba.entrada]]\ntexto=\"a\"\ncodigos=[1]\n"
    )
    .contains("uno de los dos"));
}

#[test]
fn entrada_a_un_dispositivo_que_no_existe_o_no_sirve() {
    assert!(error("[[prueba]]\nnombre=\"a\"\nprograma=\"x\"\n[[prueba.entrada]]\ndispositivo=\"nada\"\ntexto=\"a\"\n").contains("`nada`"));
    assert!(error("[[prueba]]\nnombre=\"a\"\nprograma=\"x\"\n[[prueba.entrada]]\ndispositivo=\"pantalla\"\ntexto=\"a\"\n").contains("ni una UART"));
}

#[test]
fn interrupciones_en_instantes_dados() {
    let p = leer(
        r#"
[[prueba]]
nombre = "irq"
programa = "x"
[[prueba.interrupcion]]
linea = 1
en_ciclo = 40
[[prueba.interrupcion]]
linea = 1
en_instruccion = 5
"#,
    )
    .unwrap();
    let e = &p.pruebas[0].eventos;
    // Ordenados por momento.
    assert_eq!(e[0].0, Momento::Instruccion(5));
    assert_eq!(e[1].0, Momento::Ciclo(40));
}

#[test]
fn interrupcion_sin_vector() {
    let e = error("[[prueba]]\nnombre=\"a\"\nprograma=\"x\"\n[[prueba.interrupcion]]\nlinea = 9\n");
    assert!(e.contains("linea 9"), "{e}");
}

#[test]
fn un_evento_no_puede_tener_dos_momentos() {
    let e = error("[[prueba]]\nnombre=\"a\"\nprograma=\"x\"\n[[prueba.interrupcion]]\nlinea = 1\nen_instruccion = 1\nen_ciclo = 2\n");
    assert!(e.contains("a la vez"), "{e}");
}

#[test]
fn salida_de_un_dispositivo_necesita_texto_o_codigos() {
    let e = error("[[prueba]]\nnombre=\"a\"\nprograma=\"x\"\n[prueba.espera]\nsalida = [ { dispositivo = \"teclado\" } ]\n");
    assert!(e.contains("texto"), "{e}");
    let e = error("[[prueba]]\nnombre=\"a\"\nprograma=\"x\"\n[prueba.espera]\nsalida = [ { dispositivo = \"nada\", texto = \"a\" } ]\n");
    assert!(e.contains("`nada`"), "{e}");
}

#[test]
fn seleccion_por_nombre_y_etiqueta() {
    let p = leer(
        r#"
[[prueba]]
nombre = "suma simple"
etiquetas = ["aritmetica", "rapida"]
programa = "x"
[[prueba]]
nombre = "Suma con acarreo"
etiquetas = ["aritmetica"]
programa = "x"
[[prueba]]
nombre = "teclado"
etiquetas = ["entrada"]
programa = "x"
"#,
    )
    .unwrap();
    assert_eq!(p.seleccionar(None, None).len(), 3);
    assert_eq!(p.seleccionar(Some("suma"), None).len(), 2);
    assert_eq!(p.seleccionar(None, Some("RAPIDA")).len(), 1);
    assert_eq!(p.seleccionar(Some("suma"), Some("rapida")).len(), 1);
    assert_eq!(p.seleccionar(Some("nada"), None).len(), 0);
}

#[test]
fn toml_roto() {
    let e = Proyecto::desde_texto("esto [ no", "p.toml", Path::new(BASE)).unwrap_err();
    assert!(matches!(e, ErrorPruebas::Sintaxis { .. }));
}
