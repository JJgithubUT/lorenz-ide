//! Explicar la instruccion que toca (RF-DEP-032).

use lorenz_dbg::comandos::Sesion;
use lorenz_dbg::Depurador;
use lorenz_emu::Emulador;
use lorenz_ens::ensamblador::{Ensamblador, Fuente};

const PROGRAMA: &str = "\
inicio: LDI A, 5
        LDI B, 251
        ADD A, B
        ST [0x2000], A
        JZ inicio
        JZ fin
fin:    HLT
";

fn depurador() -> Depurador {
    let t = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
    let m = lorenz_arch::cargar(&t).unwrap();
    let obj = Ensamblador::nuevo(&m)
        .unwrap()
        .ensamblar(&Fuente::nueva("p.s", PROGRAMA))
        .unwrap();
    let mut emu = Emulador::nuevo(m).unwrap();
    for s in &obj.segmentos {
        emu.cargar_programa(s.direccion, &s.palabras).unwrap();
    }
    let mut d = Depurador::nuevo(emu);
    d.con_fuente(&obj, &[("p.s".to_string(), PROGRAMA.to_string())]);
    d
}

#[test]
fn explicar_no_ejecuta_nada() {
    let d = depurador();
    let antes = d.emulador().estado().pc();
    let e = d.explicar().unwrap();
    assert_eq!(d.emulador().estado().pc(), antes);
    assert_eq!(d.emulador().estado().registro("A").unwrap(), 0);
    assert_eq!(e.direccion, 0);
}

#[test]
fn una_carga_muestra_el_registro_que_cambia() {
    let d = depurador();
    let e = d.explicar().unwrap();
    assert_eq!(e.registros.len(), 1);
    assert_eq!(e.registros[0].nombre, "A");
    assert_eq!((e.registros[0].antes, e.registros[0].despues), (0, 5));
    assert!(e.texto.contains("LDI"));
    assert_eq!(e.salto, None, "LDI no toca el contador de programa");
    assert!(!e.operaciones.is_empty());
}

#[test]
fn una_suma_que_da_cero_enciende_z_y_c() {
    let mut d = depurador();
    d.paso(2);
    let e = d.explicar().unwrap();
    let nombres: Vec<_> = e.banderas.iter().map(|b| b.nombre.as_str()).collect();
    assert!(nombres.contains(&"Z"), "{e:?}");
    assert!(e.banderas.iter().all(|b| !b.antes && b.despues));
    assert_eq!(e.registros[0].despues, 0, "5 + 251 = 256 -> 0");
    assert!(!e.afecta.is_empty());
}

#[test]
fn una_escritura_en_memoria_se_anota_con_el_valor_anterior() {
    let mut d = depurador();
    d.paso(3);
    let e = d.explicar().unwrap();
    assert_eq!(e.escrituras.len(), 1);
    assert_eq!(e.escrituras[0].direccion, 0x2000);
    assert_eq!(e.escrituras[0].antes, 0);
    // Y la memoria real sigue intacta.
    assert_eq!(d.emulador().memoria().leer_privilegiado(0x2000).unwrap(), 0);
}

#[test]
fn un_salto_condicional_dice_si_se_toma() {
    let mut d = depurador();
    d.paso(4);
    let tomado = d.explicar().unwrap();
    assert_eq!(
        tomado.salto,
        Some(true),
        "Z esta encendida: JZ inicio salta"
    );
    assert_eq!(tomado.pc_despues, 0);
    // Con Z apagada el mismo salto no se toma.
    d.emulador_mut()
        .estado_mut()
        .fijar_bandera("Z", false)
        .unwrap();
    let no = d.explicar().unwrap();
    assert_eq!(no.salto, Some(false));
    assert_eq!(no.pc_despues, d.emulador().estado().pc() + 3);
}

#[test]
fn la_orden_explicar_da_texto_legible() {
    let mut s = Sesion::nueva(depurador());
    let t = s.ejecutar("explicar").texto;
    assert!(t.contains("LDI A, 5"), "{t}");
    assert!(t.contains("operaciones:"), "{t}");
    assert!(t.contains("registro A: 0x0 -> 0x5"), "{t}");
    assert_eq!(s.ejecutar("x").texto, t);
}

#[test]
fn explicar_la_parada_avisa_en_lugar_de_fallar_en_silencio() {
    let mut d = depurador();
    d.paso(4);
    d.emulador_mut()
        .estado_mut()
        .fijar_bandera("Z", false)
        .unwrap();
    assert_eq!(d.continuar(100), lorenz_dbg::Parada::Detenida);
    assert_eq!(
        d.explicar().unwrap_err().to_string().contains("detenida"),
        true
    );
}
