//! Inclusion de archivos (RF-ENS-008).

mod comun;
use comun::*;
use lorenz_ens::ensamblador::{Ensamblador, Fuente};
use lorenz_ens::error::Causa;
use lorenz_ens::preprocesar::CargadorEnMemoria;

fn cargador(archivos: &[(&str, &str)]) -> CargadorEnMemoria {
    CargadorEnMemoria {
        archivos: archivos
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    }
}

fn con(
    principal: &str,
    archivos: &[(&str, &str)],
) -> Result<Vec<u64>, Vec<lorenz_ens::error::ErrorEns>> {
    let m = ejemplo_8();
    let e = Ensamblador::nuevo(&m).unwrap();
    e.ensamblar_con(
        &Fuente::nueva("principal.s", principal),
        &cargador(archivos),
    )
    .map(|(o, _)| o.palabras().into_iter().map(|(_, p)| p).collect())
}

#[test]
fn un_archivo_incluido_se_ensambla_en_su_lugar() {
    let r = con(
        "LDI A, 1\n.incluir \"extra.s\"\nHLT",
        &[("extra.s", "LDI B, 2\n")],
    )
    .unwrap();
    assert_eq!(r, vec![0x20, 1, 0x24, 2, 0xF0]);
}

#[test]
fn las_constantes_y_etiquetas_de_un_incluido_se_ven_desde_el_principal() {
    let r = con(
        ".include \"def.s\"\nLDI A, VALOR\nJZ marca\n",
        &[("def.s", "VALOR = 9\nmarca: NOP\n")],
    )
    .unwrap();
    assert_eq!(r, vec![0, 0x20, 9, 0x60, 0, 0]);
}

#[test]
fn las_inclusiones_se_anidan() {
    let r = con(
        ".incluir \"a.s\"",
        &[("a.s", ".incluir \"b.s\"\nNOP"), ("b.s", "HLT")],
    )
    .unwrap();
    assert_eq!(r, vec![0xF0, 0x00]);
}

#[test]
fn incluir_dos_veces_repite_el_contenido() {
    let r = con(".incluir \"a.s\"\n.incluir \"a.s\"", &[("a.s", "NOP")]).unwrap();
    assert_eq!(r, vec![0, 0]);
}

#[test]
fn un_archivo_inexistente_se_reporta_en_la_linea_del_incluir() {
    let e = con("NOP\n.incluir \"nada.s\"", &[]).unwrap_err();
    assert_eq!((e[0].archivo.as_str(), e[0].linea), ("principal.s", 2));
    assert!(matches!(&e[0].causa, Causa::NoSePudoLeer { archivo, .. } if archivo == "nada.s"));
}

#[test]
fn una_inclusion_circular_se_detecta() {
    let e = con(
        ".incluir \"a.s\"",
        &[("a.s", ".incluir \"b.s\""), ("b.s", ".incluir \"a.s\"")],
    )
    .unwrap_err();
    assert!(matches!(e[0].causa, Causa::InclusionCircular { .. }));
}

#[test]
fn un_archivo_que_se_incluye_a_si_mismo_se_detecta() {
    let e = con(".incluir \"principal.s\"", &[("principal.s", "NOP")]).unwrap_err();
    assert!(matches!(e[0].causa, Causa::InclusionCircular { .. }));
}

#[test]
fn un_error_dentro_de_un_incluido_cita_ese_archivo() {
    let e = con(".incluir \"roto.s\"", &[("roto.s", "NOP\nFOO\n")]).unwrap_err();
    assert_eq!((e[0].archivo.as_str(), e[0].linea), ("roto.s", 2));
}

#[test]
fn incluir_sin_comillas_o_sin_argumento_es_un_error() {
    assert!(con(".incluir extra.s", &[("extra.s", "")]).is_err());
    assert!(con(".incluir", &[]).is_err());
    assert!(con(".incluir \"a\", \"b\"", &[]).is_err());
}

#[test]
fn el_ensamblado_devuelve_todos_los_archivos_leidos() {
    let m = ejemplo_8();
    let e = Ensamblador::nuevo(&m).unwrap();
    let c = cargador(&[("a.s", "NOP")]);
    let (_, fuentes) = e
        .ensamblar_con(&Fuente::nueva("principal.s", ".incluir \"a.s\""), &c)
        .unwrap();
    assert_eq!(
        fuentes
            .iter()
            .map(|f| f.nombre.as_str())
            .collect::<Vec<_>>(),
        ["principal.s", "a.s"]
    );
}

#[test]
fn el_mapa_atribuye_cada_tramo_a_su_archivo() {
    let m = ejemplo_8();
    let e = Ensamblador::nuevo(&m).unwrap();
    let c = cargador(&[("a.s", "\nNOP")]);
    let (o, _) = e
        .ensamblar_con(&Fuente::nueva("principal.s", "HLT\n.incluir \"a.s\""), &c)
        .unwrap();
    assert_eq!(
        (o.mapa[0].archivo.as_str(), o.mapa[0].linea),
        ("principal.s", 1)
    );
    assert_eq!((o.mapa[1].archivo.as_str(), o.mapa[1].linea), ("a.s", 2));
}

#[test]
fn una_etiqueta_en_la_linea_del_incluir_se_conserva() {
    let r = con("alli: .incluir \"a.s\"\nJZ alli", &[("a.s", "HLT")]).unwrap();
    assert_eq!(r, vec![0xF0, 0x60, 0, 0]);
}
