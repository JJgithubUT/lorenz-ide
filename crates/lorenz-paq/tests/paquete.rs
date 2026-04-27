use lorenz_paq::paquete::{nombre_valido, ruta_valida};
use lorenz_paq::*;

const M: &str = "[paquete]\nnombre = \"mi-cpu\"\nversion = \"1.2.0\"\nautor = \"Ana\"\nlicencia = \"MIT\"\ntipo = \"arquitectura\"\n\n[dependencias]\nbase = \">=1.0\"\n";

fn carpeta(nombre: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("lorenz-paq-{}-{nombre}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn paquete() -> Paquete {
    let d = carpeta("ejemplo");
    std::fs::write(d.join("paquete.toml"), M).unwrap();
    std::fs::write(d.join("maquina.toml"), "[maquina]\nnombre=\"x\"\n").unwrap();
    std::fs::create_dir_all(d.join("programas")).unwrap();
    std::fs::write(d.join("programas/hola.s"), "HLT\n").unwrap();
    // cosas que no se empaquetan
    std::fs::create_dir_all(d.join("target")).unwrap();
    std::fs::write(d.join("target/x"), "no").unwrap();
    std::fs::create_dir_all(d.join(".git")).unwrap();
    std::fs::write(d.join("viejo.lpkg"), "no").unwrap();
    Paquete::desde_carpeta(&d).unwrap()
}

#[test]
fn el_manifiesto_se_lee_completo() {
    let m = Manifiesto::leer(M).unwrap();
    assert_eq!(
        (m.nombre.as_str(), m.version.to_string().as_str()),
        ("mi-cpu", "1.2.0")
    );
    assert_eq!(m.tipo, Tipo::Arquitectura);
    assert_eq!(m.dependencias.len(), 1);
    assert_eq!(m.dependencias[0].0, "base");
}

#[test]
fn el_manifiesto_rechaza_lo_incompleto_o_raro() {
    for (cambio, frag) in [
        (("nombre = \"mi-cpu\"", "nombre = \"Mi CPU\""), "nombre"),
        (("version = \"1.2.0\"", "version = \"x\""), "version"),
        (("tipo = \"arquitectura\"", "tipo = \"cosa\""), "tipo"),
        (("licencia = \"MIT\"\n", ""), "licencia"),
        (("autor = \"Ana\"\n", ""), "autor"),
        ((">=1.0", ">=uno"), "dependencia `base`"),
        (("base = \">=1.0\"", "mi-cpu = \"*\""), "de si mismo"),
        (
            ("[dependencias]", "[extra]\nx = 1\n[dependencias]"),
            "extra",
        ),
    ] {
        let t = M.replace(cambio.0, cambio.1);
        let e = Manifiesto::leer(&t).unwrap_err().to_string();
        assert!(e.contains(frag), "{cambio:?} -> {e}");
    }
}

#[test]
fn los_nombres_y_rutas_validos() {
    assert!(nombre_valido("a") && nombre_valido("cpu-8_bits2"));
    assert!(
        !nombre_valido("") && !nombre_valido("A") && !nombre_valido("-a") && !nombre_valido("a b")
    );
    assert!(!nombre_valido(&"a".repeat(65)));
    assert!(ruta_valida("a/b.txt").is_ok());
    for mala in [
        "", "/etc/x", "../x", "a/../b", "a//b", "a\\b", "C:/x", "./a", "a/",
    ] {
        assert!(ruta_valida(mala).is_err(), "{mala}");
    }
}

#[test]
fn empaquetar_deja_fuera_lo_que_no_va() {
    let p = paquete();
    let rutas: Vec<&str> = p.archivos.keys().map(String::as_str).collect();
    assert_eq!(rutas, ["maquina.toml", "paquete.toml", "programas/hola.s"]);
}

#[test]
fn ida_y_vuelta_y_bytes_repetibles() {
    let p = paquete();
    let b = p.a_bytes();
    assert!(
        b.starts_with(b"LORENZ-PAQUETE 1\nARCHIVO paquete.toml "),
        "el manifiesto va primero"
    );
    assert_eq!(b, p.a_bytes());
    assert_eq!(Paquete::desde_bytes(&b).unwrap(), p);
}

#[test]
fn cualquier_alteracion_se_detecta() {
    let b = paquete().a_bytes();
    // cambiar un byte en cada posicion del contenido de un archivo
    let pos = b.windows(4).position(|w| w == b"HLT\n").unwrap();
    let mut x = b.clone();
    x[pos] = b'X';
    assert!(
        matches!(Paquete::desde_bytes(&x), Err(ErrorPaquete::Integridad(m)) if m.contains("programas/hola.s"))
    );
    // truncado
    for n in [0, 5, b.len() / 2, b.len() - 1] {
        assert!(Paquete::desde_bytes(&b[..n]).is_err(), "truncado a {n}");
    }
    // datos de mas
    let mut y = b.clone();
    y.extend(b"basura");
    assert!(Paquete::desde_bytes(&y).is_err());
    // suma final cambiada
    let mut z = b.clone();
    let l = z.len();
    z[l - 3] = if z[l - 3] == b'0' { b'1' } else { b'0' };
    assert!(matches!(
        Paquete::desde_bytes(&z),
        Err(ErrorPaquete::Integridad(_))
    ));
    // no es un paquete
    assert!(Paquete::desde_bytes(b"hola").is_err());
}

#[test]
fn rutas_peligrosas_en_el_archivo_se_rechazan_aunque_las_sumas_cuadren() {
    let mut cuerpo = b"LORENZ-PAQUETE 1\n".to_vec();
    for (ruta, datos) in [
        ("paquete.toml", M.as_bytes()),
        ("../escapa.txt", b"x".as_slice()),
    ] {
        cuerpo
            .extend(format!("ARCHIVO {ruta} {} {}\n", datos.len(), sha256_hex(datos)).into_bytes());
        cuerpo.extend(datos);
        cuerpo.push(b'\n');
    }
    let suma = sha256_hex(&cuerpo);
    cuerpo.extend(format!("FIN {suma}\n").into_bytes());
    assert!(matches!(
        Paquete::desde_bytes(&cuerpo),
        Err(ErrorPaquete::Ruta(_))
    ));
}

#[test]
fn sin_manifiesto_no_hay_paquete() {
    let d = carpeta("vacio");
    std::fs::write(d.join("a.txt"), "x").unwrap();
    assert!(Paquete::desde_carpeta(&d)
        .unwrap_err()
        .to_string()
        .contains("paquete.toml"));
}

#[test]
fn avisa_si_exige_un_formato_mas_nuevo() {
    let mut p = paquete();
    assert!(p.avisos().is_empty());
    p.manifiesto.requiere_formato = Some("0.1".into());
    assert!(p.avisos().is_empty());
    p.manifiesto.requiere_formato = Some("0.9".into());
    let a = p.avisos();
    assert_eq!(a.len(), 1);
    assert!(
        a[0].contains("exige el formato de arquitectura 0.9") && a[0].contains("0.1"),
        "{a:?}"
    );
    p.manifiesto.requiere_formato = Some("raro".into());
    assert!(p.avisos()[0].contains("no es una version"));
}
