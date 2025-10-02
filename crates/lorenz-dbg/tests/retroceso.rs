//! Grabacion, retroceso y navegacion por la traza (RF-DEP-019 a RF-DEP-023, RF-DEP-027, RF-DEP-028).

use lorenz_dbg::{Depurador, Parada};
use lorenz_emu::Emulador;
use lorenz_ens::ensamblador::{Ensamblador, Fuente};

const PROGRAMA: &str = "\
        LDI A, 1
        LDI B, 2
        ADD A, B
        ST [0x2000], A
        LDI A, 9
        ST [0x2000], A
        HLT
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
    d.con_fuente(&obj, &[("p.s".into(), PROGRAMA.into())]);
    d
}

fn a(d: &Depurador) -> u64 {
    d.emulador().estado().registro("A").unwrap()
}

fn celda(d: &Depurador, dir: u64) -> u64 {
    d.emulador().memoria().leer_privilegiado(dir).unwrap()
}

#[test]
fn sin_grabar_no_hay_traza() {
    let mut d = depurador();
    d.paso(3);
    assert!(d.traza().is_none());
    assert!(d.atras(1).is_err());
}

#[test]
fn graba_un_paso_por_instruccion() {
    let mut d = depurador();
    d.grabar(true);
    d.paso(3);
    assert_eq!(d.traza().unwrap().longitud(), 3);
    assert_eq!(d.instante(), 3);
    let p = &d.traza().unwrap().pasos()[0];
    assert_eq!(p.mnemonico, "LDI");
    assert_eq!(p.registros.len(), 1);
    assert_eq!(p.registros[0].nombre, "A");
    assert_eq!((p.registros[0].antes, p.registros[0].despues), (0, 1));
}

#[test]
fn desactivar_descarta_la_traza() {
    let mut d = depurador();
    d.grabar(true);
    d.paso(2);
    d.grabar(false);
    assert!(d.traza().is_none());
    d.paso(1);
    assert!(d.traza().is_none());
}

#[test]
fn retroceder_un_paso() {
    let mut d = depurador();
    d.grabar(true);
    d.paso(4);
    assert_eq!(celda(&d, 0x2000), 3);
    d.atras(1).unwrap();
    assert_eq!(d.instante(), 3);
    assert_eq!(celda(&d, 0x2000), 0);
    d.atras(1).unwrap();
    assert_eq!(a(&d), 1);
}

#[test]
fn retroceder_varios_y_hasta_el_inicio() {
    let mut d = depurador();
    d.grabar(true);
    d.paso(5);
    d.atras(100).unwrap();
    assert_eq!(d.instante(), 0);
    assert_eq!(a(&d), 0);
    assert_eq!(d.emulador().estado().pc(), 0);
}

#[test]
fn ir_a_cualquier_instante_da_el_mismo_estado_que_ejecutar() {
    let mut d = depurador();
    d.grabar(true);
    d.continuar(100);
    let fin = d.traza().unwrap().longitud();
    for t in 0..=fin {
        d.ir_a(t).unwrap();
        let mut limpio = depurador();
        limpio.paso(t.max(1) * u64::from(t > 0));
        if t == 0 {
            assert_eq!(d.emulador().estado().pc(), 0);
        } else {
            assert_eq!(
                d.emulador().memoria().suma_de_verificacion(),
                limpio.emulador().memoria().suma_de_verificacion(),
                "memoria en {t}"
            );
            assert_eq!(a(&d), a(&limpio), "A en {t}");
            assert_eq!(
                d.emulador().estado().pc(),
                limpio.emulador().estado().pc(),
                "pc en {t}"
            );
        }
    }
}

#[test]
fn adelante_recorre_la_traza_sin_ejecutar_de_nuevo() {
    let mut d = depurador();
    d.grabar(true);
    d.paso(5);
    d.atras(5).unwrap();
    d.adelante(2).unwrap();
    assert_eq!(d.instante(), 2);
    assert!(d.en_el_pasado());
    d.adelante(100).unwrap();
    assert_eq!(d.instante(), 5);
    assert!(!d.en_el_pasado());
    assert_eq!(a(&d), 9);
}

#[test]
fn ejecutar_desde_el_pasado_reescribe_la_historia() {
    let mut d = depurador();
    d.grabar(true);
    d.paso(4);
    d.atras(2).unwrap();
    d.fijar_registro("A", 77).unwrap();
    d.paso(1);
    assert_eq!(d.traza().unwrap().longitud(), 3);
    assert_eq!(d.instante(), 3);
    // El paso grabado desde el pasado ve el valor que se puso a mano.
    assert_eq!(d.traza().unwrap().pasos()[2].mnemonico, "ADD");
    assert_eq!(a(&d), 79);
}

#[test]
fn lo_hecho_a_mano_se_repite_al_volver() {
    let mut d = depurador();
    d.grabar(true);
    d.paso(1);
    d.fijar_registro("B", 40).unwrap();
    d.paso(2); // LDI B,2 pisa B; ADD A,B
    d.paso(1); // ST
    d.ir_a(0).unwrap();
    d.ir_a(4).unwrap();
    assert_eq!(celda(&d, 0x2000), 3);
    d.ir_a(1).unwrap();
    // En el instante 1 aun no se hizo el cambio.
    assert_eq!(d.emulador().estado().registro("B").unwrap(), 0);
}

#[test]
fn escribir_memoria_a_mano_queda_en_la_historia() {
    let mut d = depurador();
    d.grabar(true);
    d.paso(2);
    d.escribir_memoria(0x2005, 0x55).unwrap();
    d.paso(2);
    d.ir_a(0).unwrap();
    assert_eq!(celda(&d, 0x2005), 0);
    d.ir_a(4).unwrap();
    assert_eq!(celda(&d, 0x2005), 0x55);
}

#[test]
fn instante_inexistente() {
    let mut d = depurador();
    d.grabar(true);
    d.paso(2);
    assert!(d.ir_a(3).is_err());
}

#[test]
fn registra_las_escrituras_de_memoria_de_cada_paso() {
    let mut d = depurador();
    d.grabar(true);
    d.paso(4);
    let p = &d.traza().unwrap().pasos()[3];
    assert_eq!(p.mnemonico, "ST");
    assert_eq!(p.escrituras.len(), 1);
    assert_eq!(p.escrituras[0].direccion, 0x2000);
    assert_eq!(p.escrituras[0].despues, 3);
}

#[test]
fn registra_las_banderas_que_cambian() {
    let mut d = depurador();
    d.grabar(true);
    d.paso(3);
    let p = &d.traza().unwrap().pasos()[2];
    assert_eq!(p.mnemonico, "ADD");
    // 1 + 2 no pone ninguna bandera; el cambio de A si queda.
    assert!(p.registros.iter().any(|r| r.nombre == "A"));
}

#[test]
fn con_instantaneas_frecuentes_el_resultado_es_el_mismo() {
    let mut d = depurador();
    d.grabar(true);
    d.traza_mut().unwrap().fijar_intervalo(2);
    d.continuar(100);
    assert!(d.traza().unwrap().instantaneas() > 1);
    d.ir_a(5).unwrap();
    assert_eq!(a(&d), 9);
    assert_eq!(celda(&d, 0x2000), 3);
}

#[test]
fn la_traza_llena_avisa_y_deja_de_grabar() {
    let mut d = depurador();
    d.grabar(true);
    d.traza_mut().unwrap().fijar_limite(3);
    d.paso(5);
    assert_eq!(d.traza().unwrap().longitud(), 3);
    assert!(d.traza().unwrap().llena());
    let avisos = d.tomar_avisos();
    assert_eq!(avisos.len(), 1);
    assert!(avisos[0].contains("limite"));
    assert!(d.tomar_avisos().is_empty());
    // La maquina siguio ejecutando aunque no grabe.
    assert_eq!(d.emulador().estado().instrucciones(), 5);
}

#[test]
fn las_vigilancias_sobreviven_al_retroceso() {
    let mut d = depurador();
    d.grabar(true);
    d.paso(2);
    let id = d.observar(0x2000, lorenz_dbg::Vigilar::Escritura).unwrap();
    d.paso(1);
    d.ir_a(0).unwrap();
    match d.continuar(100) {
        Parada::Observacion { id: i, .. } => assert_eq!(i, id),
        otra => panic!("{otra:?}"),
    }
}
