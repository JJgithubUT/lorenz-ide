//! `lorenz proyecto` y `lorenz tarea` (RF-IDE-001 a RF-IDE-006).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use lorenz_cli::proyecto::Abierto;
use lorenz_cli::{ejecutar_cli, SALIDA_ERROR_USO, SALIDA_OK};

static N: AtomicUsize = AtomicUsize::new(0);

fn ejemplo(r: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../ejemplos")
        .join(r)
}

fn carpeta() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "lorenz-proy-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn escribir(d: &Path, r: &str, t: &str) {
    let f = d.join(r);
    std::fs::create_dir_all(f.parent().unwrap()).unwrap();
    std::fs::write(f, t).unwrap();
}

fn copiar(de: &str, d: &Path, a: &str) {
    let f = d.join(a);
    std::fs::create_dir_all(f.parent().unwrap()).unwrap();
    std::fs::copy(ejemplo(de), f).unwrap();
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

/// Un proyecto de una sola fuente sobre `ejemplo-8`, con el `proyecto.toml` dado.
fn proyecto(toml: &str) -> PathBuf {
    let d = carpeta();
    copiar("ejemplo-8/maquina.toml", &d, "maquina.toml");
    escribir(&d, "uno.s", "LDI A, 7\nHLT\n");
    escribir(&d, "proyecto.toml", toml);
    d
}

const BASE: &str = "[proyecto]\nnombre = \"p\"\n[[arquitectura]]\nnombre = \"a\"\nruta = \"maquina.toml\"\n[[fuente]]\nruta = \"uno.s\"\n";

#[test]
fn el_ejemplo_del_repositorio_se_construye_y_se_ejecuta() {
    let ruta = ejemplo("proyecto-8");
    let ruta = ruta.to_str().unwrap();
    let (c, s, e) = correr(&["proyecto", ruta]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(
        s.contains("* ejemplo-8") && s.contains("correr: construir -> ejecutar"),
        "{s}"
    );
    let (c, s, e) = correr(&["tarea", ruta]);
    assert_eq!(
        (c, s.contains("construir: ensamblar")),
        (SALIDA_OK, true),
        "{e}"
    );
}

#[test]
fn una_tarea_construye_y_ejecuta() {
    let d = proyecto(&format!("{BASE}[[tarea]]\nnombre = \"correr\"\npasos = [\"ensamblar\", \"ejecutar\"]\nlimite = 50\n"));
    let (c, s, e) = correr(&["tarea", d.to_str().unwrap(), "correr"]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(
        s.contains("> ensamblar") && s.contains("> ejecutar") && s.contains("A=0x7"),
        "{s}"
    );
    assert!(d.join("construccion/p.hex").is_file());
}

#[test]
fn varios_fuentes_se_enlazan_en_orden() {
    let d = carpeta();
    copiar("ejemplo-8/maquina.toml", &d, "maquina.toml");
    escribir(&d, "a.s", ".externo v\n.org 0x10\nLDI A, v\nHLT\n");
    escribir(&d, "b.s", "v = 42\n");
    escribir(
        &d,
        "proyecto.toml",
        &format!(
            "{}[[fuente]]\nruta = \"b.s\"\n[[tarea]]\nnombre = \"t\"\npasos = [\"ensamblar\", \"ejecutar\"]\n",
            BASE.replace("uno.s", "a.s")
        ),
    );
    let (c, s, e) = correr(&["tarea", d.to_str().unwrap(), "t"]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(
        s.contains("2 objeto(s) enlazados") && s.contains("A=0x2a"),
        "{s}"
    );
}

#[test]
fn sin_nombre_lista_las_tareas() {
    let d = proyecto(&format!(
        "{BASE}[[tarea]]\nnombre = \"uno\"\ndescripcion = \"hola\"\npasos = [\"ensamblar\"]\n"
    ));
    let (c, s, _) = correr(&["tarea", d.to_str().unwrap()]);
    assert_eq!(c, SALIDA_OK);
    assert_eq!(s, "uno: ensamblar  # hola\n");
    let d = proyecto(BASE);
    assert!(correr(&["tarea", d.to_str().unwrap()])
        .1
        .contains("no declara tareas"));
}

#[test]
fn una_tarea_que_no_existe_o_un_paso_que_falla_dan_error() {
    let d = proyecto(&format!(
        "{BASE}[[tarea]]\nnombre = \"sola\"\npasos = [\"ejecutar\"]\n"
    ));
    let ruta = d.to_str().unwrap();
    let (c, _, e) = correr(&["tarea", ruta, "nada"]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("`nada` no existe") && e.contains("sola"), "{e}");
    // `ejecutar` sin haber construido antes.
    let (c, _, e) = correr(&["tarea", ruta, "sola"]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(
        e.contains("pon `ensamblar` antes") && e.contains("se detuvo en `ejecutar`"),
        "{e}"
    );
}

#[test]
fn comprueba_fuentes_y_dispositivos() {
    let d = proyecto(&BASE.replace("uno.s", "falta.s"));
    let e = Abierto::abrir(d.to_str().unwrap())
        .unwrap()
        .comprobar(None)
        .unwrap_err();
    assert!(e.contains("fuente `falta.s`: no existe"), "{e}");

    let d = proyecto(&format!("{BASE}[[dispositivo]]\nnombre = \"nada\"\n"));
    let e = Abierto::abrir(d.to_str().unwrap())
        .unwrap()
        .comprobar(None)
        .unwrap_err();
    assert!(
        e.contains("dispositivo `nada`") && e.contains("no lo tiene"),
        "{e}"
    );

    let d = carpeta();
    copiar("so-ejemplo/maquina.toml", &d, "maquina.toml");
    escribir(&d, "uno.s", "HLT\n");
    escribir(
        &d,
        "proyecto.toml",
        &format!("{BASE}[[dispositivo]]\nnombre = \"pantalla\"\nentrada = \"x\"\n"),
    );
    let e = Abierto::abrir(d.to_str().unwrap())
        .unwrap()
        .comprobar(None)
        .unwrap_err();
    assert!(e.contains("solo los teclados y las lineas serie"), "{e}");
}

#[test]
fn la_entrada_de_los_dispositivos_llega_al_ejecutar() {
    let d = carpeta();
    copiar("ejemplo-8-serie/maquina.toml", &d, "maquina.toml");
    let (c, _, e) = correr(&["validar", d.join("maquina.toml").to_str().unwrap()]);
    assert_eq!(c, SALIDA_OK, "{e}");
    escribir(&d, "uno.s", "HLT\n");
    escribir(
        &d,
        "proyecto.toml",
        &format!("{BASE}[[dispositivo]]\nnombre = \"teclado\"\nentrada = \"abc\"\n[[tarea]]\nnombre = \"t\"\npasos = [\"ensamblar\", \"ejecutar\"]\n"),
    );
    let (c, _, e) = correr(&["tarea", d.to_str().unwrap(), "t"]);
    assert_eq!(c, SALIDA_OK, "{e}");
}

#[test]
fn elige_la_arquitectura_activa() {
    let d = carpeta();
    copiar("ejemplo-8/maquina.toml", &d, "a/maquina.toml");
    copiar("ejemplo-8/maquina.toml", &d, "b/maquina.toml");
    escribir(&d, "uno.s", "LDI A, 9\nHLT\n");
    escribir(
        &d,
        "proyecto.toml",
        "[proyecto]\nnombre = \"p\"\narquitectura = \"a\"\n[[arquitectura]]\nnombre = \"a\"\nruta = \"a/maquina.toml\"\n[[arquitectura]]\nnombre = \"b\"\nruta = \"b/maquina.toml\"\n[[fuente]]\nruta = \"uno.s\"\n[[tarea]]\nnombre = \"t\"\npasos = [\"ensamblar\", \"ejecutar\"]\n",
    );
    let ruta = d.to_str().unwrap();
    let (c, s, _) = correr(&["proyecto", ruta, "--arquitectura", "b"]);
    assert_eq!(c, SALIDA_OK);
    assert!(
        s.contains("* b (b/maquina.toml)") && s.contains("  a (a/maquina.toml)"),
        "{s}"
    );
    assert_eq!(
        correr(&["tarea", ruta, "t", "--arquitectura", "b"]).0,
        SALIDA_OK
    );
    let (c, _, e) = correr(&["proyecto", ruta, "--arquitectura", "z"]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("`z` no esta declarada"), "{e}");
}

#[test]
fn una_arquitectura_secundaria_rota_solo_avisa() {
    let d = carpeta();
    copiar("ejemplo-8/maquina.toml", &d, "a/maquina.toml");
    escribir(&d, "b/maquina.toml", "esto no es una maquina");
    escribir(&d, "uno.s", "HLT\n");
    escribir(
        &d,
        "proyecto.toml",
        "[proyecto]\nnombre = \"p\"\narquitectura = \"a\"\n[[arquitectura]]\nnombre = \"a\"\nruta = \"a/maquina.toml\"\n[[arquitectura]]\nnombre = \"b\"\nruta = \"b/maquina.toml\"\n[[fuente]]\nruta = \"uno.s\"\n",
    );
    let (c, _, e) = correr(&["proyecto", d.to_str().unwrap()]);
    assert_eq!(c, SALIDA_OK);
    assert!(e.contains("aviso: arquitectura `b`"), "{e}");
}

#[test]
fn un_proyecto_sgt_compila_el_ultimo_modulo() {
    let d = carpeta();
    copiar("sgt-8/maquina.toml", &d, "maquina.toml");
    escribir(
        &d,
        "principal.sgt",
        "var x: u8;\nfuncion principal() {\n    x = 5;\n}\n",
    );
    escribir(
        &d,
        "proyecto.toml",
        "[proyecto]\nnombre = \"s\"\n[[arquitectura]]\nnombre = \"a\"\nruta = \"maquina.toml\"\n[[fuente]]\nruta = \"principal.sgt\"\n[[tarea]]\nnombre = \"t\"\npasos = [\"ensamblar\", \"ejecutar\"]\n",
    );
    let (c, s, e) = correr(&["tarea", d.to_str().unwrap(), "t"]);
    assert_eq!(c, SALIDA_OK, "{e}{s}");
    assert!(d.join("construccion/s.hex").is_file());
}

#[test]
fn no_mezcla_lenguajes_ni_acepta_un_proyecto_sin_fuentes() {
    let d = proyecto(&format!(
        "{BASE}[[fuente]]\nruta = \"x.sgt\"\n[[tarea]]\nnombre = \"t\"\npasos = [\"ensamblar\"]\n"
    ));
    escribir(&d, "x.sgt", "funcion principal() {}\n");
    let (c, _, e) = correr(&["tarea", d.to_str().unwrap(), "t"]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("mezclan"), "{e}");
    let sin_fuentes = BASE.replace("[[fuente]]\nruta = \"uno.s\"\n", "");
    let d = proyecto(&format!(
        "{sin_fuentes}[[tarea]]\nnombre = \"t\"\npasos = [\"ensamblar\"]\n"
    ));
    let (c, _, e) = correr(&["tarea", d.to_str().unwrap(), "t"]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("ninguna `[[fuente]]`"), "{e}");
}

#[test]
fn probar_usa_el_archivo_de_pruebas_del_proyecto() {
    let d = carpeta();
    copiar("ejemplo-8/maquina.toml", &d, "maquina.toml");
    escribir(
        &d,
        "pruebas.toml",
        "formato = \"lorenz-pruebas-0.1\"\nmaquina = \"maquina.toml\"\n[[prueba]]\nnombre = \"suma\"\nprograma = \"programas/suma.s\"\nlimite = 100\n[prueba.espera]\nregistros = { A = 44 }\n",
    );
    copiar("ejemplo-8/programas/suma.s", &d, "programas/suma.s");
    escribir(
        &d,
        "proyecto.toml",
        "[proyecto]\nnombre = \"p\"\npruebas = \"pruebas.toml\"\n[[arquitectura]]\nnombre = \"a\"\nruta = \"maquina.toml\"\n[[fuente]]\nruta = \"programas/suma.s\"\n[[tarea]]\nnombre = \"verificar\"\npasos = [\"probar\"]\n[[tarea]]\nnombre = \"sin\"\npasos = [\"probar\"]\n",
    );
    let (c, s, e) = correr(&["tarea", d.to_str().unwrap(), "verificar"]);
    assert_eq!(c, SALIDA_OK, "{e}{s}");
    // Sin `pruebas` declaradas, el paso avisa.
    let t = std::fs::read_to_string(d.join("proyecto.toml"))
        .unwrap()
        .replace("pruebas = \"pruebas.toml\"\n", "");
    escribir(&d, "proyecto.toml", &t);
    let (c, _, e) = correr(&["tarea", d.to_str().unwrap(), "sin"]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("no declara `pruebas`"), "{e}");
}

#[test]
fn uso_incorrecto() {
    assert_eq!(correr(&["proyecto"]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["tarea"]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["proyecto", "a", "b"]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["tarea", "a", "b", "c"]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["proyecto", "--x"]).0, SALIDA_ERROR_USO);
    let (c, _, e) = correr(&["proyecto", "no-existe"]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("no se pudo leer"), "{e}");
}

#[test]
fn llegar_al_limite_solo_es_normal_si_la_tarea_lo_dice() {
    let infinito = "LDI A, 0\nLDI B, 0\nADD A, B\ninicio: JZ inicio\n";
    for (extra, esperado) in [
        ("", lorenz_cli::SALIDA_LIMITE),
        ("hasta_el_limite = true\n", SALIDA_OK),
    ] {
        let d = carpeta();
        copiar("ejemplo-8/maquina.toml", &d, "maquina.toml");
        escribir(&d, "uno.s", infinito);
        escribir(&d, "proyecto.toml", &format!("{BASE}[[tarea]]\nnombre = \"t\"\npasos = [\"ensamblar\", \"ejecutar\"]\nlimite = 100\n{extra}"));
        let (c, s, e) = correr(&["tarea", d.to_str().unwrap(), "t"]);
        assert_eq!(c, esperado, "{extra}: {s}{e}");
        assert_eq!(s.contains("como se esperaba"), esperado == SALIDA_OK);
    }
}
