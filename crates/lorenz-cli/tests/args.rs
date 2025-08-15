//! Pruebas de la interpretacion de argumentos.

use lorenz_cli::args::{interpretar, ErrorUso, Orden, LIMITE_POR_OMISION};

fn v(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| s.to_string()).collect()
}

#[test]
fn sin_argumentos_falta_la_orden() {
    assert_eq!(interpretar(&[]), Err(ErrorUso::SinOrden));
}

#[test]
fn ayuda_tiene_varios_nombres() {
    for a in ["ayuda", "--ayuda", "-h", "--help"] {
        assert_eq!(interpretar(&v(&[a])), Ok(Orden::Ayuda));
    }
}

#[test]
fn version_tiene_varios_nombres() {
    for a in ["version", "--version", "-V"] {
        assert_eq!(interpretar(&v(&[a])), Ok(Orden::Version));
    }
}

#[test]
fn una_orden_desconocida_se_nombra() {
    assert_eq!(
        interpretar(&v(&["bailar"])),
        Err(ErrorUso::OrdenDesconocida("bailar".into()))
    );
}

#[test]
fn validar_pide_una_maquina() {
    assert_eq!(
        interpretar(&v(&["validar"])),
        Err(ErrorUso::Falta("maquina"))
    );
    assert_eq!(
        interpretar(&v(&["validar", "m.toml"])),
        Ok(Orden::Validar {
            maquina: "m.toml".into()
        })
    );
}

#[test]
fn validar_rechaza_argumentos_de_mas() {
    assert_eq!(
        interpretar(&v(&["validar", "a", "b"])),
        Err(ErrorUso::Sobra("b".into()))
    );
}

#[test]
fn validar_rechaza_opciones() {
    assert_eq!(
        interpretar(&v(&["validar", "--rapido", "a"])),
        Err(ErrorUso::OpcionDesconocida("--rapido".into()))
    );
}

#[test]
fn ejecutar_pide_maquina_y_programa() {
    assert_eq!(
        interpretar(&v(&["ejecutar"])),
        Err(ErrorUso::Falta("maquina"))
    );
    assert_eq!(
        interpretar(&v(&["ejecutar", "m"])),
        Err(ErrorUso::Falta("programa"))
    );
}

#[test]
fn ejecutar_usa_valores_por_omision() {
    assert_eq!(
        interpretar(&v(&["ejecutar", "m", "p"])),
        Ok(Orden::Ejecutar {
            maquina: "m".into(),
            programa: "p".into(),
            limite: LIMITE_POR_OMISION,
            traza: false,
            direccion: None
        })
    );
}

#[test]
fn ejecutar_acepta_todas_las_opciones_en_cualquier_orden() {
    let o = interpretar(&v(&[
        "ejecutar",
        "--traza",
        "m",
        "--limite",
        "50",
        "p",
        "--direccion",
        "0x2000",
    ]));
    assert_eq!(
        o,
        Ok(Orden::Ejecutar {
            maquina: "m".into(),
            programa: "p".into(),
            limite: 50,
            traza: true,
            direccion: Some(0x2000)
        })
    );
}

#[test]
fn los_numeros_admiten_hex_binario_y_separadores() {
    let limite = |t: &str| match interpretar(&v(&["ejecutar", "m", "p", "--limite", t])).unwrap() {
        Orden::Ejecutar { limite, .. } => limite,
        _ => unreachable!(),
    };
    assert_eq!(limite("1_000"), 1000);
    assert_eq!(limite("0x10"), 16);
    assert_eq!(limite("0b101"), 5);
}

#[test]
fn un_numero_invalido_cita_la_opcion_y_el_valor() {
    assert_eq!(
        interpretar(&v(&["ejecutar", "m", "p", "--limite", "mucho"])),
        Err(ErrorUso::NumeroInvalido {
            opcion: "--limite".into(),
            valor: "mucho".into()
        })
    );
}

#[test]
fn una_opcion_con_valor_al_final_falla() {
    assert_eq!(
        interpretar(&v(&["ejecutar", "m", "p", "--limite"])),
        Err(ErrorUso::OpcionSinValor("--limite".into()))
    );
}

#[test]
fn una_opcion_desconocida_falla() {
    assert_eq!(
        interpretar(&v(&["ejecutar", "m", "p", "--colores"])),
        Err(ErrorUso::OpcionDesconocida("--colores".into()))
    );
}

#[test]
fn ejecutar_rechaza_un_tercer_posicional() {
    assert_eq!(
        interpretar(&v(&["ejecutar", "m", "p", "q"])),
        Err(ErrorUso::Sobra("q".into()))
    );
}

#[test]
fn los_mensajes_de_uso_estan_en_espanol() {
    assert!(ErrorUso::SinOrden.to_string().contains("falta la orden"));
}
