//! La orden `probar` (RF-TST-011, RF-TST-012, RF-TST-014, RF-TST-030 a RF-TST-032).

use std::io::Cursor;
use std::path::PathBuf;

use lorenz_cli::{ejecutar_cli_con_entrada, SALIDA_ERROR_USO, SALIDA_OK};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn ejemplo() -> String {
    raiz()
        .join("ejemplos/ejemplo-8/pruebas.toml")
        .to_string_lossy()
        .into_owned()
}

fn temporal(nombre: &str, contenido: &str) -> String {
    let dir = std::env::temp_dir().join(format!("lorenz-probar-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ruta = dir.join(nombre);
    std::fs::write(&ruta, contenido).unwrap();
    ruta.to_string_lossy().into_owned()
}

fn correr(args: &[&str], entrada: &str) -> (i32, String, String) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (mut s, mut e) = (Vec::new(), Vec::new());
    let c = ejecutar_cli_con_entrada(&args, &mut Cursor::new(entrada.as_bytes()), &mut s, &mut e);
    (
        c,
        String::from_utf8(s).unwrap(),
        String::from_utf8(e).unwrap(),
    )
}

/// Un proyecto con una prueba que pasa y otra que falla, en una carpeta propia.
fn proyecto_con_fallo(nombre: &str) -> String {
    let dir = std::env::temp_dir().join(format!("lorenz-probar-{}-{nombre}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("suma.s"),
        "LDI A, 200\nLDI B, 100\nADD A, B\nHLT\n",
    )
    .unwrap();
    let m = raiz().join("ejemplos/ejemplo-8/maquina.toml");
    let ruta = dir.join("pruebas.toml");
    std::fs::write(
        &ruta,
        format!(
            "formato = \"lorenz-pruebas-0.1\"\nmaquina = {:?}\n\
             [[prueba]]\nnombre = \"bien\"\nprograma = \"suma.s\"\n[prueba.espera]\nregistros = {{ A = 44 }}\n\
             [[prueba]]\nnombre = \"mal\"\netiquetas = [\"roja\"]\nprograma = \"suma.s\"\n[prueba.espera]\nregistros = {{ A = 1 }}\n",
            m.to_string_lossy()
        ),
    )
    .unwrap();
    ruta.to_string_lossy().into_owned()
}

#[test]
fn el_ejemplo_del_repositorio_pasa() {
    let (c, salida, _) = correr(&["probar", &ejemplo()], "");
    assert_eq!(c, SALIDA_OK, "{salida}");
    assert!(salida.contains("5 pruebas: 5 pasan"), "{salida}");
}

#[test]
fn una_prueba_fallida_devuelve_uno() {
    let p = proyecto_con_fallo("uno");
    let (c, salida, _) = correr(&["probar", &p], "");
    assert_eq!(c, 1);
    assert!(salida.contains("ok     bien"), "{salida}");
    assert!(salida.contains("FALLA  mal"), "{salida}");
    assert!(salida.contains("esperado: 0x01 (1)"), "{salida}");
}

#[test]
fn un_programa_que_no_ensambla_devuelve_cinco() {
    let dir = std::env::temp_dir().join(format!("lorenz-probar-{}-cinco", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("mal.s"), "BAILAR\n").unwrap();
    let m = raiz().join("ejemplos/ejemplo-8/maquina.toml");
    let ruta = dir.join("p.toml");
    std::fs::write(
        &ruta,
        format!(
            "formato = \"lorenz-pruebas-0.1\"\nmaquina = {:?}\n[[prueba]]\nnombre = \"m\"\nprograma = \"mal.s\"\n",
            m.to_string_lossy()
        ),
    )
    .unwrap();
    let (c, salida, _) = correr(&["probar", &ruta.to_string_lossy()], "");
    assert_eq!(c, 5);
    assert!(salida.contains("ERROR  m"), "{salida}");
}

#[test]
fn archivo_de_pruebas_invalido_devuelve_dos() {
    let malo = temporal("malo.toml", "esto no es toml [");
    let (c, _, err) = correr(&["probar", &malo], "");
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(err.starts_with("error:"), "{err}");
    let (c, _, err) = correr(&["probar", "/no/existe.toml"], "");
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(err.contains("no se pudo leer"), "{err}");
}

#[test]
fn json_por_la_salida_estandar() {
    let (c, salida, _) = correr(&["probar", &ejemplo(), "--json"], "");
    assert_eq!(c, SALIDA_OK);
    let v: serde_json::Value = serde_json::from_str(&salida).unwrap();
    assert_eq!(v["formato"], "lorenz-resultados-0.1");
    assert_eq!(v["resumen"]["pasan"], 5);
}

#[test]
fn guardar_el_json_y_ver_el_texto() {
    let archivo = temporal("res.json", "");
    let (c, salida, _) = correr(&["probar", &ejemplo(), "--salida", &archivo], "");
    assert_eq!(c, SALIDA_OK);
    assert!(salida.contains("5 pruebas"), "{salida}");
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&archivo).unwrap()).unwrap();
    assert_eq!(v["resumen"]["total"], 5);
}

#[test]
fn filtrar_por_nombre_y_etiqueta() {
    let (_, salida, _) = correr(&["probar", &ejemplo(), "--nombre", "suma"], "");
    assert!(
        salida.contains("1 pruebas") && salida.contains("4 omitidas"),
        "{salida}"
    );
    let (_, salida, _) = correr(&["probar", &ejemplo(), "--etiqueta", "memoria"], "");
    assert!(salida.contains("2 pruebas"), "{salida}");
}

#[test]
fn hilos_explicitos() {
    let (c, salida, _) = correr(&["probar", &ejemplo(), "--hilos", "3"], "");
    assert_eq!(c, SALIDA_OK, "{salida}");
    let (c, _, err) = correr(&["probar", &ejemplo(), "--hilos", "x"], "");
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(err.contains("--hilos"), "{err}");
}

#[test]
fn argumentos_mal_escritos() {
    assert_eq!(correr(&["probar"], "").0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["probar", "a", "b"], "").0, SALIDA_ERROR_USO);
    assert_eq!(
        correr(&["probar", &ejemplo(), "--nada"], "").0,
        SALIDA_ERROR_USO
    );
    assert_eq!(
        correr(&["probar", &ejemplo(), "--nombre"], "").0,
        SALIDA_ERROR_USO
    );
}

#[test]
fn abrir_el_depurador_donde_fallo() {
    let p = proyecto_con_fallo("dep");
    let (c, salida, _) = correr(
        &["probar", &p, "--depurar", "mal"],
        "registros\natras 2\nregistros\nfin\n",
    );
    assert_eq!(c, 1);
    assert!(salida.contains("FALLA  mal"), "{salida}");
    assert!(
        salida.contains("depurando `mal` en el estado en que fallo"),
        "{salida}"
    );
    // Al final A vale 44; dos pasos antes (instante 2) valia 200.
    assert!(salida.contains("A = 2c"), "{salida}");
    assert!(salida.contains("A = c8"), "{salida}");
}

#[test]
fn depurar_una_prueba_que_pasa() {
    let p = proyecto_con_fallo("dep2");
    let (c, salida, _) = correr(&["probar", &p, "--depurar", "bien"], "");
    assert_eq!(c, SALIDA_OK);
    assert!(salida.contains("nada que depurar"), "{salida}");
}

#[test]
fn depurar_una_prueba_que_no_existe() {
    let p = proyecto_con_fallo("dep3");
    let (c, _, err) = correr(&["probar", &p, "--depurar", "nada"], "");
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(err.contains("no tiene una prueba llamada `nada`"), "{err}");
}

#[test]
fn funciona_sin_entorno_grafico() {
    // Nada de lo anterior abre ventanas ni necesita DISPLAY.
    std::env::remove_var("DISPLAY");
    std::env::remove_var("WAYLAND_DISPLAY");
    let (c, _, _) = correr(&["probar", &ejemplo()], "");
    assert_eq!(c, SALIDA_OK);
}

#[test]
fn con_cobertura_el_informe_cuenta_instrucciones_y_nombra_las_que_faltan() {
    let (codigo, salida, errores) = correr(&["probar", &ejemplo(), "--cobertura"], "");
    assert_eq!(codigo, SALIDA_OK, "{errores}\n{salida}");
    assert!(salida.contains("cobertura de instrucciones:"), "{salida}");
    assert!(salida.contains("sin ejercitar:"), "{salida}");
    assert!(salida.contains("cobertura de ramas:"), "{salida}");
}

#[test]
fn sin_la_opcion_no_hay_cobertura() {
    let (_, salida, _) = correr(&["probar", &ejemplo()], "");
    assert!(!salida.contains("cobertura"));
}

#[test]
fn la_cobertura_tambien_sale_en_json() {
    let (codigo, salida, _) = correr(&["probar", &ejemplo(), "--cobertura", "--json"], "");
    assert_eq!(codigo, SALIDA_OK);
    let v: serde_json::Value = serde_json::from_str(&salida).unwrap();
    assert!(v["cobertura"]["instrucciones"].as_array().unwrap().len() > 5);
}

/// Un proyecto de una sola prueba cuyo programa se puede cambiar entre ejecuciones.
fn proyecto_variable(nombre: &str, programa: &str) -> (String, PathBuf) {
    let dir = std::env::temp_dir().join(format!("lorenz-probar-{}-{nombre}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("p.s"), programa).unwrap();
    let m = raiz().join("ejemplos/ejemplo-8/maquina.toml");
    let ruta = dir.join("pruebas.toml");
    std::fs::write(
        &ruta,
        format!(
            "formato = \"lorenz-pruebas-0.1\"\nmaquina = {:?}\n\
             [[prueba]]\nnombre = \"suma\"\nprograma = \"p.s\"\n[prueba.espera]\nregistros = {{ A = 44 }}\n",
            m.to_string_lossy()
        ),
    )
    .unwrap();
    (ruta.to_string_lossy().into_owned(), dir.join("p.s"))
}

const SUMA: &str = "LDI A, 200\nLDI B, 100\nADD A, B\nHLT\n";

#[test]
fn con_historial_la_primera_vez_guarda_y_avisa_que_no_hay_con_que_comparar() {
    let (p, _) = proyecto_variable("hist1", SUMA);
    let (c, salida, errores) = correr(&["probar", &p, "--historial"], "");
    assert_eq!(c, 0, "{salida}");
    assert!(
        errores.contains("no hay una ejecucion anterior"),
        "{errores}"
    );
    assert!(!salida.contains("comparacion con la ejecucion anterior"));
    assert!(PathBuf::from(&p)
        .parent()
        .unwrap()
        .join(".lorenz/ultimo.json")
        .exists());
}

#[test]
fn con_historial_la_segunda_vez_compara_y_no_ve_cambios() {
    let (p, _) = proyecto_variable("hist2", SUMA);
    correr(&["probar", &p, "--historial"], "");
    let (c, salida, _) = correr(&["probar", &p, "--historial"], "");
    assert_eq!(c, 0, "{salida}");
    assert!(
        salida.contains("comparacion con la ejecucion anterior"),
        "{salida}"
    );
    assert!(salida.contains("1 sin cambios; sin regresion"), "{salida}");
}

#[test]
fn un_programa_mas_lento_devuelve_seis() {
    let (p, fuente) = proyecto_variable("lenta", SUMA);
    correr(&["probar", &p, "--historial"], "");
    std::fs::write(&fuente, "LDI A, 200\nNOP\nNOP\nLDI B, 100\nADD A, B\nHLT\n").unwrap();
    let (c, salida, _) = correr(&["probar", &p, "--historial"], "");
    assert_eq!(c, 6, "{salida}");
    assert!(salida.contains("MAS LENTA  suma:"), "{salida}");
    // Con un umbral grande el aumento se tolera.
    let (c, _, _) = correr(
        &[
            "probar",
            &p,
            "--comparar",
            &historial(&p),
            "--umbral",
            "1000",
        ],
        "",
    );
    assert_eq!(c, 0);
}

fn historial(pruebas: &str) -> String {
    lorenz_cli::probar::ruta_del_historial(pruebas)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn una_prueba_rota_devuelve_uno_y_dice_cual() {
    let (p, fuente) = proyecto_variable("rota", SUMA);
    correr(&["probar", &p, "--historial"], "");
    std::fs::write(&fuente, "LDI A, 1\nHLT\n").unwrap();
    let (c, salida, _) = correr(&["probar", &p, "--historial"], "");
    assert_eq!(c, 1, "{salida}");
    assert!(
        salida.contains("SE ROMPE   suma: pasaba y ahora falla"),
        "{salida}"
    );
}

#[test]
fn comparar_con_un_archivo_no_toca_el_historial() {
    let (p, _) = proyecto_variable("archivo", SUMA);
    let base = temporal("base-archivo.json", "");
    correr(&["probar", &p, "--salida", &base], "");
    let (c, salida, _) = correr(&["probar", &p, "--comparar", &base], "");
    assert_eq!(c, 0, "{salida}");
    assert!(salida.contains("sin regresion"), "{salida}");
    assert!(!PathBuf::from(historial(&p)).exists());
}

#[test]
fn el_json_lleva_la_comparacion() {
    let (p, fuente) = proyecto_variable("jsonreg", SUMA);
    let base = temporal("base-json.json", "");
    correr(&["probar", &p, "--salida", &base], "");
    std::fs::write(&fuente, "LDI A, 200\nNOP\nLDI B, 100\nADD A, B\nHLT\n").unwrap();
    let (c, salida, _) = correr(&["probar", &p, "--comparar", &base, "--json"], "");
    assert_eq!(c, 6);
    let v: serde_json::Value = serde_json::from_str(&salida).unwrap();
    assert_eq!(v["regresion"]["cambios"][0]["tipo"], "mas_lenta");
}

#[test]
fn un_archivo_de_comparacion_ilegible_es_un_error_de_uso() {
    let (p, _) = proyecto_variable("ilegible", SUMA);
    let (c, _, errores) = correr(&["probar", &p, "--comparar", "no-existe.json"], "");
    assert_eq!(c, 2);
    assert!(errores.contains("no-existe.json"));
    let malo = temporal("malo.json", "{ no es json");
    let (c, _, _) = correr(&["probar", &p, "--comparar", &malo], "");
    assert_eq!(c, 2);
    let (c, _, errores) = correr(&["probar", &p, "--umbral", "mucho"], "");
    assert_eq!(c, 2);
    assert!(errores.contains("--umbral"));
}

#[test]
fn filtrar_no_hace_desaparecer_las_demas_pruebas() {
    let p = proyecto_con_fallo("filtro");
    correr(&["probar", &p, "--historial"], "");
    let (_, salida, _) = correr(&["probar", &p, "--historial", "--nombre", "bien"], "");
    assert!(!salida.contains("desaparece"), "{salida}");
}
