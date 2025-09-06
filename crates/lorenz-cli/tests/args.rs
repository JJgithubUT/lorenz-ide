//! Pruebas de la interpretacion de argumentos.

use lorenz_cli::args::{interpretar, Disparo, ErrorUso, Orden, Volcado, LIMITE_POR_OMISION};

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
            pantalla: false,
            traza_maquina: false,
            vigilar: false,
            teclas: None,
            serie: None,
            volcados: vec![],
            disparos: vec![],
            guion: None,
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
            pantalla: false,
            traza_maquina: false,
            vigilar: false,
            teclas: None,
            serie: None,
            volcados: vec![],
            disparos: vec![],
            guion: None,
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

#[test]
fn pantalla_y_teclas_son_opciones_de_ejecutar() {
    let o = interpretar(&v(&[
        "ejecutar",
        "m",
        "p",
        "--pantalla",
        "--teclas",
        "hola",
    ]));
    assert_eq!(
        o,
        Ok(Orden::Ejecutar {
            maquina: "m".into(),
            programa: "p".into(),
            limite: LIMITE_POR_OMISION,
            traza: false,
            pantalla: true,
            traza_maquina: false,
            vigilar: false,
            teclas: Some("hola".into()),
            serie: None,
            volcados: vec![],
            disparos: vec![],
            guion: None,
            direccion: None
        })
    );
}

#[test]
fn teclas_necesita_un_valor() {
    assert_eq!(
        interpretar(&v(&["ejecutar", "m", "p", "--teclas"])),
        Err(ErrorUso::OpcionSinValor("--teclas".into()))
    );
}

#[test]
fn serie_es_una_opcion_de_ejecutar() {
    let o = interpretar(&v(&["ejecutar", "m", "p", "--serie", "hola"]));
    assert_eq!(
        o,
        Ok(Orden::Ejecutar {
            maquina: "m".into(),
            programa: "p".into(),
            limite: LIMITE_POR_OMISION,
            traza: false,
            pantalla: false,
            traza_maquina: false,
            vigilar: false,
            teclas: None,
            serie: Some("hola".into()),
            volcados: vec![],
            disparos: vec![],
            guion: None,
            direccion: None
        })
    );
}

#[test]
fn serie_necesita_un_valor() {
    assert_eq!(
        interpretar(&v(&["ejecutar", "m", "p", "--serie"])),
        Err(ErrorUso::OpcionSinValor("--serie".into()))
    );
}

fn volcados(args: &[&str]) -> Result<Vec<Volcado>, ErrorUso> {
    let mut todos = vec!["ejecutar", "m", "p"];
    todos.extend_from_slice(args);
    match interpretar(&v(&todos))? {
        Orden::Ejecutar { volcados, .. } => Ok(volcados),
        otra => panic!("se esperaba ejecutar, salio {otra:?}"),
    }
}

#[test]
fn volcar_acepta_desde_y_cantidad() {
    assert_eq!(
        volcados(&["--volcar", "0x2000:16"]),
        Ok(vec![Volcado {
            desde: 0x2000,
            cantidad: 16,
            archivo: None
        }])
    );
}

#[test]
fn volcar_acepta_un_archivo_de_destino() {
    assert_eq!(
        volcados(&["--volcar", "0:4:salida.hex"]),
        Ok(vec![Volcado {
            desde: 0,
            cantidad: 4,
            archivo: Some("salida.hex".into())
        }])
    );
}

#[test]
fn el_archivo_de_volcado_puede_contener_dos_puntos() {
    assert_eq!(
        volcados(&["--volcar", "0:4:C:/tmp/x.hex"]),
        Ok(vec![Volcado {
            desde: 0,
            cantidad: 4,
            archivo: Some("C:/tmp/x.hex".into())
        }])
    );
}

#[test]
fn volcar_se_puede_repetir() {
    let v = volcados(&["--volcar", "0:1", "--volcar", "0x10:2"]).unwrap();
    assert_eq!(v.len(), 2);
    assert_eq!(v[1].desde, 0x10);
}

#[test]
fn volcar_sin_cantidad_es_invalido() {
    assert_eq!(
        volcados(&["--volcar", "0x2000"]),
        Err(ErrorUso::VolcadoInvalido("0x2000".into()))
    );
    assert_eq!(
        volcados(&["--volcar", "0x2000:"]),
        Err(ErrorUso::VolcadoInvalido("0x2000:".into()))
    );
}

#[test]
fn volcar_sin_desde_es_invalido() {
    assert_eq!(
        volcados(&["--volcar", ":4"]),
        Err(ErrorUso::VolcadoInvalido(":4".into()))
    );
}

#[test]
fn volcar_con_archivo_vacio_es_invalido() {
    assert_eq!(
        volcados(&["--volcar", "0:4:"]),
        Err(ErrorUso::VolcadoInvalido("0:4:".into()))
    );
}

#[test]
fn volcar_con_un_numero_malo_cita_la_opcion() {
    assert_eq!(
        volcados(&["--volcar", "ab:4"]),
        Err(ErrorUso::NumeroInvalido {
            opcion: "--volcar".into(),
            valor: "ab".into()
        })
    );
}

#[test]
fn volcar_sin_valor_falla() {
    assert_eq!(
        volcados(&["--volcar"]),
        Err(ErrorUso::OpcionSinValor("--volcar".into()))
    );
}

fn disparos(args: &[&str]) -> Result<Vec<Disparo>, ErrorUso> {
    let mut todos = vec!["ejecutar", "m", "p"];
    todos.extend_from_slice(args);
    match interpretar(&v(&todos))? {
        Orden::Ejecutar { disparos, .. } => Ok(disparos),
        otra => panic!("se esperaba ejecutar, salio {otra:?}"),
    }
}

#[test]
fn interrumpir_acepta_linea_e_instruccion() {
    assert_eq!(
        disparos(&["--interrumpir", "1@25"]),
        Ok(vec![Disparo {
            linea: 1,
            instruccion: 25
        }])
    );
}

#[test]
fn interrumpir_acepta_numeros_en_hexadecimal() {
    assert_eq!(
        disparos(&["--interrumpir", "0x2@0x10"]),
        Ok(vec![Disparo {
            linea: 2,
            instruccion: 16
        }])
    );
}

#[test]
fn interrumpir_se_puede_repetir() {
    assert_eq!(
        disparos(&["--interrumpir", "0@1", "--interrumpir", "1@2"])
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn interrumpir_sin_arroba_es_invalido() {
    assert_eq!(
        disparos(&["--interrumpir", "1"]),
        Err(ErrorUso::DisparoInvalido("1".into()))
    );
}

#[test]
fn interrumpir_con_partes_vacias_es_invalido() {
    assert_eq!(
        disparos(&["--interrumpir", "@3"]),
        Err(ErrorUso::DisparoInvalido("@3".into()))
    );
    assert_eq!(
        disparos(&["--interrumpir", "1@"]),
        Err(ErrorUso::DisparoInvalido("1@".into()))
    );
}

#[test]
fn interrumpir_con_una_linea_demasiado_grande_es_invalido() {
    assert_eq!(
        disparos(&["--interrumpir", "99999999999@1"]),
        Err(ErrorUso::DisparoInvalido("99999999999@1".into()))
    );
}

#[test]
fn interrumpir_con_un_numero_malo_cita_la_opcion() {
    assert_eq!(
        disparos(&["--interrumpir", "x@1"]),
        Err(ErrorUso::NumeroInvalido {
            opcion: "--interrumpir".into(),
            valor: "x".into()
        })
    );
}

#[test]
fn interrumpir_sin_valor_falla() {
    assert_eq!(
        disparos(&["--interrumpir"]),
        Err(ErrorUso::OpcionSinValor("--interrumpir".into()))
    );
}

#[test]
fn vigilar_es_una_opcion_de_ejecutar() {
    match interpretar(&v(&["ejecutar", "m", "p", "--vigilar"])).unwrap() {
        Orden::Ejecutar { vigilar, .. } => assert!(vigilar),
        otra => panic!("{otra:?}"),
    }
}

#[test]
fn sin_la_opcion_vigilar_no_se_vigila() {
    match interpretar(&v(&["ejecutar", "m", "p"])).unwrap() {
        Orden::Ejecutar { vigilar, .. } => assert!(!vigilar),
        otra => panic!("{otra:?}"),
    }
}

#[test]
fn guion_es_una_opcion_de_ejecutar() {
    match interpretar(&v(&["ejecutar", "m", "p", "--guion", "g.txt"])).unwrap() {
        Orden::Ejecutar { guion, .. } => assert_eq!(guion, Some("g.txt".into())),
        otra => panic!("{otra:?}"),
    }
}

#[test]
fn guion_necesita_un_valor() {
    assert_eq!(
        interpretar(&v(&["ejecutar", "m", "p", "--guion"])),
        Err(ErrorUso::OpcionSinValor("--guion".into()))
    );
}

#[test]
fn traza_maquina_activa_tambien_la_traza() {
    match interpretar(&v(&["ejecutar", "m", "p", "--traza-maquina"])).unwrap() {
        Orden::Ejecutar {
            traza,
            traza_maquina,
            ..
        } => assert!(traza && traza_maquina),
        o => panic!("{o:?}"),
    }
}

#[test]
fn traza_a_secas_no_pide_el_formato_de_maquina() {
    match interpretar(&v(&["ejecutar", "m", "p", "--traza"])).unwrap() {
        Orden::Ejecutar { traza_maquina, .. } => assert!(!traza_maquina),
        o => panic!("{o:?}"),
    }
}
