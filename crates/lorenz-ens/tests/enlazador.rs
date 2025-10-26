//! El enlazador (RF-ENS-019, RF-ENS-020).

mod comun;
use comun::*;
use lorenz_ens::enlazador::{enlazar, ErrorEnlace};
use lorenz_ens::ensamblador::{Ensamblador, Fuente, Objeto};

fn obj(m: &lorenz_arch::modelo::Maquina, nombre: &str, texto: &str) -> (String, Objeto) {
    let o = Ensamblador::nuevo(m)
        .unwrap()
        .ensamblar(&Fuente::nueva(nombre, texto))
        .unwrap_or_else(|e| panic!("{}: {}", nombre, e[0]));
    (nombre.to_string(), o)
}

fn enlazado(
    m: &lorenz_arch::modelo::Maquina,
    objs: &[(String, Objeto)],
) -> Result<Objeto, Vec<ErrorEnlace>> {
    enlazar(&Ensamblador::nuevo(m).unwrap(), objs)
}

#[test]
fn el_resultado_es_igual_a_ensamblar_todo_junto() {
    let m = ejemplo_8();
    let a = obj(&m, "a.s", ".externo K\n.org 0x10\nLDI A, K\nNOP");
    let b = obj(&m, "b.s", "K = 9\n.org 0x20\nNOP");
    let o = enlazado(&m, &[a, b]).unwrap();
    let junto = ensamblar(&m, "K = 9\n.org 0x10\nLDI A, K\nNOP\n.org 0x20\nNOP").unwrap();
    assert_eq!(o.palabras(), junto.palabras());
    assert!(o.reubicaciones.is_empty() && o.externos.is_empty());
}

#[test]
fn una_etiqueta_de_otro_archivo_se_resuelve() {
    let m = ejemplo_8();
    let a = obj(&m, "a.s", ".externo destino\n.org 0x10\nLDI A, destino");
    let b = obj(&m, "b.s", ".org 0x40\ndestino: NOP");
    let o = enlazado(&m, &[a, b]).unwrap();
    let junto = ensamblar(&m, ".org 0x10\nLDI A, destino\n.org 0x40\ndestino: NOP").unwrap();
    assert_eq!(o.palabras(), junto.palabras());
    assert_eq!(o.simbolo("destino"), Some(0x40));
}

#[test]
fn el_orden_de_los_objetos_no_cambia_el_resultado() {
    let m = ejemplo_8();
    let a = obj(&m, "a.s", ".externo K\n.org 0x10\nLDI A, K");
    let b = obj(&m, "b.s", "K = 3\n.org 0x20\nNOP");
    let ab = enlazado(&m, &[a.clone(), b.clone()]).unwrap();
    let ba = enlazado(&m, &[b, a]).unwrap();
    assert_eq!(ab.palabras(), ba.palabras());
}

#[test]
fn un_simbolo_sin_definir_se_reporta_con_su_lugar() {
    let m = ejemplo_8();
    let a = obj(&m, "a.s", ".externo falta\n.org 0x10\nNOP\nLDI A, falta");
    let e = enlazado(&m, &[a]).unwrap_err();
    assert_eq!(
        e,
        vec![ErrorEnlace::NoResuelto {
            nombre: "falta".into(),
            usado_en: ("a.s".into(), 4)
        }]
    );
    assert!(e[0].to_string().contains("falta") && e[0].to_string().contains("a.s:4"));
}

#[test]
fn dos_etiquetas_iguales_en_objetos_distintos_son_duplicadas() {
    let m = ejemplo_8();
    let a = obj(&m, "a.s", ".org 0x10\nfin: NOP");
    let b = obj(&m, "b.s", ".org 0x20\nfin: NOP");
    let e = enlazado(&m, &[a, b]).unwrap_err();
    assert_eq!(
        e,
        vec![ErrorEnlace::Duplicado {
            nombre: "fin".into(),
            primero: ("a.s".into(), 2),
            segundo: ("b.s".into(), 2)
        }]
    );
}

#[test]
fn una_constante_repetida_con_el_mismo_valor_no_choca_pero_con_otro_si() {
    let m = ejemplo_8();
    let a = obj(&m, "a.s", "N = 5\n.org 0x10\nNOP");
    let b = obj(&m, "b.s", "N = 5\n.org 0x20\nNOP");
    assert!(enlazado(&m, &[a.clone(), b]).is_ok());
    let c = obj(&m, "c.s", "N = 6\n.org 0x30\nNOP");
    let e = enlazado(&m, &[a, c]).unwrap_err();
    assert!(matches!(e[0], ErrorEnlace::Duplicado { .. }));
}

#[test]
fn todos_los_problemas_se_reportan_juntos() {
    let m = ejemplo_8();
    let a = obj(
        &m,
        "a.s",
        ".externo x, y\n.org 0x10\nfin: LDI A, x\nLDI B, y",
    );
    let b = obj(&m, "b.s", ".org 0x30\nfin: NOP");
    let e = enlazado(&m, &[a, b]).unwrap_err();
    assert_eq!(e.len(), 3);
}

#[test]
fn dos_objetos_que_escriben_la_misma_direccion_chocan() {
    let m = ejemplo_8();
    let a = obj(&m, "a.s", ".org 0x10\nNOP\nNOP");
    let b = obj(&m, "b.s", ".org 0x11\nNOP");
    let e = enlazado(&m, &[a, b]).unwrap_err();
    assert_eq!(
        e,
        vec![ErrorEnlace::Solape {
            direccion: 0x11,
            primero: "a.s".into(),
            segundo: "b.s".into()
        }]
    );
}

#[test]
fn un_valor_que_no_cabe_se_descubre_al_enlazar() {
    let m = ejemplo_8();
    let a = obj(&m, "a.s", ".externo K\n.org 0x10\nLDI A, K");
    let b = obj(&m, "b.s", "K = 100000\n.org 0x20\nNOP");
    let e = enlazado(&m, &[a, b]).unwrap_err();
    assert!(matches!(e[0], ErrorEnlace::AlCompletar(_)), "{:?}", e);
}

#[test]
fn enlazar_un_solo_objeto_sin_externos_lo_deja_igual() {
    let m = ejemplo_8();
    let a = obj(&m, "a.s", "inicio: NOP\n.org 0x20\nNOP");
    let o = enlazado(&m, &[a.clone()]).unwrap();
    assert_eq!(o.palabras(), a.1.palabras());
    assert_eq!(o.simbolos, a.1.simbolos);
}

#[test]
fn el_mapa_del_resultado_reune_el_de_todos_los_objetos() {
    let m = ejemplo_8();
    let a = obj(&m, "a.s", ".org 0x30\nNOP");
    let b = obj(&m, "b.s", ".org 0x10\nNOP");
    let o = enlazado(&m, &[a, b]).unwrap();
    let archivos: Vec<_> = o.mapa.iter().map(|e| e.archivo.as_str()).collect();
    assert_eq!(archivos, vec!["b.s", "a.s"]);
}
