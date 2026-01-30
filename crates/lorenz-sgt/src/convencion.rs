//! Convencion de llamada de cada maquina, escrita a partir de su descripcion (RF-SGT-027).
//!
//! El texto no es una plantilla fija: los registros, las instrucciones de salto y los operadores
//! que se resuelven con rutinas salen de lo que el compilador reconoce en la maquina.

use lorenz_arch::modelo::Maquina;

use crate::ast::OpBinario;
use crate::destino::Destino;
use crate::emisor::Emisor;
use crate::generador::{repartir_registros, sustitucion_para};
use crate::seleccion::Forma;
use crate::tipos::Tipo;

/// Documento en Markdown con la convencion de llamada de `maq`. Falla si el compilador no puede
/// usar la maquina (por ejemplo, si le faltan registros).
pub fn describir(maq: &Maquina) -> Result<String, String> {
    let d = Destino::de(maq);
    let mut em = Emisor::nuevo(maq);
    em.preparar_registros()
        .map_err(|f| format!("le falta {}", f.operacion))?;
    let temporales = repartir_registros(&mut em, 0);
    let s0 = em.s0();
    let s1 = em.s1();
    let mnemonico = |f: &dyn Fn(&Forma) -> bool| -> String {
        em.perfil
            .instrucciones
            .iter()
            .find(|i| f(&i.forma))
            .map(|i| format!("`{}`", i.mnemonico))
            .unwrap_or_else(|| "(la maquina no tiene)".to_string())
    };
    let saltar = mnemonico(&|f| matches!(f, Forma::Saltar(_)));
    let saltar_reg = mnemonico(&|f| matches!(f, Forma::SaltarReg(_)));
    let mut t = String::new();
    t.push_str(&format!(
        "# Convencion de llamada de SGT en `{}`\n\n",
        maq.nombre
    ));
    t.push_str(&format!(
        "Palabra de {} bits, direcciones de {} bits.\n\n",
        d.palabra, d.direccion
    ));
    t.push_str("## Registros\n\n| Registro | Uso |\n| --- | --- |\n");
    t.push_str(&format!(
        "| `{s0}` | valor de retorno y resultado de cada expresion |\n"
    ));
    t.push_str(&format!(
        "| `{s1}` | auxiliar: direccion de retorno, operandos intermedios |\n"
    ));
    for r in &temporales {
        t.push_str(&format!(
            "| `{r}` | temporal de expresiones (se guarda en una celda si hay una llamada de por medio) |\n"
        ));
    }
    if let Some((gp, _)) = &em.gp {
        t.push_str(&format!(
            "| `{gp}` | apunta al inicio de los datos; permite leer y escribir con registro mas desplazamiento |\n"
        ));
    }
    let propios: Vec<&str> = maq
        .registros
        .iter()
        .map(|r| r.nombre.as_str())
        .filter(|n| !em.pool.iter().any(|p| p == n) && !matches!(&em.gp, Some((g, _)) if g == n))
        .collect();
    if !propios.is_empty() {
        let lista = propios
            .iter()
            .map(|r| format!("`{r}`"))
            .collect::<Vec<_>>()
            .join(", ");
        t.push_str(&format!(
            "\nEl compilador no usa {lista}: queda libre para ensamblador en linea y para `registro(...)`.\n"
        ));
    }
    t.push_str("\n## Llamada\n\n");
    t.push_str(
        "1. Quien llama evalua los argumentos, de izquierda a derecha, en registros temporales.\n\
         2. Guarda en celdas fijas los temporales que sigan vivos (el compilador no usa pila de datos: decision D-059).\n\
         3. Escribe cada argumento en la celda del parametro correspondiente de la funcion llamada.\n",
    );
    t.push_str(&format!(
        "4. Escribe en la celda de retorno de la funcion llamada la direccion a la que debe volver, y salta a ella con {saltar}.\n"
    ));
    t.push_str(&format!(
        "5. La funcion llamada, al terminar, lee su celda de retorno en `{s1}` y vuelve con {saltar_reg}. El valor que devuelve queda en `{s0}`.\n\n"
    ));
    t.push_str(
        "Las variables locales y los parametros viven en celdas fijas de la region de datos, una por variable y funcion. Por eso no hay recursion; el compilador la rechaza.\n\n",
    );
    t.push_str("## Interrupciones\n\n");
    t.push_str(
        "Un manejador (`@interrupcion(n)`) guarda en un area propia todos los registros que usa el compilador, ejecuta su cuerpo y los repone antes de volver con la instruccion de retorno de interrupcion. Cada manejador recibe su propia copia de las funciones que llama (decision D-063), porque la rutina interrumpida podria estar ejecutando la misma.\n\n",
    );
    t.push_str("## Operadores resueltos con rutinas\n\n");
    let sustituye = sustitucion_para(maq);
    let u = Tipo::Entero {
        bits: d.palabra,
        signo: false,
    };
    let i = Tipo::Entero {
        bits: d.palabra,
        signo: true,
    };
    let mut filas = Vec::new();
    for (op, tipo) in [
        (OpBinario::Mul, &u),
        (OpBinario::Div, &u),
        (OpBinario::Mod, &u),
        (OpBinario::Div, &i),
        (OpBinario::Mod, &i),
        (OpBinario::DespIzq, &u),
        (OpBinario::DespDer, &u),
        (OpBinario::DespDer, &i),
        (OpBinario::SumaSat, &u),
        (OpBinario::RestaSat, &u),
        (OpBinario::SumaSat, &i),
        (OpBinario::RestaSat, &i),
    ] {
        if let Some(f) = sustituye(op, tipo) {
            filas.push(format!("| `{}` sobre {tipo} | `{f}` |\n", op.simbolo()));
        }
    }
    if filas.is_empty() {
        t.push_str("La maquina tiene instruccion para todos los operadores.\n");
    } else {
        t.push_str("| Operador | Rutina de la biblioteca |\n| --- | --- |\n");
        filas.iter().for_each(|f| t.push_str(f));
    }
    Ok(t)
}
