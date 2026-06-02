//! La lista de proyectos recientes (RF-IDE-009).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use lorenz_cli::recientes::{Recientes, MAXIMO};
use lorenz_cli::{ejecutar_cli, SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_OK};

static N: AtomicUsize = AtomicUsize::new(0);

fn carpeta() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "lorenz-rec-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn proyecto(base: &Path, nombre: &str) -> PathBuf {
    let d = base.join(nombre);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::copy(
        "../../ejemplos/ejemplo-8/maquina.toml",
        d.join("maquina.toml"),
    )
    .unwrap();
    std::fs::write(d.join("uno.s"), "HLT\n").unwrap();
    std::fs::write(
        d.join("proyecto.toml"),
        format!("[proyecto]\nnombre = \"{nombre}\"\n[[arquitectura]]\nnombre = \"a\"\nruta = \"maquina.toml\"\n[[fuente]]\nruta = \"uno.s\"\n"),
    )
    .unwrap();
    d
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

#[test]
fn lo_mas_reciente_va_primero_y_no_se_repite() {
    let base = carpeta();
    let r = Recientes::en(base.join("r.toml"));
    let (a, b) = (proyecto(&base, "a"), proyecto(&base, "b"));
    r.agregar(&a).unwrap();
    r.agregar(&b).unwrap();
    r.agregar(&a.join("proyecto.toml")).unwrap();
    let nombres: Vec<_> = r.listar().into_iter().map(|p| p.nombre).collect();
    assert_eq!(nombres, ["a", "b"]);
    assert!(r.listar().iter().all(|p| p.existe));
}

#[test]
fn solo_guarda_los_ultimos() {
    let base = carpeta();
    let r = Recientes::en(base.join("r.toml"));
    for i in 0..MAXIMO + 3 {
        r.agregar(&proyecto(&base, &format!("p{i}"))).unwrap();
    }
    let l = r.listar();
    assert_eq!(l.len(), MAXIMO);
    assert_eq!(l[0].nombre, format!("p{}", MAXIMO + 2));
}

#[test]
fn un_proyecto_borrado_se_marca_y_una_lista_danada_se_ignora() {
    let base = carpeta();
    let r = Recientes::en(base.join("r.toml"));
    let a = proyecto(&base, "a");
    r.agregar(&a).unwrap();
    std::fs::remove_file(a.join("proyecto.toml")).unwrap();
    assert!(!r.listar()[0].existe);
    std::fs::write(base.join("r.toml"), "esto = no es toml [").unwrap();
    assert!(r.listar().is_empty());
    // Y se puede volver a escribir encima.
    r.agregar(&proyecto(&base, "b")).unwrap();
    assert_eq!(r.listar().len(), 1);
}

#[test]
fn olvidar_quita_una_o_todas() {
    let base = carpeta();
    let r = Recientes::en(base.join("r.toml"));
    for n in ["a", "b", "c"] {
        r.agregar(&proyecto(&base, n)).unwrap();
    }
    assert!(r.olvidar(2).unwrap());
    let nombres: Vec<_> = r.listar().into_iter().map(|p| p.nombre).collect();
    assert_eq!(nombres, ["c", "a"]);
    assert!(!r.olvidar(0).unwrap() && !r.olvidar(9).unwrap());
    r.olvidar_todo().unwrap();
    assert!(r.listar().is_empty());
}

#[test]
fn la_orden_lista_abre_y_olvida() {
    let base = carpeta();
    let lista = base.join("r.toml");
    let lista = lista.to_str().unwrap();
    let (c, s, _) = correr(&["recientes", "--en", lista]);
    assert_eq!((c, s.as_str()), (SALIDA_OK, "no hay proyectos recientes\n"));

    let r = Recientes::en(lista);
    r.agregar(&proyecto(&base, "uno")).unwrap();
    r.agregar(&proyecto(&base, "dos")).unwrap();
    let (c, s, _) = correr(&["recientes", "listar", "--en", lista]);
    assert_eq!(c, SALIDA_OK);
    assert!(s.starts_with(" 1  dos  ") && s.contains(" 2  uno  "), "{s}");

    // Abrir el 2 muestra el proyecto y lo sube al primer lugar.
    let (c, s, e) = correr(&["recientes", "abrir", "2", "--en", lista]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.starts_with("proyecto uno"), "{s}");
    assert_eq!(r.listar()[0].nombre, "uno");

    assert_eq!(
        correr(&["recientes", "olvidar", "1", "--en", lista]).0,
        SALIDA_OK
    );
    assert_eq!(r.listar().len(), 1);
    assert_eq!(
        correr(&["recientes", "olvidar", "todos", "--en", lista]).0,
        SALIDA_OK
    );
    assert!(r.listar().is_empty());
}

#[test]
fn errores_de_la_orden() {
    let base = carpeta();
    let lista = base.join("r.toml");
    let lista = lista.to_str().unwrap();
    let (c, _, e) = correr(&["recientes", "abrir", "1", "--en", lista]);
    assert_eq!(c, SALIDA_ERROR_USO);
    assert!(e.contains("numero 1"), "{e}");
    let a = proyecto(&base, "a");
    Recientes::en(lista).agregar(&a).unwrap();
    std::fs::remove_file(a.join("proyecto.toml")).unwrap();
    let (c, _, e) = correr(&["recientes", "abrir", "1", "--en", lista]);
    assert_eq!(c, SALIDA_ERROR_EJECUCION);
    assert!(e.contains("ya no existe"), "{e}");
    assert_eq!(
        correr(&["recientes", "abrir", "x", "--en", lista]).0,
        SALIDA_ERROR_USO
    );
    assert_eq!(
        correr(&["recientes", "abrir", "--en", lista]).0,
        SALIDA_ERROR_USO
    );
    assert_eq!(correr(&["recientes", "volar"]).0, SALIDA_ERROR_USO);
    assert_eq!(correr(&["recientes", "listar", "x"]).0, SALIDA_ERROR_USO);
}

#[test]
fn abrir_o_crear_un_proyecto_lo_apunta_en_la_lista_del_usuario() {
    // `.cargo/config.toml` apunta LORENZ_HOME a `target/`, asi que esto no toca la carpeta personal.
    let home = std::env::var("LORENZ_HOME").expect("LORENZ_HOME lo fija .cargo/config.toml");
    assert!(home.contains("target"), "{home}");
    // Otras pruebas escriben en la misma lista a la vez; si una pisa la entrada, se repite.
    let mut aparece = false;
    for _ in 0..5 {
        let d = carpeta().join("mi-nuevo-proyecto");
        assert_eq!(
            correr(&["nuevo", "programa-simple", d.to_str().unwrap()]).0,
            SALIDA_OK
        );
        let canonica = std::fs::canonicalize(&d)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        if Recientes::por_omision()
            .listar()
            .iter()
            .any(|p| p.ruta == canonica)
        {
            aparece = true;
            break;
        }
    }
    assert!(aparece);
}
