//! Las ordenes `restricciones` y `variantes` (RF-CIC-033).

use std::path::PathBuf;

use lorenz_cli::{ejecutar_cli, SALIDA_ERROR_USO, SALIDA_LIMITE, SALIDA_NO_CUMPLE, SALIDA_OK};

fn ruta(r: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(r)
        .to_string_lossy()
        .into_owned()
}

fn correr(args: &[&str]) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (mut s, mut e) = (Vec::new(), Vec::new());
    let c = ejecutar_cli(&args, &mut s, &mut e);
    (
        c,
        String::from_utf8(s).unwrap(),
        String::from_utf8(e).unwrap(),
    )
}

fn temporal(nombre: &str, texto: &str) -> String {
    let d = std::env::temp_dir().join(format!("lorenz-variantes-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let r = d.join(nombre);
    std::fs::write(&r, texto).unwrap();
    r.to_string_lossy().into_owned()
}

/// La maquina de ejemplo con ciclos, mas las restricciones dadas.
fn maquina_con(nombre: &str, restricciones: &str) -> String {
    let base = std::fs::read_to_string(ruta("ejemplos/ejemplo-8-ciclos/maquina.toml")).unwrap();
    temporal(nombre, &format!("{base}\n{restricciones}\n"))
}

// Se escribe una sola vez: las pruebas corren a la vez y una lectura no debe coincidir con una
// escritura del mismo archivo.
fn programa() -> String {
    static R: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    R.get_or_init(|| {
        temporal(
            "suma.s",
            "LDI A, 1\nLDI B, 2\nADD A, B\nADD A, B\nADD A, B\nHLT\n",
        )
    })
    .clone()
}

#[test]
fn si_se_cumplen_todas_sale_bien() {
    let m = maquina_con(
        "ok.toml",
        "[[restriccion]]\nmetrica = \"ciclos\"\nmaximo = 1000\n\n\
         [[restriccion]]\nnombre = \"codigo corto\"\nmetrica = \"palabras-codigo\"\nmaximo = 20\n",
    );
    let (c, s, e) = correr(&["restricciones", &m, &programa()]);
    assert_eq!(c, SALIDA_OK, "{e}{s}");
    assert!(s.contains("codigo corto") && s.contains("cumple"), "{s}");
    assert!(s.contains("se cumplen todas"), "{s}");
}

#[test]
fn si_alguna_falla_lo_dice_y_sale_con_su_codigo() {
    let m = maquina_con(
        "falla.toml",
        "[[restriccion]]\nmetrica = \"ciclos\"\nmaximo = 3\n",
    );
    let (c, s, _) = correr(&["restricciones", &m, &programa()]);
    assert_eq!(c, SALIDA_NO_CUMPLE);
    assert!(s.contains("NO CUMPLE"), "{s}");
    assert!(s.contains("no se cumple 1 restriccion"), "{s}");
}

#[test]
fn sin_restricciones_declaradas_es_un_error_de_uso() {
    let m = maquina_con("vacia.toml", "");
    let (c, _, e) = correr(&["restricciones", &m, &programa()]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("no declara ninguna [[restriccion]]"), "{e}");
}

#[test]
fn un_programa_que_no_termina_no_se_puede_comprobar() {
    let m = maquina_con(
        "limite.toml",
        "[[restriccion]]\nmetrica = \"ciclos\"\nmaximo = 10\n",
    );
    let (c, s, _) = correr(&[
        "restricciones",
        &m,
        &ruta("ejemplos/ejemplo-8-ciclos/programas/bucle.s"),
        "--limite",
        "50",
    ]);
    assert_eq!(c, SALIDA_LIMITE);
    assert!(s.contains("no termino en 50"), "{s}");
}

#[test]
fn variantes_lista_las_que_cumplen_primero() {
    let base = correr(&[
        "restricciones",
        &maquina_con(
            "m0.toml",
            "[[restriccion]]\nmetrica = \"ciclos\"\nmaximo = 100000\n",
        ),
        &programa(),
    ]);
    assert_eq!(base.0, SALIDA_OK);
    // un limite imposible: ninguna variante lo cumple
    let m = maquina_con(
        "imposible.toml",
        "[[restriccion]]\nmetrica = \"ciclos\"\nmaximo = 1\n",
    );
    let (c, s, _) = correr(&["variantes", &m, &programa()]);
    assert_eq!(c, SALIDA_NO_CUMPLE);
    assert!(s.contains("0 cumplen"), "{s}");
    assert!(s.contains("ninguna variante cumple"), "{s}");
    // un limite holgado: todas cumplen y la base aparece
    let m = maquina_con(
        "holgado.toml",
        "[[restriccion]]\nmetrica = \"ciclos\"\nmaximo = 100000\n",
    );
    let (c, s, e) = correr(&["variantes", &m, &programa()]);
    assert_eq!(c, SALIDA_OK, "{e}{s}");
    assert!(s.contains("base"), "{s}");
    assert!(s.contains("prediccion="), "{s}");
}

#[test]
fn variantes_sugiere_instrucciones_nuevas_estimadas() {
    let m = maquina_con(
        "isa.toml",
        "[[restriccion]]\nmetrica = \"ciclos\"\nmaximo = 100000\n",
    );
    let (c, s, e) = correr(&["variantes", &m, &programa()]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.contains("instrucciones nuevas"), "{s}");
    assert!(s.contains("ADD+ADD"), "{s}");
}

#[test]
fn cuantas_limita_las_filas_y_bus_anade_variantes() {
    let m = maquina_con(
        "filas.toml",
        "[[restriccion]]\nmetrica = \"ciclos\"\nmaximo = 100000\n",
    );
    let (_, sin, _) = correr(&["variantes", &m, &programa(), "--cuantas", "2"]);
    assert!(sin.contains("mas)"), "{sin}");
    let cuenta = |t: &str| -> usize {
        t.lines()
            .next()
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    let (_, con, _) = correr(&["variantes", &m, &programa(), "--bus"]);
    let (_, sin_bus, _) = correr(&["variantes", &m, &programa()]);
    assert!(cuenta(&con) > cuenta(&sin_bus), "{con}\n{sin_bus}");
}

#[test]
fn las_opciones_desconocidas_y_los_argumentos_que_faltan_se_rechazan() {
    let (c, _, e) = correr(&["restricciones", "m.toml"]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("programa"), "{e}");
    let (c, _, _) = correr(&["restricciones", "m.toml", "p.s", "--bus"]);
    assert_eq!(c, SALIDA_ERROR_USO);
    let (c, _, _) = correr(&["variantes", "m.toml", "p.s", "--nada"]);
    assert_eq!(c, SALIDA_ERROR_USO);
}
