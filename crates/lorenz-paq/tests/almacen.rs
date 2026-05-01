use lorenz_paq::*;
use std::path::PathBuf;

static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn carpeta(n: &str) -> PathBuf {
    let k = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("lorenz-alm-{}-{k}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn paq(nombre: &str, version: &str, deps: &[(&str, &str)]) -> Paquete {
    let d = carpeta(&format!("src-{nombre}-{version}"));
    let mut m = format!("[paquete]\nnombre = \"{nombre}\"\nversion = \"{version}\"\nautor = \"A\"\nlicencia = \"MIT\"\ntipo = \"biblioteca\"\n");
    if !deps.is_empty() {
        m.push_str("[dependencias]\n");
        for (n, r) in deps {
            m.push_str(&format!("{n} = \"{r}\"\n"));
        }
    }
    std::fs::write(d.join("paquete.toml"), m).unwrap();
    std::fs::write(d.join("datos.txt"), format!("{nombre} {version}")).unwrap();
    Paquete::desde_carpeta(&d).unwrap()
}

fn publicar(repo: &std::path::Path, p: &Paquete) {
    let m = &p.manifiesto;
    std::fs::write(
        repo.join(format!("{}-{}.lpkg", m.nombre, m.version)),
        p.a_bytes(),
    )
    .unwrap();
}

#[test]
fn instala_y_lista() {
    let alm = Almacen::nuevo(carpeta("a1"));
    let inf = alm.instalar(&paq("base", "1.0.0", &[]), &[]).unwrap();
    assert_eq!(inf.instalados.len(), 1);
    let v = Version::parsear("1.0.0").unwrap();
    assert_eq!(
        std::fs::read_to_string(alm.carpeta_de("base", v).join("datos.txt")).unwrap(),
        "base 1.0.0"
    );
    assert_eq!(alm.instalados().len(), 1);
    // repetir no hace nada
    let inf = alm.instalar(&paq("base", "1.0.0", &[]), &[]).unwrap();
    assert!(inf.instalados.is_empty() && inf.ya_estaban.len() == 1);
}

#[test]
fn resuelve_dependencias_con_la_version_mas_alta() {
    let repo = carpeta("r2");
    publicar(&repo, &paq("base", "1.0.0", &[]));
    publicar(&repo, &paq("base", "1.4.0", &[]));
    publicar(&repo, &paq("base", "2.0.0", &[]));
    let alm = Almacen::nuevo(carpeta("a2"));
    let inf = alm
        .instalar(&paq("cpu", "0.1.0", &[("base", "^1.0")]), &[repo])
        .unwrap();
    assert_eq!(inf.instalados[0].0, "base");
    assert_eq!(inf.instalados[0].1.to_string(), "1.4.0");
    assert_eq!(inf.instalados[1].0, "cpu");
}

#[test]
fn dependencia_transitiva_y_ya_instalada() {
    let repo = carpeta("r3");
    publicar(&repo, &paq("b", "1.0.0", &[("c", "*")]));
    publicar(&repo, &paq("c", "1.0.0", &[]));
    let alm = Almacen::nuevo(carpeta("a3"));
    let inf = alm
        .instalar(
            &paq("a", "1.0.0", &[("b", "*")]),
            std::slice::from_ref(&repo),
        )
        .unwrap();
    let orden: Vec<_> = inf.instalados.iter().map(|x| x.0.as_str()).collect();
    assert_eq!(orden, ["c", "b", "a"]);
    let inf = alm
        .instalar(&paq("d", "1.0.0", &[("b", "*")]), &[])
        .unwrap();
    assert_eq!(inf.ya_estaban.len(), 1);
}

#[test]
fn falta_y_ciclo() {
    let alm = Almacen::nuevo(carpeta("a4"));
    let e = alm
        .instalar(&paq("x", "1.0.0", &[("nada", ">=1")]), &[])
        .unwrap_err()
        .to_string();
    assert!(e.contains("falta `nada`"), "{e}");
    assert!(alm.instalados().is_empty());

    let repo = carpeta("r4");
    publicar(&repo, &paq("p", "1.0.0", &[("q", "*")]));
    publicar(&repo, &paq("q", "1.0.0", &[("p", "*")]));
    let e = alm
        .instalar(&paq("p", "1.0.0", &[("q", "*")]), &[repo])
        .unwrap_err()
        .to_string();
    assert!(e.contains("ciclo"), "{e}");
}

#[test]
fn version_que_no_cumple_no_sirve() {
    let repo = carpeta("r5");
    publicar(&repo, &paq("base", "1.0.0", &[]));
    let alm = Almacen::nuevo(carpeta("a5"));
    assert!(alm
        .instalar(&paq("cpu", "1.0.0", &[("base", ">=2.0")]), &[repo])
        .is_err());
}
