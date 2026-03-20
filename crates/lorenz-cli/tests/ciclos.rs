//! La orden `ciclos` (RF-CIC-001, RF-CIC-002, RF-CIC-012, RF-CIC-025, RF-CIC-026, RF-CIC-031).

use std::path::PathBuf;

use lorenz_cli::{ejecutar_cli, SALIDA_ERROR_USO, SALIDA_LIMITE, SALIDA_OK};

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn ruta(r: &str) -> String {
    raiz().join(r).to_string_lossy().into_owned()
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

fn maquina() -> String {
    ruta("ejemplos/ejemplo-8-ciclos/maquina.toml")
}

fn suma() -> String {
    ruta("ejemplos/ejemplo-8-ciclos/programas/suma.s")
}

fn bucle() -> String {
    ruta("ejemplos/ejemplo-8-ciclos/programas/bucle.s")
}

#[test]
fn cuenta_ciclos_instrucciones_y_tamano_del_codigo() {
    let (c, s, e) = correr(&["ciclos", &maquina(), &suma()]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.contains("ciclos: "), "{s}");
    assert!(s.contains("instrucciones: 4"), "{s}");
    assert!(s.contains("tamano del codigo: 6 bytes"), "{s}");
    assert!(s.contains("ciclos por instruccion: "), "{s}");
}

#[test]
fn el_mismo_programa_y_la_misma_descripcion_sirven_en_los_dos_modos() {
    let (c1, _, e1) = correr(&["ejecutar", &maquina(), &suma()]);
    let (c2, _, e2) = correr(&["ciclos", &maquina(), &suma()]);
    assert_eq!((c1, c2), (SALIDA_OK, SALIDA_OK), "{e1}{e2}");
}

#[test]
fn la_vista_muestra_el_pipeline_ciclo_a_ciclo() {
    let (c, s, _) = correr(&["ciclos", &maquina(), &suma(), "--vista", "6"]);
    assert_eq!(c, SALIDA_OK);
    assert!(s.contains("ciclo    0 | IF LDI@0x0000"), "{s}");
    assert!(s.contains("| ID "), "{s}");
}

#[test]
fn el_diagrama_dibuja_una_fila_por_instruccion() {
    let (_, s, _) = correr(&["ciclos", &maquina(), &suma(), "--diagrama", "4"]);
    assert!(s.contains("instruccion"), "{s}");
    assert!(s.contains("ADD@0x0004"), "{s}");
}

#[test]
fn el_perfil_atribuye_ciclos_a_cada_instruccion() {
    let (_, s, _) = correr(&["ciclos", &maquina(), &suma(), "--perfil"]);
    assert!(
        s.contains("# direccion instruccion ejecuciones ciclos"),
        "{s}"
    );
    let total: u64 = s
        .lines()
        .skip_while(|l| !l.starts_with("# direccion"))
        .skip(1)
        .map(|l| l.split_whitespace().last().unwrap().parse::<u64>().unwrap())
        .sum();
    let ciclos: u64 = s
        .lines()
        .find_map(|l| l.strip_prefix("ciclos: "))
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(total, ciclos, "{s}");
}

#[test]
fn un_bucle_infinito_llega_al_limite_y_cuenta_los_saltos() {
    let (c, s, _) = correr(&["ciclos", &maquina(), &bucle(), "--limite", "100"]);
    assert_eq!(c, SALIDA_LIMITE, "{s}");
    assert!(s.contains("saltos: 50"), "{s}");
}

#[test]
fn el_volcado_de_la_cache_se_pide_aparte() {
    let (_, s, _) = correr(&["ciclos", &maquina(), &suma(), "--cache"]);
    assert!(
        s.contains("# conjunto via valida sucia etiqueta direccion"),
        "{s}"
    );
}

#[test]
fn argumentos_incorrectos() {
    let (c, _, _) = correr(&["ciclos", &maquina()]);
    assert_eq!(c, SALIDA_ERROR_USO);
    let (c, _, e) = correr(&["ciclos", "no-existe.toml", &suma()]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("no-existe.toml"), "{e}");
}

fn temporal(nombre: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("lorenz-ciclos-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d.join(nombre)
}

#[test]
fn el_mapa_de_calor_pone_ciclos_a_cada_linea_del_fuente() {
    let (c, s, e) = correr(&["ciclos", &maquina(), &suma(), "--calor"]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.contains("# mapa de calor:"), "{s}");
    let con_ciclos = s
        .lines()
        .filter(|l| l.contains('#') && l.contains('|'))
        .count();
    assert!(con_ciclos >= 3, "{s}");
    assert!(s.contains("ADD A, B"), "{s}");
}

#[test]
fn el_mapa_de_calor_en_html_se_escribe_en_un_archivo() {
    let html = temporal("calor.html");
    let (c, _, e) = correr(&[
        "ciclos",
        &maquina(),
        &suma(),
        "--calor-html",
        &html.to_string_lossy(),
    ]);
    assert_eq!(c, SALIDA_OK, "{e}");
    let t = std::fs::read_to_string(html).unwrap();
    assert!(t.contains("<table>") && t.contains("rgba(220,60,40"), "{t}");
}

#[test]
fn sin_fuente_no_hay_mapa_de_calor() {
    let hex = ruta("ejemplos/ejemplo-8/programas/suma.hex");
    let (c, _, e) = correr(&["ciclos", &maquina(), &hex, "--calor"]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("no tiene fuente"), "{e}");
}

#[test]
fn comparar_dos_descripciones_muestra_las_cifras_lado_a_lado() {
    let (c, s, e) = correr(&[
        "comparar-ciclos",
        &ruta("ejemplos/ejemplo-8/maquina.toml"),
        &maquina(),
        &suma(),
    ]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.contains("ejemplo-8-ciclos"), "{s}");
    assert!(s.contains("ciclos por instruccion"), "{s}");
    assert!(
        s.lines()
            .any(|l| l.starts_with("ciclos ") && l.contains('%')),
        "{s}"
    );
}

#[test]
fn el_historial_guarda_una_medicion_por_version_de_la_arquitectura() {
    let h = temporal("historial.toml");
    let _ = std::fs::remove_file(&h);
    let hs = h.to_string_lossy().into_owned();
    for etiqueta in ["base", "otra"] {
        let (c, s, e) = correr(&[
            "ciclos",
            &maquina(),
            &suma(),
            "--historial",
            &hs,
            "--etiqueta",
            etiqueta,
        ]);
        assert_eq!(c, SALIDA_OK, "{e}");
        assert!(s.contains("medicion anadida"), "{s}");
    }
    let (c, s, _) = correr(&["historial-ciclos", &hs]);
    assert_eq!(c, SALIDA_OK);
    let filas: Vec<_> = s.lines().skip(1).collect();
    assert_eq!(filas.len(), 2, "{s}");
    assert!(
        filas[0].ends_with(" base") && filas[1].ends_with(" otra"),
        "{s}"
    );
    // La misma descripcion da la misma version.
    let version = |f: &str| f.split_whitespace().nth(1).unwrap().to_string();
    assert_eq!(version(filas[0]), version(filas[1]));
}

#[test]
fn cambiar_la_arquitectura_cambia_la_version() {
    let a = lorenz_cli::ciclos::version_de(&maquina()).unwrap();
    let texto = std::fs::read_to_string(maquina())
        .unwrap()
        .replace("latencia = 4", "latencia = 5");
    let otra = temporal("otra.toml");
    std::fs::write(&otra, texto).unwrap();
    let b = lorenz_cli::ciclos::version_de(&otra.to_string_lossy()).unwrap();
    assert_ne!(a, b);
    // Y un comentario no la cambia.
    let con_comentario = temporal("comentada.toml");
    std::fs::write(
        &con_comentario,
        format!("# nota\n{}", std::fs::read_to_string(maquina()).unwrap()),
    )
    .unwrap();
    assert_eq!(
        a,
        lorenz_cli::ciclos::version_de(&con_comentario.to_string_lossy()).unwrap()
    );
}

#[test]
fn el_vigilante_recalcula_solo_cuando_algo_cambia() {
    use lorenz_cli::ciclos::{OpcionesCiclos, VigilanteCiclos};
    let dir = temporal("vigilar");
    std::fs::create_dir_all(&dir).unwrap();
    let m = dir.join("maquina.toml");
    let p = dir.join("suma.s");
    std::fs::copy(maquina(), &m).unwrap();
    std::fs::copy(suma(), &p).unwrap();
    let (ms, ps) = (
        m.to_string_lossy().into_owned(),
        p.to_string_lossy().into_owned(),
    );
    let o = OpcionesCiclos::default();
    let mut v = VigilanteCiclos::default();
    let primera = v
        .pasar(&ms, &ps, &o)
        .expect("la primera vez simula")
        .ok()
        .unwrap();
    let antes = primera.sim.metricas().ciclos;
    assert!(v.pasar(&ms, &ps, &o).is_none(), "sin cambios no repite");
    let nuevo = std::fs::read_to_string(&m)
        .unwrap()
        .replace("latencia = 2", "latencia = 7");
    std::fs::write(&m, nuevo).unwrap();
    let segunda = v
        .pasar(&ms, &ps, &o)
        .expect("cambio la descripcion")
        .ok()
        .unwrap();
    assert!(
        segunda.sim.metricas().ciclos > antes,
        "una rom mas lenta tarda mas"
    );
    assert_ne!(primera.version, segunda.version);
}

#[test]
fn un_bus_compartido_muestra_sus_esperas_y_solo_entonces() {
    let programa = temporal("cargas.s");
    std::fs::write(
        &programa,
        "LD A, [0x2000]\nLD B, [0x2010]\nLD C, [0x2020]\nLD D, [0x2030]\nHLT\n",
    )
    .unwrap();
    let programa = programa.to_string_lossy().into_owned();
    let (_, sin, _) = correr(&["ciclos", &maquina(), &programa]);
    assert!(!sin.contains("bus compartido"), "{sin}");
    let base = std::fs::read_to_string(maquina()).unwrap();
    let con = temporal("con-bus.toml");
    std::fs::write(&con, format!("{base}\n[bus]\ncompartido = true\n")).unwrap();
    let con = con.to_string_lossy().into_owned();
    let (c, s, e) = correr(&["ciclos", &con, &programa]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.contains("esperas por el bus compartido: "), "{s}");
    let (_, tabla, _) = correr(&["comparar-ciclos", &maquina(), &con, &programa]);
    assert!(tabla.contains("esperas por el bus"), "{tabla}");
}
