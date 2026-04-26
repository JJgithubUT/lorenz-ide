use lorenz_ens::ensamblador::{Ensamblador, Fuente};
use lorenz_gpu::*;

fn ruta(rel: &str) -> String {
    format!("{}/../../ejemplos/gpu-16/{rel}", env!("CARGO_MANIFEST_DIR"))
}

fn sistema(fuente: &str) -> SistemaGrafico {
    let ruta_m = ruta("maquina.toml");
    let maquina = lorenz_arch::cargar_archivo(&ruta_m).unwrap();
    let config = cargar_config_archivo(&ruta_m).unwrap().unwrap();
    let obj = Ensamblador::nuevo(&maquina)
        .unwrap()
        .ensamblar(&Fuente::nueva("prueba.s", fuente))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let mut emu = lorenz_emu::Emulador::nuevo(maquina).unwrap();
    for (d, w) in obj.palabras() {
        emu.cargar_programa(d, &[w]).unwrap();
    }
    SistemaGrafico::nuevo(emu, config).unwrap()
}

const TRIANGULO: &str = "
        LI    R1, 1
        OUT   0x20, R1
        LI    R1, 0x1F
        OUT   0x20, R1
        LI    R1, 5
        OUT   0x20, R1
        LI    R1, 2
        OUT   0x20, R1
        OUT   0x20, R1
        LI    R2, 20
        OUT   0x20, R2
        OUT   0x20, R1
        OUT   0x20, R1
        OUT   0x20, R2
        HLT
";

#[test]
fn la_cpu_dibuja_un_triangulo_por_el_puerto() {
    let mut s = sistema(TRIANGULO);
    s.ejecutar(1000).unwrap();
    s.terminar_gpu(100_000);
    let m = s.gpu().metricas();
    assert_eq!(m.comandos, 2);
    assert_eq!(m.triangulos, 1);
    assert!(m.pixeles_escritos > 100);
    assert_eq!(
        s.gpu().superficie().pixel(3, 3),
        Some(0x0000FF),
        "azul RGB565 ampliado"
    );
    assert_eq!(s.gpu().superficie().pixel(30, 20), Some(0));
}

#[test]
fn los_ciclos_de_cpu_y_gpu_van_separados() {
    let mut s = sistema(TRIANGULO);
    s.ejecutar(1000).unwrap();
    let cpu = s.ciclos_cpu();
    s.terminar_gpu(100_000);
    let m = s.gpu().metricas();
    assert_eq!(cpu, s.emulador().estado().ciclos());
    assert!(
        m.ciclos_ocupada > cpu,
        "dibujar cuesta mas que mandar el comando"
    );
}

#[test]
fn el_estado_informa_de_ocupada_y_de_huecos() {
    let mut s = sistema(
        "
        LI R1, 2
        OUT 0x20, R1
        IN R2, 0x21
        HLT
",
    );
    s.ejecutar(100).unwrap();
    let estado = s.emulador().estado().registro("R2").unwrap();
    assert_eq!(estado & 1, 1, "la unidad tiene un LIMPIAR en curso");
    assert_eq!(
        estado >> 8,
        8,
        "la unidad ya tomo el comando: la cola esta vacia"
    );
}

#[test]
fn un_comando_desconocido_enciende_el_bit_de_error_y_control_lo_apaga() {
    let mut s = sistema(
        "
        LI R1, 99
        OUT 0x20, R1
        IN R2, 0x21
        LI R3, 1
        OUT 0x22, R3
        IN R4, 0x21
        HLT
",
    );
    s.ejecutar(100).unwrap();
    let e = |r| s.emulador().estado().registro(r).unwrap();
    assert_eq!(e("R2") >> 2 & 1, 1);
    assert_eq!(e("R4") >> 2 & 1, 0);
}

#[test]
fn la_cola_llena_rechaza_y_marca_error() {
    // 12 comandos de limpiar seguidos con la cola de 8
    let mut f = String::from("LI R1, 2\n");
    for _ in 0..14 {
        f.push_str("OUT 0x20, R1\n");
    }
    f.push_str("IN R2, 0x21\nHLT\n");
    let mut s = sistema(&f);
    s.ejecutar(100).unwrap();
    assert!(s.gpu().metricas().rechazados > 0);
    assert_eq!(s.emulador().estado().registro("R2").unwrap() >> 2 & 1, 1);
}

#[test]
fn el_fin_de_lote_interrumpe_a_la_cpu() {
    let mut s = sistema(
        "
        LI R1, 9
        OUT 0x20, R1
        LI R2, 100
bucle:  ADDI R2, R2, -1
        BNE R2, R0, bucle
        HLT
        .org 0x0100
        .dw rutina
rutina: LI R5, 7
        HLT
",
    );
    s.ejecutar(10_000).unwrap();
    assert_eq!(
        s.emulador().estado().registro("R5").unwrap(),
        7,
        "entro en la rutina de la GPU"
    );
}

#[test]
fn sin_seccion_gpu_la_maquina_corre_igual() {
    let ruta_m = ruta("../risc-16/maquina.toml");
    assert_eq!(cargar_config_archivo(&ruta_m).unwrap(), None);
}

#[test]
fn palabras_de_16_bits_se_extienden_con_signo() {
    let v = convertir_palabras(&[4, 0xFFFF, 0, 5, 0xFFFE], 16);
    assert_eq!(v, vec![4, -1, 0, 5, -2]);
    assert_eq!(color_nativo(0xF800, 16), 0xFF0000);
    assert_eq!(color_nativo(0x07E0, 16), 0x00FF00);
    assert_eq!(color_nativo(0xE0, 8), 0xFF0000);
    assert_eq!(color_nativo(0x123456, 32), 0x123456);
}

#[test]
fn falta_el_dispositivo_o_sus_puertos() {
    let ruta_m = ruta("maquina.toml");
    let maquina = lorenz_arch::cargar_archivo(&ruta_m).unwrap();
    let emu = || lorenz_emu::Emulador::nuevo(maquina.clone()).unwrap();
    let mut c = cargar_config_archivo(&ruta_m).unwrap().unwrap();
    c.dispositivo = None;
    assert!(SistemaGrafico::nuevo(emu(), c.clone())
        .err()
        .unwrap()
        .to_string()
        .contains("falta `dispositivo`"));
    c.dispositivo = Some("otra".into());
    assert!(SistemaGrafico::nuevo(emu(), c)
        .err()
        .unwrap()
        .to_string()
        .contains("`otra`"));
}
