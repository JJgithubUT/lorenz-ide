//! Comparacion de dos implementaciones (RF-EMU-046).

mod comun;

use comun::ejemplo_8;
use lorenz_arch::modelo::Ciclos;
use lorenz_emu::{comparar, Ejecutor, Emulador, ErrorEjecucion, Paso, Veredicto, Vista};

// LDI A,5 ; LDI B,7 ; ADD A,B ; HLT
const SUMA: [u64; 6] = [0x20, 5, 0x24, 7, 0x31, 0xF0];

fn con(palabras: &[u64]) -> Emulador {
    let mut e = Emulador::nuevo(ejemplo_8()).unwrap();
    e.cargar_programa(0, palabras).unwrap();
    e
}

#[test]
fn dos_ejecuciones_identicas_son_iguales() {
    let veredicto = comparar(&mut con(&SUMA), &mut con(&SUMA), 100);
    assert_eq!(veredicto, Veredicto::Iguales { instrucciones: 4 });
}

#[test]
fn un_programa_distinto_se_nota_desde_la_primera_instruccion_por_la_memoria() {
    let mut otra = SUMA;
    otra[3] = 9; // LDI B,9
    match comparar(&mut con(&SUMA), &mut con(&otra), 100) {
        Veredicto::Divergen {
            instruccion,
            diferencias,
            ..
        } => {
            assert_eq!(instruccion, 0);
            assert!(
                diferencias.iter().any(|d| d.starts_with("memoria:")),
                "{diferencias:?}"
            );
        }
        v => panic!("{v:?}"),
    }
}

#[test]
fn una_diferencia_de_ciclos_se_detecta() {
    let mut maquina = ejemplo_8();
    for i in &mut maquina.instrucciones {
        if i.mnemonico == "ADD" {
            i.ciclos = Ciclos::Fijo(3);
        }
    }
    let mut otra = Emulador::nuevo(maquina).unwrap();
    otra.cargar_programa(0, &SUMA).unwrap();
    match comparar(&mut con(&SUMA), &mut otra, 100) {
        Veredicto::Divergen {
            instruccion,
            diferencias,
            ..
        } => {
            assert_eq!(instruccion, 2);
            assert!(diferencias.iter().any(|d| d.starts_with("ciclos:")));
        }
        v => panic!("{v:?}"),
    }
}

#[test]
fn una_diferencia_solo_en_memoria_se_detecta() {
    // ST A,[0x2000] ; HLT, con distinto valor en A
    let programa = [0x50, 0x00, 0x20, 0xF0];
    let mut a = con(&programa);
    let mut b = con(&programa);
    a.estado_mut().fijar_registro("A", 1).unwrap();
    b.estado_mut().fijar_registro("A", 1).unwrap();
    b.memoria_mut().escribir_privilegiado(0x2005, 3).unwrap();
    match comparar(&mut a, &mut b, 100) {
        Veredicto::Divergen { diferencias, .. } => {
            assert!(diferencias.iter().any(|d| d.starts_with("memoria:")));
        }
        v => panic!("{v:?}"),
    }
}

#[test]
fn un_programa_que_no_termina_llega_al_limite() {
    // JZ 0 con Z encendida, en bucle
    let programa = [0x60, 0x00, 0x00];
    let mut a = con(&programa);
    let mut b = con(&programa);
    a.estado_mut().fijar_bandera("Z", true).unwrap();
    b.estado_mut().fijar_bandera("Z", true).unwrap();
    assert_eq!(
        comparar(&mut a, &mut b, 10),
        Veredicto::LimiteAlcanzado { limite: 10 }
    );
}

#[test]
fn dos_fallos_iguales_cuentan_como_iguales() {
    // 0x00 es NOP, pero 0x90 no es ninguna instruccion
    let programa = [0x00, 0x90];
    let veredicto = comparar(&mut con(&programa), &mut con(&programa), 100);
    assert_eq!(veredicto, Veredicto::Iguales { instrucciones: 1 });
}

#[test]
fn un_fallo_frente_a_una_ejecucion_correcta_diverge() {
    let mut malo = SUMA;
    malo[0] = 0x90;
    match comparar(&mut con(&SUMA), &mut con(&malo), 100) {
        Veredicto::Divergen {
            instruccion,
            diferencias,
            ..
        } => {
            assert_eq!(instruccion, 0);
            assert!(diferencias[0].contains("fallo"), "{diferencias:?}");
        }
        v => panic!("{v:?}"),
    }
}

#[test]
fn la_vista_lista_los_registros_de_la_arquitectura() {
    let v = con(&SUMA).vista();
    let nombres: Vec<&str> = v.registros.iter().map(|(n, _)| n.as_str()).collect();
    assert!(nombres.contains(&"A") && nombres.contains(&"PC"));
    assert!(!v.detenida);
}

#[test]
fn dos_vistas_iguales_no_tienen_diferencias() {
    assert!(con(&SUMA)
        .vista()
        .diferencias(&con(&SUMA).vista())
        .is_empty());
}

// --- Con una implementacion que no es el emulador ---

struct Falso {
    pasos: u64,
    ciclos_extra: u64,
}

impl Ejecutor for Falso {
    fn paso(&mut self) -> Result<Option<Paso>, ErrorEjecucion> {
        self.pasos += 1;
        if self.pasos > 3 {
            return Ok(None);
        }
        Ok(Some(Paso {
            direccion: self.pasos,
            mnemonico: "X".into(),
            longitud: 1,
            ciclos: 1,
            salto: false,
            detenida: false,
        }))
    }

    fn vista(&self) -> Vista {
        Vista {
            pc: self.pasos,
            registros: vec![("A".into(), self.pasos)],
            instrucciones: self.pasos.min(3),
            ciclos: self.pasos.min(3) + self.ciclos_extra,
            detenida: false,
            memoria: 0,
        }
    }
}

#[test]
fn se_pueden_comparar_dos_implementaciones_ajenas_al_emulador() {
    let v = comparar(
        &mut Falso {
            pasos: 0,
            ciclos_extra: 0,
        },
        &mut Falso {
            pasos: 0,
            ciclos_extra: 0,
        },
        100,
    );
    assert!(matches!(v, Veredicto::Iguales { .. }));
}

#[test]
fn la_diferencia_se_describe_con_ambos_valores() {
    let v = comparar(
        &mut Falso {
            pasos: 0,
            ciclos_extra: 0,
        },
        &mut Falso {
            pasos: 0,
            ciclos_extra: 2,
        },
        100,
    );
    match v {
        Veredicto::Divergen { diferencias, .. } => {
            assert_eq!(diferencias, ["ciclos: 1 frente a 3"]);
        }
        v => panic!("{v:?}"),
    }
}
