//! Dispositivos externos y modo seguro (RF-ECO-037, RF-ECO-038, RF-ECO-009).
//!
//! Las pruebas comparten el interruptor global del modo seguro, asi que corren de una en una.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use lorenz_cli::{ejecutar_cli, SALIDA_ERROR_EJECUCION, SALIDA_ERROR_USO, SALIDA_OK};

static CERROJO: Mutex<()> = Mutex::new(());
static N: AtomicUsize = AtomicUsize::new(0);

fn temporal() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "lorenz-ext-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&d).unwrap();
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

fn hay_python() -> bool {
    Command::new("python3").arg("--version").output().is_ok()
}

const SUMA: &str = "../../ejemplos/externo-8/suma.s";

/// La maquina de ejemplo con la linea del dispositivo cambiada.
fn maquina_con(dispositivo: &str) -> PathBuf {
    let base = std::fs::read_to_string("../../ejemplos/externo-8/maquina.toml").unwrap();
    let inicio = base
        .find("[[dispositivo]]\nnombre  = \"acumulador\"")
        .unwrap();
    let texto = format!("{}{}\n", &base[..inicio], dispositivo);
    let d = temporal();
    let ruta = d.join("maquina.toml");
    std::fs::write(&ruta, texto).unwrap();
    ruta
}

#[test]
fn un_proceso_externo_atiende_los_puertos() {
    let _g = CERROJO.lock().unwrap_or_else(|e| e.into_inner());
    if !hay_python() {
        eprintln!("python3 no esta instalado: se omite");
        return;
    }
    let m = maquina_con(
        "[[dispositivo]]\nnombre = \"acumulador\"\ntipo = \"externo\"\norden = \"python3 ../../ejemplos/externo-8/acumulador.py\"\npuertos = { datos = 0x10 }",
    );
    let (c, s, e) = correr(&["ejecutar", m.to_str().unwrap(), SUMA]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.contains("B=0xc"), "{s}");
}

/// Servidor TCP con el mismo protocolo que el proceso de ejemplo.
fn servidor() -> (String, std::thread::JoinHandle<Vec<String>>) {
    let escucha = TcpListener::bind("127.0.0.1:0").unwrap();
    let direccion = escucha.local_addr().unwrap().to_string();
    let hilo = std::thread::spawn(move || {
        let (flujo, _) = escucha.accept().unwrap();
        let mut escritura = flujo.try_clone().unwrap();
        let mut total = 0u64;
        let mut vistas = Vec::new();
        for linea in BufReader::new(flujo).lines() {
            let linea = linea.unwrap();
            vistas.push(linea.clone());
            let p: Vec<&str> = linea.split_whitespace().collect();
            match p.as_slice() {
                ["ESCRIBIR", _, v] => {
                    total += v.parse::<u64>().unwrap();
                    writeln!(escritura, "OK").unwrap();
                }
                ["LEER", _] => writeln!(escritura, "0x{total:x}").unwrap(),
                ["FIN"] => break,
                _ => writeln!(escritura, "ERR no entiendo").unwrap(),
            }
        }
        vistas
    });
    (direccion, hilo)
}

#[test]
fn un_servidor_tcp_atiende_los_puertos() {
    let _g = CERROJO.lock().unwrap_or_else(|e| e.into_inner());
    let (direccion, hilo) = servidor();
    let m = maquina_con(&format!(
        "[[dispositivo]]\nnombre = \"acumulador\"\ntipo = \"externo\"\ndireccion = \"{direccion}\"\npuertos = {{ datos = 0x10 }}"
    ));
    let (c, s, e) = correr(&["ejecutar", m.to_str().unwrap(), SUMA]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.contains("B=0xc"), "{s}");
    let vistas = hilo.join().unwrap();
    assert_eq!(vistas, ["ESCRIBIR 16 5", "ESCRIBIR 16 7", "LEER 16", "FIN"]);
}

#[test]
fn un_dispositivo_que_no_existe_da_un_error_claro() {
    let _g = CERROJO.lock().unwrap_or_else(|e| e.into_inner());
    let m = maquina_con("[[dispositivo]]\nnombre = \"acumulador\"\ntipo = \"externo\"\norden = \"este-programa-no-existe-lorenz\"\npuertos = { datos = 0x10 }");
    let (c, _, e) = correr(&["ejecutar", m.to_str().unwrap(), SUMA]);
    assert_eq!(c, SALIDA_ERROR_EJECUCION, "{e}");
    assert!(
        e.contains("acumulador") && e.contains("no se pudo lanzar"),
        "{e}"
    );
}

#[test]
fn un_servidor_caido_da_un_error_claro() {
    let _g = CERROJO.lock().unwrap_or_else(|e| e.into_inner());
    // Se reserva un puerto y se libera: nadie escucha.
    let libre = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .to_string();
    let m = maquina_con(&format!(
        "[[dispositivo]]\nnombre = \"acumulador\"\ntipo = \"externo\"\ndireccion = \"{libre}\"\npuertos = {{ datos = 0x10 }}"
    ));
    let (c, _, e) = correr(&["ejecutar", m.to_str().unwrap(), SUMA]);
    assert_eq!(c, SALIDA_ERROR_EJECUCION, "{e}");
    assert!(e.contains("no se pudo conectar"), "{e}");
}

#[test]
fn una_respuesta_de_error_del_dispositivo_se_propaga() {
    let _g = CERROJO.lock().unwrap_or_else(|e| e.into_inner());
    if !hay_python() {
        return;
    }
    let d = temporal();
    let script = d.join("malo.py");
    std::fs::write(
        &script,
        "import sys\nfor l in sys.stdin:\n    print('ERR no quiero', flush=True)\n",
    )
    .unwrap();
    let m = maquina_con(&format!(
        "[[dispositivo]]\nnombre = \"acumulador\"\ntipo = \"externo\"\norden = \"python3 {}\"\npuertos = {{ datos = 0x10 }}",
        script.display()
    ));
    let (c, _, e) = correr(&["ejecutar", m.to_str().unwrap(), SUMA]);
    assert_eq!(c, SALIDA_ERROR_EJECUCION);
    assert!(e.contains("no quiero"), "{e}");
}

#[test]
fn la_descripcion_exige_orden_o_direccion_y_no_las_dos() {
    let _g = CERROJO.lock().unwrap_or_else(|e| e.into_inner());
    for d in [
        "[[dispositivo]]\nnombre = \"x\"\ntipo = \"externo\"\npuertos = { datos = 0x10 }",
        "[[dispositivo]]\nnombre = \"x\"\ntipo = \"externo\"\norden = \"a\"\ndireccion = \"h:1\"\npuertos = { datos = 0x10 }",
        "[[dispositivo]]\nnombre = \"x\"\ntipo = \"externo\"\ndireccion = \"sin-puerto\"\npuertos = { datos = 0x10 }",
        "[[dispositivo]]\nnombre = \"x\"\ntipo = \"externo\"\norden = \"a\"",
    ] {
        let m = maquina_con(d);
        let (c, _, e) = correr(&["validar", m.to_str().unwrap()]);
        assert_ne!(c, SALIDA_OK, "{d}");
        assert!(!e.is_empty());
    }
}

#[test]
fn el_modo_seguro_no_arranca_dispositivos_externos() {
    let _g = CERROJO.lock().unwrap_or_else(|e| e.into_inner());
    let (direccion, hilo) = servidor();
    let m = maquina_con(&format!(
        "[[dispositivo]]\nnombre = \"acumulador\"\ntipo = \"externo\"\ndireccion = \"{direccion}\"\npuertos = {{ datos = 0x10 }}"
    ));
    let (c, _, e) = correr(&["--seguro", "ejecutar", m.to_str().unwrap(), SUMA]);
    assert_eq!(c, SALIDA_ERROR_EJECUCION, "{e}");
    assert!(e.contains("modo seguro"), "{e}");
    // Sin el modo seguro, la misma orden si conecta (y deja al servidor terminar).
    let (c, _, _) = correr(&["ejecutar", m.to_str().unwrap(), SUMA]);
    assert_eq!(c, SALIDA_OK);
    hilo.join().unwrap();
}

#[test]
fn el_modo_seguro_prohibe_escribir_en_disco() {
    let _g = CERROJO.lock().unwrap_or_else(|e| e.into_inner());
    let d = temporal();
    let hex = d.join("p.hex");
    let m = "../../ejemplos/ejemplo-8/maquina.toml";
    let fuente = d.join("p.s");
    std::fs::write(&fuente, "HLT\n").unwrap();
    for args in [
        vec![
            "--seguro",
            "ensamblar",
            m,
            fuente.to_str().unwrap(),
            "-o",
            hex.to_str().unwrap(),
        ],
        vec![
            "--seguro",
            "documentar",
            m,
            "-o",
            d.join("doc.md").to_str().unwrap(),
        ],
        vec![
            "--seguro",
            "exportar",
            "pdf",
            m,
            "-o",
            d.join("x.pdf").to_str().unwrap(),
        ],
        vec![
            "--seguro",
            "nuevo",
            "so-minimo",
            d.join("nuevo").to_str().unwrap(),
        ],
        vec!["--seguro", "paquete", "listar"],
    ] {
        let (c, _, e) = correr(&args);
        assert_eq!(c, SALIDA_ERROR_USO, "{args:?}: {e}");
        assert!(e.contains("modo seguro"), "{args:?}: {e}");
    }
    assert!(
        !hex.exists()
            && !d.join("doc.md").exists()
            && !d.join("x.pdf").exists()
            && !d.join("nuevo").exists()
    );
}

#[test]
fn el_modo_seguro_deja_ejecutar_y_ensamblar_a_la_salida() {
    let _g = CERROJO.lock().unwrap_or_else(|e| e.into_inner());
    let d = temporal();
    let fuente = d.join("p.s");
    std::fs::write(&fuente, "LDI A, 3\nHLT\n").unwrap();
    let m = "../../ejemplos/ejemplo-8/maquina.toml";
    let (c, s, e) = correr(&["--seguro", "ejecutar", m, fuente.to_str().unwrap()]);
    assert_eq!(c, SALIDA_OK, "{e}");
    assert!(s.contains("A=0x3"));
    let (c, s, _) = correr(&["--seguro", "ensamblar", m, fuente.to_str().unwrap()]);
    assert_eq!(c, SALIDA_OK);
    assert!(s.contains('#') || !s.is_empty());
}

#[test]
fn la_variable_de_entorno_activa_el_modo_seguro() {
    let _g = CERROJO.lock().unwrap_or_else(|e| e.into_inner());
    let d = temporal();
    std::env::set_var("LORENZ_SEGURO", "1");
    let (c, _, e) = correr(&["nuevo", "so-minimo", d.join("n").to_str().unwrap()]);
    std::env::remove_var("LORENZ_SEGURO");
    assert_eq!(c, SALIDA_ERROR_USO, "{e}");
    assert!(!d.join("n").exists());
}

#[test]
fn el_modo_seguro_se_apaga_al_terminar_la_orden() {
    let _g = CERROJO.lock().unwrap_or_else(|e| e.into_inner());
    let _ = correr(&["--seguro", "version"]);
    assert!(!lorenz_emu::externo::estan_prohibidos());
}
