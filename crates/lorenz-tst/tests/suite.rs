//! Ejecutar proyectos completos (RF-TST-011, RF-TST-012, RF-TST-015, RF-TST-031, RF-TST-032).

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use lorenz_tst::ejecutor::Veredicto;
use lorenz_tst::formato::Proyecto;
use lorenz_tst::suite::{
    ejecutar_suite, Informe, Opciones, FORMATO_RESULTADOS, SALIDA_ERROR_EN_PRUEBAS, SALIDA_FALLAN,
    SALIDA_OK,
};

fn proyecto(nombre: &str, pruebas: &str) -> Proyecto {
    // Cada llamada usa su propia carpeta: las pruebas corren en paralelo y reescribir los mismos
    // archivos mientras otra prueba los lee daba fallos esporadicos.
    static CONTADOR: AtomicUsize = AtomicUsize::new(0);
    let n = CONTADOR.fetch_add(1, Ordering::SeqCst);
    let dir =
        std::env::temp_dir().join(format!("lorenz-suite-{}-{nombre}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("suma.s"),
        "LDI A, 200\nLDI B, 100\nADD A, B\nHLT\n",
    )
    .unwrap();
    std::fs::write(dir.join("mal.s"), "BAILAR\n").unwrap();
    let m = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ejemplos/ejemplo-8/maquina.toml");
    let texto = format!(
        "formato = \"lorenz-pruebas-0.1\"\nmaquina = {:?}\nsemilla = 5\n{pruebas}",
        m.to_string_lossy()
    );
    Proyecto::desde_texto(&texto, "pruebas.toml", &dir).unwrap()
}

fn prueba(nombre: &str, etiquetas: &str, programa: &str, espera: &str) -> String {
    format!(
        "[[prueba]]\nnombre = \"{nombre}\"\netiquetas = [{etiquetas}]\nprograma = \"{programa}\"\n[prueba.espera]\n{espera}\n"
    )
}

fn mixto() -> Proyecto {
    let t = [
        prueba(
            "suma",
            "\"rapida\", \"aritmetica\"",
            "suma.s",
            "registros = { A = 44 }",
        ),
        prueba(
            "suma carry",
            "\"aritmetica\"",
            "suma.s",
            "banderas = { C = true }",
        ),
        prueba(
            "suma mal",
            "\"aritmetica\"",
            "suma.s",
            "registros = { A = 1 }",
        ),
        prueba("no ensambla", "", "mal.s", ""),
    ]
    .concat();
    proyecto("mixto", &t)
}

fn ids(i: &Informe) -> Vec<(String, Veredicto)> {
    i.pruebas
        .iter()
        .map(|r| (r.nombre.clone(), r.veredicto))
        .collect()
}

#[test]
fn una_sola_orden_ejecuta_todo() {
    let i = ejecutar_suite(&mixto(), &Opciones::default());
    assert_eq!(i.resumen.total, 4);
    assert_eq!(i.resumen.pasan, 2);
    assert_eq!(i.resumen.fallan, 1);
    assert_eq!(i.resumen.errores, 1);
    assert_eq!(i.resumen.omitidas, 0);
}

#[test]
fn el_informe_conserva_el_orden_del_archivo() {
    let i = ejecutar_suite(&mixto(), &Opciones::default());
    let nombres: Vec<&str> = i.pruebas.iter().map(|r| r.nombre.as_str()).collect();
    assert_eq!(nombres, ["suma", "suma carry", "suma mal", "no ensambla"]);
}

#[test]
fn en_paralelo_da_lo_mismo_que_en_serie() {
    let p = mixto();
    let a = ejecutar_suite(
        &p,
        &Opciones {
            hilos: 1,
            ..Default::default()
        },
    );
    let b = ejecutar_suite(
        &p,
        &Opciones {
            hilos: 4,
            ..Default::default()
        },
    );
    assert_eq!(ids(&a), ids(&b));
    for (x, y) in a.pruebas.iter().zip(&b.pruebas) {
        assert_eq!(x.fallos, y.fallos);
        assert_eq!(x.instrucciones, y.instrucciones);
        assert_eq!(x.ciclos, y.ciclos);
    }
}

#[test]
fn muchas_pruebas_en_paralelo() {
    let t: String = (0..60)
        .map(|n| prueba(&format!("p{n}"), "", "suma.s", "registros = { A = 44 }"))
        .collect();
    let i = ejecutar_suite(
        &proyecto("muchas", &t),
        &Opciones {
            hilos: 8,
            ..Default::default()
        },
    );
    assert_eq!(i.resumen.pasan, 60);
    assert_eq!(i.pruebas[59].nombre, "p59");
}

#[test]
fn con_cero_hilos_elige_solo() {
    let i = ejecutar_suite(
        &mixto(),
        &Opciones {
            hilos: 0,
            ..Default::default()
        },
    );
    assert_eq!(i.resumen.total, 4);
}

#[test]
fn filtrar_por_nombre() {
    let i = ejecutar_suite(
        &mixto(),
        &Opciones {
            nombre: Some("CARRY".into()),
            ..Default::default()
        },
    );
    assert_eq!(i.resumen.total, 1);
    assert_eq!(i.resumen.omitidas, 3);
    assert_eq!(i.pruebas[0].nombre, "suma carry");
}

#[test]
fn filtrar_por_etiqueta() {
    let i = ejecutar_suite(
        &mixto(),
        &Opciones {
            etiqueta: Some("aritmetica".into()),
            ..Default::default()
        },
    );
    assert_eq!(i.resumen.total, 3);
    assert_eq!(i.resumen.omitidas, 1);
}

#[test]
fn un_filtro_que_no_encuentra_nada_no_es_un_error() {
    let i = ejecutar_suite(
        &mixto(),
        &Opciones {
            nombre: Some("zzz".into()),
            ..Default::default()
        },
    );
    assert_eq!(i.resumen.total, 0);
    assert_eq!(i.codigo_de_salida(), SALIDA_OK);
}

#[test]
fn codigos_de_salida_distinguibles() {
    let t = [prueba("a", "", "suma.s", "registros = { A = 44 }")].concat();
    assert_eq!(
        ejecutar_suite(&proyecto("c0", &t), &Opciones::default()).codigo_de_salida(),
        SALIDA_OK
    );
    let t = [
        prueba("a", "", "suma.s", "registros = { A = 44 }"),
        prueba("b", "", "suma.s", "registros = { A = 1 }"),
    ]
    .concat();
    assert_eq!(
        ejecutar_suite(&proyecto("c1", &t), &Opciones::default()).codigo_de_salida(),
        SALIDA_FALLAN
    );
    // Un error de preparacion pesa mas que un fallo.
    assert_eq!(
        ejecutar_suite(&mixto(), &Opciones::default()).codigo_de_salida(),
        SALIDA_ERROR_EN_PRUEBAS
    );
    let sol = [prueba("e", "", "mal.s", "")].concat();
    assert_eq!(
        ejecutar_suite(&proyecto("c2", &sol), &Opciones::default()).codigo_de_salida(),
        SALIDA_ERROR_EN_PRUEBAS
    );
    assert_ne!(SALIDA_FALLAN, SALIDA_ERROR_EN_PRUEBAS);
}

#[test]
fn json_legible_por_maquina() {
    let i = ejecutar_suite(&mixto(), &Opciones::default());
    let j = i.a_json();
    let v: serde_json::Value = serde_json::from_str(&j).unwrap();
    assert_eq!(v["formato"], FORMATO_RESULTADOS);
    assert_eq!(v["semilla"], 5);
    assert_eq!(v["resumen"]["total"], 4);
    assert_eq!(v["resumen"]["fallan"], 1);
    assert_eq!(v["pruebas"][0]["veredicto"], "pasa");
    assert_eq!(v["pruebas"][2]["veredicto"], "falla");
    assert_eq!(v["pruebas"][3]["veredicto"], "error");
    assert_eq!(v["pruebas"][2]["fallos"][0]["que"], "registro A");
    assert_eq!(
        v["pruebas"][2]["fallos"][0]["donde"]["instruccion"],
        "ADD A, B"
    );
    assert!(v["pruebas"][3]["motivo"]
        .as_str()
        .unwrap()
        .contains("mal.s"));
    assert_eq!(v["pruebas"][0]["etiquetas"][1], "aritmetica");
}

#[test]
fn json_ida_y_vuelta() {
    let i = ejecutar_suite(&mixto(), &Opciones::default());
    let j = Informe::desde_json(&i.a_json()).unwrap();
    assert_eq!(i.pruebas, j.pruebas);
    assert_eq!(i.resumen, j.resumen);
}

#[test]
fn json_de_otro_formato() {
    let i = ejecutar_suite(&mixto(), &Opciones::default());
    let malo = i.a_json().replace(FORMATO_RESULTADOS, "otro");
    assert!(Informe::desde_json(&malo).is_err());
    assert!(Informe::desde_json("{").is_err());
}

#[test]
fn texto_con_esperado_obtenido_y_donde() {
    let t = ejecutar_suite(&mixto(), &Opciones::default()).a_texto();
    assert!(t.contains("ok     suma\n"), "{t}");
    assert!(t.contains("FALLA  suma mal"), "{t}");
    assert!(t.contains("ERROR  no ensambla"), "{t}");
    assert!(t.contains("esperado: 0x01 (1)"), "{t}");
    assert!(t.contains("obtenido: 0x2c (44)"), "{t}");
    assert!(t.contains("en la instruccion 2 0x0004: ADD A, B"), "{t}");
    assert!(
        t.contains("4 pruebas: 2 pasan, 1 fallan, 1 con error"),
        "{t}"
    );
}

#[test]
fn texto_menciona_las_omitidas() {
    let i = ejecutar_suite(
        &mixto(),
        &Opciones {
            nombre: Some("suma".into()),
            ..Default::default()
        },
    );
    assert!(i.a_texto().contains("1 omitidas"));
}
