//! Guardar y reproducir un fallo (RF-DEP-030, RF-DEP-031).

use lorenz_dbg::reproducir::Fallo;
use lorenz_dbg::{Depurador, Parada};
use lorenz_emu::Emulador;
use lorenz_ens::ensamblador::{Ensamblador, Fuente};

fn maquina() -> lorenz_arch::modelo::Maquina {
    let t = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    lorenz_arch::cargar(&t).unwrap()
}

fn depurador(texto: &str) -> Depurador {
    let m = maquina();
    let obj = Ensamblador::nuevo(&m)
        .unwrap()
        .ensamblar(&Fuente::nueva("p.s", texto))
        .unwrap();
    let mut emu = Emulador::nuevo(m).unwrap();
    for s in &obj.segmentos {
        emu.cargar_programa(s.direccion, &s.palabras).unwrap();
    }
    Depurador::nuevo(emu)
}

const CON_FALLO: &str = "LDI A, 5\nLDI B, 7\nADD A, B\nST [0x0010], A\nHLT\n";

fn capturar_fallo() -> Fallo {
    let mut d = depurador(CON_FALLO);
    d.grabar(true);
    let texto = match d.continuar(100) {
        Parada::Fallo { error, .. } => error.to_string(),
        otra => panic!("{otra:?}"),
    };
    Fallo::capturar(&mut d, Some(texto)).unwrap()
}

#[test]
fn captura_pasos_y_fallo() {
    let f = capturar_fallo();
    assert_eq!(f.pasos(), 3);
    assert_eq!(f.maquina(), maquina().nombre);
    assert!(f.fallo().unwrap().contains("solo lectura"));
}

#[test]
fn se_reproduce_identico() {
    let f = capturar_fallo();
    let r = f.reproducir(&maquina()).unwrap();
    assert!(r.identica, "{:?}", r.diferencias);
    assert_eq!(r.emulador.estado().registro("A").unwrap(), 12);
}

#[test]
fn ida_y_vuelta_por_texto() {
    let f = capturar_fallo();
    let texto = f.a_toml();
    assert!(texto.contains("formato = \"lorenz-fallo-0.1\""));
    let g = Fallo::desde_toml(&texto).unwrap();
    assert_eq!(f, g);
    assert!(g.reproducir(&maquina()).unwrap().identica);
}

#[test]
fn reproducir_no_necesita_el_programa_original() {
    // Solo el archivo y la descripcion de la arquitectura.
    let texto = capturar_fallo().a_toml();
    let r = Fallo::desde_toml(&texto)
        .unwrap()
        .reproducir(&maquina())
        .unwrap();
    assert!(r.identica);
}

#[test]
fn guarda_lo_hecho_a_mano() {
    let mut d = depurador("LDI A, 1\nADD A, B\nST [0x2000], A\nHLT\n");
    d.grabar(true);
    d.paso(1);
    d.fijar_registro("B", 41).unwrap();
    d.escribir_memoria(0x2004, 0x77).unwrap();
    d.continuar(100);
    let f = Fallo::capturar(&mut d, None).unwrap();
    let texto = f.a_toml();
    assert!(texto.contains("[[intervencion]]"));
    let r = Fallo::desde_toml(&texto)
        .unwrap()
        .reproducir(&maquina())
        .unwrap();
    assert!(r.identica, "{:?}", r.diferencias);
    assert_eq!(r.emulador.memoria().leer_privilegiado(0x2000).unwrap(), 42);
    assert_eq!(
        r.emulador.memoria().leer_privilegiado(0x2004).unwrap(),
        0x77
    );
}

#[test]
fn guarda_las_entradas_de_puertos() {
    let mut d = depurador("HLT\n");
    d.grabar(true);
    d.encolar_entrada(0x01, 0x41).unwrap();
    d.leer_puerto(0x01).unwrap();
    d.paso(1);
    let f = Fallo::capturar(&mut d, None).unwrap();
    let g = Fallo::desde_toml(&f.a_toml()).unwrap();
    assert!(g.reproducir(&maquina()).unwrap().identica);
}

#[test]
fn el_estado_inicial_incluye_registros_y_memoria() {
    let mut d = depurador("LDI A, 3\nHLT\n");
    d.emulador_mut()
        .estado_mut()
        .fijar_registro("B", 9)
        .unwrap();
    d.emulador_mut()
        .memoria_mut()
        .escribir_privilegiado(0x2100, 0xEE)
        .unwrap();
    d.grabar(true);
    d.continuar(10);
    let f = Fallo::capturar(&mut d, None).unwrap();
    let r = f.reproducir(&maquina()).unwrap();
    assert!(r.identica, "{:?}", r.diferencias);
    assert_eq!(r.emulador.estado().registro("B").unwrap(), 9);
    assert_eq!(
        r.emulador.memoria().leer_privilegiado(0x2100).unwrap(),
        0xEE
    );
}

#[test]
fn una_alteracion_se_detecta() {
    let texto = capturar_fallo().a_toml();
    let alterado = texto.replacen("pasos = 3", "pasos = 2", 1);
    let r = Fallo::desde_toml(&alterado)
        .unwrap()
        .reproducir(&maquina())
        .unwrap();
    assert!(!r.identica);
    assert!(!r.diferencias.is_empty());
}

#[test]
fn si_no_hay_fallo_donde_debia() {
    // Un fallo esperado pero el programa corre bien: se avisa.
    let mut d = depurador("LDI A, 5\nHLT\n");
    d.grabar(true);
    d.paso(1);
    let f = Fallo::capturar(&mut d, Some("algo".into())).unwrap();
    let r = f.reproducir(&maquina()).unwrap();
    assert!(!r.identica);
    assert!(r.diferencias.iter().any(|x| x.contains("debia ocurrir")));
}

#[test]
fn maquina_distinta() {
    let f = capturar_fallo();
    let otra = lorenz_arch::cargar(
        &std::fs::read_to_string("../../ejemplos/ejemplo-8-serie/maquina.toml").unwrap(),
    )
    .unwrap();
    assert_ne!(otra.nombre, f.maquina());
    assert!(f.reproducir(&otra).is_err());
}

#[test]
fn archivos_invalidos() {
    assert!(Fallo::desde_toml("esto no es toml [").is_err());
    let texto = capturar_fallo().a_toml();
    assert!(Fallo::desde_toml(&texto.replace("lorenz-fallo-0.1", "otro")).is_err());
    assert!(Fallo::desde_toml(&texto.replacen("0xc", "0xZ", 1)).is_err());
}

#[test]
fn sin_traza_no_se_puede_capturar() {
    let mut d = depurador("HLT\n");
    assert!(Fallo::capturar(&mut d, None).is_err());
}

#[test]
fn capturar_no_mueve_la_maquina() {
    let mut d = depurador("LDI A, 5\nLDI B, 1\nHLT\n");
    d.grabar(true);
    d.continuar(10);
    d.ir_a(1).unwrap();
    Fallo::capturar(&mut d, None).unwrap();
    assert_eq!(d.instante(), 1);
    assert_eq!(d.emulador().estado().registro("A").unwrap(), 5);
    assert_eq!(d.emulador().estado().registro("B").unwrap(), 0);
}

#[test]
fn guarda_teclas_serie_e_interrupciones() {
    let t = std::fs::read_to_string("../../ejemplos/ejemplo-8-serie/maquina.toml").unwrap();
    let m = lorenz_arch::cargar(&t).unwrap();
    let mut d = Depurador::nuevo(Emulador::nuevo(m.clone()).unwrap());
    d.grabar(true);
    d.pulsar_tecla("teclado", 0x41).unwrap();
    d.recibir_serie("serie", b"hola").unwrap();
    d.levantar_interrupcion(0).unwrap();
    d.paso(1);
    let f = Fallo::capturar(&mut d, None).unwrap();
    let texto = f.a_toml();
    assert!(texto.contains("tipo = \"serie\""), "{texto}");
    assert!(texto.contains("686f6c61"), "{texto}"); // "hola" en hexadecimal
    let g = Fallo::desde_toml(&texto).unwrap();
    let r = g.reproducir(&m).unwrap();
    assert!(r.identica, "{:?}", r.diferencias);
}

#[test]
fn serie_con_bytes_invalidos() {
    let t = std::fs::read_to_string("../../ejemplos/ejemplo-8-serie/maquina.toml").unwrap();
    let m = lorenz_arch::cargar(&t).unwrap();
    let mut d = Depurador::nuevo(Emulador::nuevo(m).unwrap());
    d.grabar(true);
    d.recibir_serie("serie", b"a").unwrap();
    let texto = Fallo::capturar(&mut d, None).unwrap().a_toml();
    assert!(Fallo::desde_toml(&texto.replace("\"61\"", "\"6\"")).is_err());
    assert!(Fallo::desde_toml(&texto.replace("\"61\"", "\"zz\"")).is_err());
}
