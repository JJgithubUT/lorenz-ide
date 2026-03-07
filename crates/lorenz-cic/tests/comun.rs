#![allow(dead_code)]
use lorenz_cic::{cargar_config, Simulador};
use lorenz_emu::Emulador;

pub const LDI_A: u8 = 0x20;
pub const LDI_B: u8 = 0x24;
pub const LDI_C: u8 = 0x28;
pub const ADD_AB: u8 = 0x31;
pub const ADD_CA: u8 = 0x38;
pub const HLT: u8 = 0xF0;

pub fn texto_maquina() -> String {
    std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap()
}

/// Un simulador sobre el ejemplo de 8 bits, con la temporizacion extra y el programa dados.
pub fn simulador(extra: &str, programa: &[u8]) -> Simulador {
    let texto = format!("{}\n{extra}\n", texto_maquina());
    let m = lorenz_arch::cargar(&texto).expect("la maquina debe cargar");
    let cfg = cargar_config(&texto).expect("la temporizacion debe cargar");
    let mut e = Emulador::nuevo(m).unwrap();
    let palabras: Vec<u64> = programa.iter().map(|b| u64::from(*b)).collect();
    e.cargar_programa(0, &palabras).unwrap();
    Simulador::nuevo(e, cfg).unwrap()
}

/// Como `simulador`, pero con la latencia de la region `ram` cambiada.
pub fn simulador_con_ram_lenta(extra: &str, latencia: u64, programa: &[u8]) -> Simulador {
    let base = texto_maquina();
    let marca = "nombre = \"ram\"";
    assert!(base.contains(marca));
    let texto = base.replacen(marca, &format!("{marca}\nlatencia = {latencia}"), 1);
    let texto = format!("{texto}\n{extra}\n");
    let m = lorenz_arch::cargar(&texto).expect("la maquina debe cargar");
    let cfg = cargar_config(&texto).expect("la temporizacion debe cargar");
    let mut e = Emulador::nuevo(m).unwrap();
    let palabras: Vec<u64> = programa.iter().map(|b| u64::from(*b)).collect();
    e.cargar_programa(0, &palabras).unwrap();
    Simulador::nuevo(e, cfg).unwrap()
}

pub const CINCO: &str = "[pipeline]\netapas = [\"IF\",\"ID\",\"EX\",\"MEM\",\"WB\"]\n";
