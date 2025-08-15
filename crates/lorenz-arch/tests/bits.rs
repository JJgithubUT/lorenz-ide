//! Pruebas del lenguaje de campos de bits (RF-ARQ-014, RF-ARQ-015, RF-ARQ-017).

use lorenz_arch::bits::{Codificacion, ErrorBits, Pieza};

#[test]
fn analiza_bits_fijos() {
    let c = Codificacion::analizar("0000 ....").unwrap();
    assert_eq!(c.longitud_en_palabras(), 1);
    assert_eq!(c.palabras()[0].ancho(), 8);
    assert_eq!(c.palabras()[0].valor_fijo(), 0);
}

#[test]
fn los_espacios_no_cuentan() {
    let con = Codificacion::analizar("0001 rd:2 rs:2").unwrap();
    let sin = Codificacion::analizar("0001rd:2rs:2").unwrap();
    assert_eq!(con.palabras()[0].ancho(), sin.palabras()[0].ancho());
}

#[test]
fn calcula_el_ancho_de_una_palabra() {
    let c = Codificacion::analizar("0001 rd:2 rs:2").unwrap();
    assert_eq!(c.palabras()[0].ancho(), 8);
}

#[test]
fn separa_varias_palabras() {
    let c = Codificacion::analizar("0010 rd:2 .. | imm:8").unwrap();
    assert_eq!(c.longitud_en_palabras(), 2);
    assert_eq!(c.palabras()[0].ancho(), 8);
    assert_eq!(c.palabras()[1].ancho(), 8);
}

#[test]
fn admite_tres_palabras() {
    let c = Codificacion::analizar("0100 .... | dir:8 | dir2:8").unwrap();
    assert_eq!(c.longitud_en_palabras(), 3);
}

#[test]
fn el_valor_fijo_toma_solo_los_bits_literales() {
    let c = Codificacion::analizar("1111 ....").unwrap();
    assert_eq!(c.palabras()[0].valor_fijo(), 0b1111_0000);
}

#[test]
fn la_mascara_marca_los_bits_fijos() {
    let c = Codificacion::analizar("0011 rd:2 rs:2").unwrap();
    assert_eq!(c.palabras()[0].mascara(), 0b1111_0000);
}

#[test]
fn el_relleno_no_entra_en_la_mascara() {
    let c = Codificacion::analizar("0110 ....").unwrap();
    assert_eq!(c.palabras()[0].mascara(), 0b1111_0000);
}

#[test]
fn ubica_un_campo_por_nombre() {
    let c = Codificacion::analizar("0001 rd:2 rs:2").unwrap();
    let rd = c.campo("rd").unwrap();
    assert_eq!(rd.palabra, 0);
    assert_eq!(rd.bits, 2);
    assert_eq!(rd.desplazamiento, 2);
    let rs = c.campo("rs").unwrap();
    assert_eq!(rs.desplazamiento, 0);
}

#[test]
fn ubica_un_campo_de_la_segunda_palabra() {
    let c = Codificacion::analizar("0010 rd:2 .. | imm:8").unwrap();
    let imm = c.campo("imm").unwrap();
    assert_eq!(imm.palabra, 1);
    assert_eq!(imm.bits, 8);
    assert_eq!(imm.desplazamiento, 0);
}

#[test]
fn lista_los_campos_en_orden() {
    let c = Codificacion::analizar("0001 rd:2 rs:2").unwrap();
    assert_eq!(c.campos(), vec!["rd", "rs"]);
}

#[test]
fn conserva_el_texto_original() {
    let texto = "0011 rd:2 rs:2";
    let c = Codificacion::analizar(texto).unwrap();
    assert_eq!(c.texto(), texto);
}

#[test]
fn rechaza_un_caracter_desconocido() {
    let e = Codificacion::analizar("00?1 ....").unwrap_err();
    assert!(matches!(
        e,
        ErrorBits::CaracterInesperado { caracter: '?', .. }
    ));
}

#[test]
fn rechaza_un_campo_sin_ancho() {
    let e = Codificacion::analizar("0001 rd rs:2").unwrap_err();
    assert!(matches!(e, ErrorBits::CampoSinAncho { .. }));
}

#[test]
fn rechaza_un_campo_con_dos_puntos_y_nada_mas() {
    let e = Codificacion::analizar("0001 rd:").unwrap_err();
    assert!(matches!(e, ErrorBits::CampoSinAncho { .. }));
}

#[test]
fn rechaza_un_campo_de_cero_bits() {
    let e = Codificacion::analizar("0001 rd:0").unwrap_err();
    assert!(matches!(e, ErrorBits::AnchoCero { .. }));
}

#[test]
fn rechaza_un_campo_mayor_de_64_bits() {
    let e = Codificacion::analizar("imm:65").unwrap_err();
    assert!(matches!(e, ErrorBits::AnchoExcesivo { bits: 65, .. }));
}

#[test]
fn rechaza_un_campo_repetido() {
    let e = Codificacion::analizar("0001 rd:2 rd:2").unwrap_err();
    assert!(matches!(e, ErrorBits::CampoDuplicado { .. }));
}

#[test]
fn rechaza_una_codificacion_vacia() {
    assert!(matches!(
        Codificacion::analizar(""),
        Err(ErrorBits::PalabraVacia)
    ));
}

#[test]
fn rechaza_una_palabra_vacia_entre_separadores() {
    assert!(matches!(
        Codificacion::analizar("0001 .... | | imm:8"),
        Err(ErrorBits::PalabraVacia)
    ));
}

#[test]
fn detecta_dos_codificaciones_ambiguas() {
    let a = Codificacion::analizar("0001 rd:2 rs:2").unwrap();
    let b = Codificacion::analizar("0001 rx:4").unwrap();
    assert!(a.es_ambigua_con(&b));
}

#[test]
fn no_son_ambiguas_si_difiere_un_bit_fijo() {
    let a = Codificacion::analizar("0001 rd:2 rs:2").unwrap();
    let b = Codificacion::analizar("0010 rd:2 rs:2").unwrap();
    assert!(!a.es_ambigua_con(&b));
}

#[test]
fn una_codificacion_es_ambigua_consigo_misma() {
    let a = Codificacion::analizar("0001 rd:2 rs:2").unwrap();
    assert!(a.es_ambigua_con(&a));
}

#[test]
fn un_campo_que_cubre_todo_choca_con_cualquiera() {
    let abierta = Codificacion::analizar("op:8").unwrap();
    let concreta = Codificacion::analizar("1111 ....").unwrap();
    assert!(abierta.es_ambigua_con(&concreta));
}

#[test]
fn el_guion_bajo_agrupa_sin_ocupar_bits() {
    let c = Codificacion::analizar("0001_0010").unwrap();
    assert_eq!(c.palabras()[0].ancho(), 8);
}

#[test]
fn las_piezas_declaran_sus_bits() {
    let c = Codificacion::analizar("1 . rd:3").unwrap();
    let piezas = c.palabras()[0].piezas();
    assert_eq!(piezas[0], Pieza::Fijo(true));
    assert_eq!(piezas[1], Pieza::Relleno);
    assert_eq!(piezas[2].bits(), 3);
}

#[test]
fn un_campo_puede_repartirse_en_dos_palabras() {
    let c = Codificacion::analizar("0100 rd:2 .. | dir:8 | dir:8").unwrap();
    assert_eq!(c.longitud_en_palabras(), 3);
    assert_eq!(c.fragmentos("dir").len(), 2);
    assert_eq!(c.bits_totales("dir"), 16);
}

#[test]
fn los_fragmentos_van_en_orden_de_aparicion() {
    let c = Codificacion::analizar("0100 .... | dir:8 | dir:4 ....").unwrap();
    let f = c.fragmentos("dir");
    assert_eq!(f[0].palabra, 1);
    assert_eq!(f[0].bits, 8);
    assert_eq!(f[1].palabra, 2);
    assert_eq!(f[1].bits, 4);
    assert_eq!(f[1].desplazamiento, 4);
}

#[test]
fn un_campo_repetido_en_la_misma_palabra_sigue_siendo_error() {
    assert!(matches!(
        Codificacion::analizar("rd:2 rd:2 .... | imm:8"),
        Err(ErrorBits::CampoDuplicado { .. })
    ));
}

#[test]
fn un_campo_inexistente_no_tiene_fragmentos() {
    let c = Codificacion::analizar("0001 rd:2 rs:2").unwrap();
    assert!(c.fragmentos("zz").is_empty());
    assert_eq!(c.bits_totales("zz"), 0);
}

#[test]
fn un_campo_sin_repartir_tiene_un_solo_fragmento() {
    let c = Codificacion::analizar("0001 rd:2 rs:2").unwrap();
    assert_eq!(c.fragmentos("rd").len(), 1);
    assert_eq!(c.bits_totales("rd"), 2);
}

#[test]
fn campos_unicos_no_repite_los_nombres() {
    let c = Codificacion::analizar("0100 rd:2 .. | dir:8 | dir:8").unwrap();
    assert_eq!(c.campos(), vec!["rd", "dir", "dir"]);
    assert_eq!(c.campos_unicos(), vec!["rd", "dir"]);
}

#[test]
fn el_mismo_nombre_en_palabras_distintas_no_cambia_el_ancho_de_cada_palabra() {
    let c = Codificacion::analizar("0100 rd:2 .. | dir:8 | dir:8").unwrap();
    assert!(c.palabras().iter().all(|p| p.ancho() == 8));
}
