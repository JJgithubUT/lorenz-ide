//! El nucleo de Lorenz como modulo WebAssembly (RF-ECO-029).
//!
//! No usa `wasm-bindgen`: el modulo exporta una interfaz C minima. El anfitrion reserva memoria con
//! `lorenz_reservar`, copia ahi los textos, llama a `lorenz_ejecutar` y lee la respuesta (cuatro
//! bytes de longitud en little endian y el texto UTF-8) que devuelve el puntero.

use lorenz_emu::Emulador;
use lorenz_ens::ensamblador::{Ensamblador, Fuente};

/// Ensambla `fuente` para la maquina descrita en `maquina` (TOML) y la ejecuta hasta que se detenga
/// o pase `limite` instrucciones. El resultado es texto: `ok`, instrucciones, ciclos y registros;
/// o `error` y las lineas del problema.
pub fn ejecutar_texto(maquina: &str, fuente: &str, limite: u64) -> Result<String, String> {
    let m = lorenz_arch::cargar(maquina).map_err(|e| e.to_string())?;
    let inicio = m.arranque;
    let nombres: Vec<String> = m.registros.iter().map(|r| r.nombre.clone()).collect();
    let ens = Ensamblador::nuevo(&m)?;
    let objeto = ens
        .ensamblar(&Fuente::nueva("programa.s", fuente))
        .map_err(|errores| {
            errores
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        })?;
    let mut emulador = Emulador::nuevo(m).map_err(|e| e.to_string())?;
    for s in &objeto.segmentos {
        emulador
            .cargar_programa(s.direccion, &s.palabras)
            .map_err(|e| e.to_string())?;
    }
    emulador.estado_mut().fijar_pc(inicio);
    let resumen = emulador.ejecutar(limite).map_err(|e| e.to_string())?;
    let mut salida = format!(
        "ok\ninstrucciones {}\nciclos {}\npc {}\n",
        resumen.instrucciones,
        resumen.ciclos,
        emulador.estado().pc()
    );
    for n in nombres {
        if let Ok(v) = emulador.estado().registro(&n) {
            salida.push_str(&format!("{n} {v}\n"));
        }
    }
    Ok(salida)
}

fn empaquetar(texto: String) -> *mut u8 {
    let mut v = Vec::with_capacity(4 + texto.len());
    v.extend_from_slice(&(texto.len() as u32).to_le_bytes());
    v.extend_from_slice(texto.as_bytes());
    let mut caja = v.into_boxed_slice();
    let p = caja.as_mut_ptr();
    std::mem::forget(caja);
    p
}

/// Reserva `n` bytes para que el anfitrion copie un texto.
#[no_mangle]
pub extern "C" fn lorenz_reservar(n: usize) -> *mut u8 {
    let mut caja = vec![0u8; n].into_boxed_slice();
    let p = caja.as_mut_ptr();
    std::mem::forget(caja);
    p
}

/// Libera `n` bytes reservados por este modulo (una reserva o una respuesta con su cabecera).
///
/// # Safety
/// `p` y `n` deben ser los de una reserva o respuesta de este modulo, y no usarse despues.
#[no_mangle]
pub unsafe extern "C" fn lorenz_liberar(p: *mut u8, n: usize) {
    if !p.is_null() {
        drop(Box::from_raw(std::slice::from_raw_parts_mut(p, n)));
    }
}

/// Ejecuta un programa. Devuelve un puntero a la respuesta (longitud de 4 bytes y texto); si el
/// texto empieza por `ok` fue bien, y si empieza por `error` lo que sigue es el motivo.
///
/// # Safety
/// Cada puntero debe apuntar a tantos bytes UTF-8 validos como diga su longitud.
#[no_mangle]
pub unsafe extern "C" fn lorenz_ejecutar(
    maquina: *const u8,
    maquina_n: usize,
    fuente: *const u8,
    fuente_n: usize,
    limite: u64,
) -> *mut u8 {
    let m = std::str::from_utf8(std::slice::from_raw_parts(maquina, maquina_n));
    let f = std::str::from_utf8(std::slice::from_raw_parts(fuente, fuente_n));
    let texto = match (m, f) {
        (Ok(m), Ok(f)) => match ejecutar_texto(m, f, limite) {
            Ok(t) => t,
            Err(e) => format!("error\n{e}"),
        },
        _ => "error\nel texto no es UTF-8".to_string(),
    };
    empaquetar(texto)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn ejecuta_el_ejemplo_y_da_los_registros() {
        let m = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
        let p = std::fs::read_to_string("../../ejemplos/ejemplo-8/programas/suma.s").unwrap();
        let s = ejecutar_texto(&m, &p, 10_000).unwrap();
        assert!(s.starts_with("ok\n"), "{s}");
        assert!(s.contains("instrucciones "));
    }

    #[test]
    fn un_error_de_ensamblado_se_explica() {
        let m = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
        let e = ejecutar_texto(&m, "        NOEXISTE\n", 100).unwrap_err();
        assert!(e.to_lowercase().contains("noexiste"), "{e}");
    }

    #[test]
    fn la_interfaz_c_devuelve_longitud_y_texto() {
        let m = std::fs::read_to_string("../../ejemplos/ejemplo-8/maquina.toml").unwrap();
        let p = "        HLT\n";
        let r = unsafe { lorenz_ejecutar(m.as_ptr(), m.len(), p.as_ptr(), p.len(), 100) };
        let n = u32::from_le_bytes(
            unsafe { std::slice::from_raw_parts(r, 4) }
                .try_into()
                .unwrap(),
        ) as usize;
        let t = unsafe { std::slice::from_raw_parts(r.add(4), n) };
        assert!(t.starts_with(b"ok"));
        unsafe { lorenz_liberar(r, 4 + n) };
    }
}
