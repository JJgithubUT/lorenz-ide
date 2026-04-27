//! La orden `paquete` (RF-ECO-001 a 003, 005 a 008).

use lorenz_cli::{ejecutar_cli, SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_OK};
use std::path::PathBuf;

static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

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

fn dir(n: &str) -> PathBuf {
    let k = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("lorenz-cli-paq-{}-{k}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn fuente(nombre: &str, version: &str, dep: Option<&str>) -> PathBuf {
    let d = dir(nombre);
    let mut m = format!("[paquete]\nnombre = \"{nombre}\"\nversion = \"{version}\"\nautor = \"Ana\"\nlicencia = \"MIT\"\ntipo = \"biblioteca\"\ndescripcion = \"prueba\"\n");
    if let Some(d) = dep {
        m.push_str(&format!("[dependencias]\n{d} = \">=1.0\"\n"));
    }
    std::fs::write(d.join("paquete.toml"), m).unwrap();
    std::fs::write(d.join("a.txt"), "hola").unwrap();
    d
}

#[test]
fn crear_verificar_info() {
    let f = fuente("base", "1.0.0", None);
    let out = dir("out").join("base.lpkg");
    let (c, s, _) = correr(&[
        "paquete",
        "crear",
        f.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(c, SALIDA_OK, "{s}");
    assert!(s.contains("base 1.0.0") && s.contains("2 archivos"));
    let (c, s, _) = correr(&["paquete", "verificar", out.to_str().unwrap()]);
    assert_eq!((c, s.contains("correcto")), (SALIDA_OK, true));
    let (c, s, _) = correr(&["paquete", "info", out.to_str().unwrap()]);
    assert_eq!(c, SALIDA_OK);
    assert!(s.contains("autor:     Ana") && s.contains("a.txt"), "{s}");
}

#[test]
fn verificar_detecta_un_paquete_alterado() {
    let f = fuente("base", "1.0.0", None);
    let out = dir("out").join("base.lpkg");
    correr(&[
        "paquete",
        "crear",
        f.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    let mut b = std::fs::read(&out).unwrap();
    let i = b.windows(4).position(|w| w == b"hola").unwrap();
    b[i] = b'H';
    std::fs::write(&out, b).unwrap();
    let (c, _, e) = correr(&["paquete", "verificar", out.to_str().unwrap()]);
    assert_eq!(c, SALIDA_ERROR_EJECUCION);
    assert!(e.contains("error"), "{e}");
}

#[test]
fn instalar_con_dependencia_y_listar() {
    let repo = dir("repo");
    let base = fuente("base", "1.2.0", None);
    let p = repo.join("base-1.2.0.lpkg");
    correr(&[
        "paquete",
        "crear",
        base.to_str().unwrap(),
        "-o",
        p.to_str().unwrap(),
    ]);
    let cpu = fuente("cpu", "0.1.0", Some("base"));
    let q = dir("o2").join("cpu.lpkg");
    correr(&[
        "paquete",
        "crear",
        cpu.to_str().unwrap(),
        "-o",
        q.to_str().unwrap(),
    ]);
    let alm = dir("almacen");
    let (c, s, e) = correr(&[
        "paquete",
        "instalar",
        q.to_str().unwrap(),
        "--en",
        alm.to_str().unwrap(),
        "--repositorio",
        repo.to_str().unwrap(),
    ]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(
        s.find("instalado base 1.2.0").unwrap() < s.find("instalado cpu 0.1.0").unwrap(),
        "{s}"
    );
    let (c, s, _) = correr(&["paquete", "listar", "--en", alm.to_str().unwrap()]);
    assert_eq!(c, SALIDA_OK);
    assert!(s.contains("base 1.2.0") && s.contains("cpu 0.1.0"), "{s}");
    // sin el repositorio no se puede instalar
    let alm2 = dir("almacen2");
    let (c, _, e) = correr(&[
        "paquete",
        "instalar",
        q.to_str().unwrap(),
        "--en",
        alm2.to_str().unwrap(),
    ]);
    assert_eq!(c, SALIDA_ERROR_EJECUCION);
    assert!(e.contains("falta `base`"), "{e}");
}

#[test]
fn uso_incorrecto() {
    assert_eq!(correr(&["paquete"]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["paquete", "volar"]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["paquete", "info"]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["paquete", "listar", "x"]).0, SALIDA_ERROR_USO);
    assert_eq!(
        correr(&["paquete", "listar", "-o", "x"]).0,
        SALIDA_ERROR_USO
    );
}
