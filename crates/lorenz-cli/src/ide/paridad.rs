//! Paridad entre el depurador de linea de comandos y el grafico (RF-DEP-044).
//!
//! Cada orden de `lorenz depurar` tiene aqui su equivalente en la interfaz: el elemento grafico
//! que hace lo mismo. Ademas, el panel de ordenes acepta todas tal cual. Una prueba comprueba que
//! no se anada una orden al depurador sin decir donde esta en el IDE.

/// (orden de la CLI, elemento grafico que la cubre, descripcion).
/// El elemento es el `id` de un panel, o el de un boton de la barra con prefijo `accion-`.
#[rustfmt::skip]
pub const PARIDAD: &[(&str, &str, &str)] = &[
    ("paso", "accion-paso", "boton Paso (F10)"),
    ("continuar", "accion-ejecutar", "boton Ejecutar (F5)"),
    ("hasta", "ordenes", "solo en el panel de ordenes"),
    ("salir", "ordenes", "solo en el panel de ordenes"),
    ("registros", "registros", "panel Registros"),
    ("avisar", "ordenes", "solo en el panel de ordenes"),
    ("explicar", "dato", "panel Camino del dato"),
    ("volcar", "memoria", "editor hexadecimal"),
    ("desensamblar", "desensamblado", "panel Desensamblado"),
    ("pila", "ordenes", "solo en el panel de ordenes"),
    ("fijar", "registros", "clic en el valor de un registro"),
    ("escribir", "memoria", "clic en una celda"),
    ("puerto", "ordenes", "solo en el panel de ordenes"),
    ("entrada", "consola", "teclado de la consola"),
    ("inyectar", "ordenes", "solo en el panel de ordenes"),
    ("explorar", "ordenes", "solo en el panel de ordenes"),
    ("imprimir", "ordenes", "solo en el panel de ordenes"),
    ("ruptura", "rupturas", "panel Rupturas y margen del desensamblado"),
    ("observar", "ordenes", "solo en el panel de ordenes"),
    ("puntos", "rupturas", "panel Rupturas"),
    ("borrar", "rupturas", "panel Rupturas"),
    ("activar", "rupturas", "panel Rupturas (activar y desactivar)"),
    ("traza", "instante", "control de instante"),
    ("atras", "accion-atras", "boton Atras (Shift+F10) y control de instante (atras y adelante)"),
    ("instante", "instante", "control deslizante"),
    ("quien-escribio", "ordenes", "solo en el panel de ordenes"),
    ("quien-cambio", "ordenes", "solo en el panel de ordenes"),
    ("buscar", "ordenes", "solo en el panel de ordenes"),
    ("pantalla", "pantalla", "panel Pantalla"),
    ("fallo", "ordenes", "solo en el panel de ordenes"),
    ("ayuda", "ordenes", "orden ayuda"),
    ("fin", "ordenes", "cerrar la sesion con el boton de la barra"),
];

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn los_elementos_existen_en_la_interfaz() {
        let fuentes: String = crate::ide::ESTATICOS
            .iter()
            .map(|(_, t)| *t)
            .chain([crate::ide::PAGINA])
            .collect::<Vec<_>>()
            .join("\n");
        for (orden, elem, _) in PARIDAD {
            let existe = match elem.strip_prefix("accion-") {
                Some(b) => fuentes.contains(&format!("boton('{b}'")),
                None => fuentes.contains(&format!("id: '{elem}'")),
            };
            assert!(
                existe,
                "`{orden}` apunta a `{elem}`, que no existe en la interfaz"
            );
        }
    }

    #[test]
    fn no_hay_ordenes_repetidas() {
        let mut v: Vec<_> = PARIDAD.iter().map(|p| p.0).collect();
        v.sort_unstable();
        let n = v.len();
        v.dedup();
        assert_eq!(n, v.len());
    }
}
