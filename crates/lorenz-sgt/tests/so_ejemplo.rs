//! Lorenz OS, el sistema operativo de ejemplo (RF-ECO-039, RF-ECO-040, RF-ECO-042).
//!
//! Se compila desde su carpeta, se arranca en el emulador, se le teclea y se mira la pantalla.

use lorenz_arch::carga::cargar_archivo;
use lorenz_emu::Emulador;
use lorenz_sgt::analisis::Opciones;
use lorenz_sgt::compilar::{compilar_con, en_carpeta_de};

fn carpeta() -> String {
    format!("{}/../../ejemplos/so-ejemplo", env!("CARGO_MANIFEST_DIR"))
}

fn arrancar() -> Emulador {
    let maquina = cargar_archivo(format!("{}/maquina.toml", carpeta())).unwrap();
    let ruta = format!("{}/so.sgt", carpeta());
    let texto = std::fs::read_to_string(&ruta).unwrap();
    let c = compilar_con(
        &ruta,
        &texto,
        &maquina,
        Opciones::default(),
        &en_carpeta_de(&ruta),
    )
    .unwrap_or_else(|e| {
        panic!(
            "{}",
            e.iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    let mut emu = Emulador::nuevo(maquina).unwrap();
    for (dir, w) in c.palabras() {
        emu.cargar_programa(dir, &[w]).unwrap();
    }
    emu
}

/// Reconoce el texto de la pantalla: cada casilla de 4x6 pixeles se compara con el dibujo de los
/// caracteres de la fuente del sistema.
fn texto_en_pantalla(emu: &Emulador) -> Vec<String> {
    let img = emu.pantalla("pantalla").unwrap();
    let mut filas = Vec::new();
    for fila in 0..5u32 {
        let mut linea = String::new();
        for col in 0..16u32 {
            let mut bits = 0u32;
            for f in 0..5u32 {
                for k in 0..3u32 {
                    let p = img.pixel(col * 4 + k, fila * 6 + f).unwrap();
                    bits = (bits << 1) | u32::from(p != 0);
                }
            }
            linea.push(caracter_de(bits));
        }
        filas.push(linea.trim_end().to_string());
    }
    filas
}

/// Lo inverso de `glifo` en `pantalla.sgt`, leido del propio archivo para no repetir la fuente.
fn caracter_de(bits: u32) -> char {
    let fuente = std::fs::read_to_string(format!("{}/pantalla.sgt", carpeta())).unwrap();
    for linea in fuente.lines() {
        let l = linea.trim();
        // `        65 => { devolver 21965; }  // A`
        if let Some((codigo, resto)) = l.split_once(" => { devolver ") {
            if let (Ok(c), Some((valor, _))) = (codigo.parse::<u32>(), resto.split_once(';')) {
                if valor.parse::<u32>() == Ok(bits) && bits != 0 {
                    return char::from_u32(c).unwrap();
                }
            }
        }
    }
    ' '
}

fn teclear(emu: &mut Emulador, texto: &str) {
    for b in texto.bytes() {
        emu.pulsar_tecla("teclado", u64::from(b)).unwrap();
        emu.avanzar(40_000).unwrap();
    }
    // Deja que el sistema termine lo que esta haciendo (subir el texto cuesta miles de pasos).
    emu.avanzar(600_000).unwrap();
}

#[test]
fn arranca_y_anuncia_el_sistema() {
    let mut emu = arrancar();
    emu.avanzar(2_000_000).unwrap();
    let t = texto_en_pantalla(&emu);
    assert_eq!(t[0], "LORENZ OS");
    assert_eq!(t[1], "RAM 32");
    assert_eq!(t[2], "LISTO");
    assert_eq!(t[3], ">");
}

#[test]
fn lo_que_se_teclea_sale_en_pantalla() {
    let mut emu = arrancar();
    emu.avanzar(2_000_000).unwrap();
    teclear(&mut emu, "hola");
    assert_eq!(texto_en_pantalla(&emu)[3], ">HOLA");
}

#[test]
fn borrar_quita_el_ultimo_caracter() {
    let mut emu = arrancar();
    emu.avanzar(2_000_000).unwrap();
    teclear(&mut emu, "abc\x08\x08x");
    assert_eq!(texto_en_pantalla(&emu)[3], ">AX");
}

#[test]
fn las_ordenes_responden() {
    let mut emu = arrancar();
    emu.avanzar(2_000_000).unwrap();
    teclear(&mut emu, "ayuda\n");
    let t = texto_en_pantalla(&emu);
    assert_eq!(&t[1..], [">AYUDA", "HORA LIMPIAR", "TAREAS", ">"], "{t:?}");
    teclear(&mut emu, "zzz\n");
    let t = texto_en_pantalla(&emu);
    assert_eq!(&t[2..], [">ZZZ", "NO SE", ">"], "{t:?}");
}

#[test]
fn limpiar_borra_la_pantalla_y_hora_cuenta_tics() {
    let mut emu = arrancar();
    emu.avanzar(2_000_000).unwrap();
    teclear(&mut emu, "hora\n");
    let t = texto_en_pantalla(&emu);
    let tics: u32 = t
        .iter()
        .find_map(|l| l.parse().ok())
        .unwrap_or_else(|| panic!("no hay numero: {t:?}"));
    assert!(tics > 10, "{tics}");
    teclear(&mut emu, "limpiar\n");
    let t = texto_en_pantalla(&emu);
    assert_eq!(t[0], ">");
    assert!(t[1..].iter().all(|l| l.is_empty()), "{t:?}");
}

#[test]
fn el_texto_sube_cuando_se_llena_la_pantalla() {
    let mut emu = arrancar();
    emu.avanzar(2_000_000).unwrap();
    teclear(&mut emu, "a\nb\nc\nd\n");
    let t = texto_en_pantalla(&emu);
    assert!(
        t.iter().filter(|l| l.as_str() == "NO SE").count() >= 2,
        "{t:?}"
    );
}

#[test]
fn el_planificador_ejecuta_las_tareas_a_su_ritmo() {
    let mut emu = arrancar();
    emu.avanzar(2_000_000).unwrap();
    teclear(&mut emu, "tareas\n");
    let t = texto_en_pantalla(&emu);
    let cuentas: Vec<u32> = t
        .iter()
        .find(|l| l.split(' ').count() == 2 && l.split(' ').all(|n| n.parse::<u32>().is_ok()))
        .unwrap_or_else(|| panic!("no hay cuentas: {t:?}"))
        .split(' ')
        .map(|n| n.parse().unwrap())
        .collect();
    // La tarea 0 toca cada 5 tics y la 1 cada 50: la primera va unas diez veces por vuelta de la segunda.
    assert!(cuentas[0] >= 5 && cuentas[1] >= 1, "{cuentas:?}");
    assert!(cuentas[0] / cuentas[1].max(1) >= 5, "{cuentas:?}");
}

#[test]
fn el_latido_parpadea() {
    let mut emu = arrancar();
    emu.avanzar(2_000_000).unwrap();
    let mut vistos = std::collections::BTreeSet::new();
    for _ in 0..60 {
        emu.avanzar(1_000).unwrap();
        vistos.insert(emu.pantalla("pantalla").unwrap().pixel(63, 31).unwrap());
    }
    assert!(vistos.len() >= 2, "el pixel no cambia: {vistos:?}");
}
