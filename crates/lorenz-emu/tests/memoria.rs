//! Pruebas de la memoria del emulador (RF-EMU-019 a 024).

mod comun;

use comun::*;
use lorenz_emu::{ErrorEjecucion, Memoria};

fn memoria() -> Memoria {
    Memoria::nueva(&con_tres_regiones()).unwrap()
}

#[test]
fn reserva_una_region_por_cada_declarada() {
    assert_eq!(memoria().numero_de_regiones(), 3);
}

#[test]
fn cuenta_las_celdas_reservadas() {
    // rom 0x100 + ram 0x100 + buzon 0x10
    assert_eq!(memoria().celdas_totales(), 0x100 + 0x100 + 0x10);
}

#[test]
fn toda_la_memoria_arranca_en_cero() {
    let m = memoria();
    assert_eq!(m.leer(0x0000).unwrap(), 0);
    assert_eq!(m.leer(0x01FF).unwrap(), 0);
}

#[test]
fn localiza_la_region_de_una_direccion() {
    let m = memoria();
    assert_eq!(m.region_de(0x0000), Some("rom"));
    assert_eq!(m.region_de(0x00FF), Some("rom"));
    assert_eq!(m.region_de(0x0100), Some("ram"));
    assert_eq!(m.region_de(0x030F), Some("buzon"));
}

#[test]
fn las_direcciones_entre_regiones_no_pertenecen_a_ninguna() {
    let m = memoria();
    assert_eq!(m.region_de(0x0200), None);
    assert_eq!(m.region_de(0x02FF), None);
    assert!(!m.es_valida(0x0200));
}

#[test]
fn las_direcciones_pasada_la_ultima_region_no_son_validas() {
    let m = memoria();
    assert!(!m.es_valida(0x0310));
    assert!(!m.es_valida(0xFFFF));
}

#[test]
fn escribe_y_lee_en_ram() {
    let mut m = memoria();
    m.escribir(0x0150, 0x42).unwrap();
    assert_eq!(m.leer(0x0150).unwrap(), 0x42);
}

#[test]
fn escribir_trunca_al_ancho_de_palabra() {
    let mut m = memoria();
    m.escribir(0x0150, 0x1FF).unwrap();
    assert_eq!(m.leer(0x0150).unwrap(), 0xFF);
}

#[test]
fn la_mascara_de_palabra_sale_de_la_maquina() {
    assert_eq!(memoria().mascara_palabra(), 0xFF);
}

#[test]
fn escribir_en_solo_lectura_falla() {
    let mut m = memoria();
    let e = m.escribir(0x0010, 1).unwrap_err();
    assert_eq!(
        e,
        ErrorEjecucion::EscrituraEnSoloLectura { direccion: 0x10, region: "rom".into() }
    );
}

#[test]
fn escribir_en_solo_lectura_no_modifica_nada() {
    let mut m = memoria();
    let _ = m.escribir(0x0010, 1);
    assert_eq!(m.leer(0x0010).unwrap(), 0);
}

#[test]
fn leer_de_solo_escritura_falla() {
    let m = memoria();
    let e = m.leer(0x0300).unwrap_err();
    assert_eq!(
        e,
        ErrorEjecucion::LecturaDeSoloEscritura { direccion: 0x300, region: "buzon".into() }
    );
}

#[test]
fn escribir_en_solo_escritura_funciona() {
    let mut m = memoria();
    assert!(m.escribir(0x0300, 7).is_ok());
}

#[test]
fn leer_fuera_de_toda_region_falla() {
    let e = memoria().leer(0x0200).unwrap_err();
    assert_eq!(e, ErrorEjecucion::FueraDeRegion { direccion: 0x200 });
}

#[test]
fn escribir_fuera_de_toda_region_falla() {
    let e = memoria().escribir(0xFFFF, 1).unwrap_err();
    assert_eq!(e, ErrorEjecucion::FueraDeRegion { direccion: 0xFFFF });
}

#[test]
fn el_mensaje_de_error_nombra_la_direccion_y_la_region() {
    let e = memoria().escribir(0x0010, 1).unwrap_err();
    let texto = e.to_string();
    assert!(texto.contains("0x10"), "{texto}");
    assert!(texto.contains("rom"), "{texto}");
}

#[test]
fn el_acceso_privilegiado_salta_los_permisos() {
    let mut m = memoria();
    m.escribir_privilegiado(0x0010, 0xAB).unwrap();
    assert_eq!(m.leer(0x0010).unwrap(), 0xAB);
    m.escribir_privilegiado(0x0300, 0xCD).unwrap();
    assert_eq!(m.leer_privilegiado(0x0300).unwrap(), 0xCD);
}

#[test]
fn el_acceso_privilegiado_tambien_falla_fuera_de_region() {
    let mut m = memoria();
    assert!(m.leer_privilegiado(0x0200).is_err());
    assert!(m.escribir_privilegiado(0x0200, 1).is_err());
}

#[test]
fn el_acceso_privilegiado_tambien_trunca() {
    let mut m = memoria();
    m.escribir_privilegiado(0x0010, 0x1AB).unwrap();
    assert_eq!(m.leer(0x0010).unwrap(), 0xAB);
}

#[test]
fn carga_un_bloque_en_una_rom() {
    let mut m = memoria();
    m.cargar_bloque(0x0000, &[1, 2, 3, 4]).unwrap();
    assert_eq!(m.volcar(0x0000, 4).unwrap(), vec![1, 2, 3, 4]);
}

#[test]
fn cargar_un_bloque_es_todo_o_nada() {
    let mut m = memoria();
    // empieza en la ram y se sale por el final de la region
    let e = m.cargar_bloque(0x01FE, &[9, 9, 9]).unwrap_err();
    assert!(matches!(e, ErrorEjecucion::BloqueFueraDeRegion { .. }));
    assert_eq!(m.leer(0x01FE).unwrap(), 0);
    assert_eq!(m.leer(0x01FF).unwrap(), 0);
}

#[test]
fn un_bloque_no_puede_cruzar_un_hueco() {
    let mut m = memoria();
    assert!(m.cargar_bloque(0x00FE, &[1, 2, 3, 4]).is_ok(), "rom y ram son contiguas");
    let mut m2 = memoria();
    assert!(m2.cargar_bloque(0x01FF, &[1, 2]).is_err(), "despues de la ram hay un hueco");
}

#[test]
fn un_bloque_vacio_siempre_cabe() {
    assert!(memoria().cargar_bloque(0x0200, &[]).is_ok());
}

#[test]
fn volcar_un_rango_con_hueco_falla() {
    assert!(memoria().volcar(0x01FE, 4).is_err());
}

#[test]
fn volcar_no_exige_permiso_de_lectura() {
    let mut m = memoria();
    m.escribir(0x0300, 5).unwrap();
    assert_eq!(m.volcar(0x0300, 1).unwrap(), vec![5]);
}

#[test]
fn la_direccion_maxima_no_desborda_al_cargar() {
    let mut m = memoria();
    assert!(m.cargar_bloque(u64::MAX, &[1, 2]).is_err());
}

#[test]
fn una_region_demasiado_grande_se_rechaza() {
    let texto = r#"
[maquina]
nombre = "enorme"
formato = "0.1"
bits_palabra = 8
bits_direccion = 40
arranque = 0x0000

[[registro]]
nombre = "PC"
bits = 40
rol = "pc"

[[instruccion]]
mnemonico = "HLT"
codigo = "1111 ...."
semantica = "detener"

[[region]]
nombre = "gigante"
desde = 0x0
hasta = 0xFFFFFFFF
acceso = "rw"
"#;
    let maquina = lorenz_arch::cargar(texto).unwrap();
    let e = Memoria::nueva(&maquina).unwrap_err();
    assert!(matches!(e, ErrorEjecucion::RegionDemasiadoGrande { .. }), "{e}");
}

#[test]
fn ejemplo_8_reserva_su_mapa_completo() {
    let m = Memoria::nueva(&ejemplo_8()).unwrap();
    assert_eq!(m.numero_de_regiones(), 3);
    assert_eq!(m.region_de(0xF000), Some("video"));
    assert_eq!(m.dispositivo_de(0xF000), Some("pantalla"));
    assert_eq!(m.dispositivo_de(0x2000), None);
}
