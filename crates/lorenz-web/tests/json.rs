use lorenz_web::Json;

#[test]
fn ida_y_vuelta() {
    let t = r#"{"a":[1,2.5,-3,true,null],"b":"x\ny\"z","c":{}}"#;
    let j = Json::analizar(t).unwrap();
    assert_eq!(Json::analizar(&j.serializar()).unwrap(), j);
    assert_eq!(j.get("a").unwrap().como_lista().unwrap().len(), 5);
    assert_eq!(j.get("b").unwrap().como_texto(), Some("x\ny\"z"));
}

#[test]
fn numeros() {
    assert_eq!(Json::analizar("1e3").unwrap().como_entero(), Some(1000));
    assert_eq!(Json::analizar("-0.5").unwrap().como_num(), Some(-0.5));
    assert_eq!(Json::analizar("1.5").unwrap().como_entero(), None);
    for malo in ["01x", "-", "1.", ".5", "1e", "1e999", "+1"] {
        assert!(Json::analizar(malo).is_err(), "{malo}");
    }
    assert_eq!(Json::Num(f64::NAN).serializar(), "null");
    assert_eq!(Json::Num(42.0).serializar(), "42");
}

#[test]
fn textos_y_escapes() {
    assert_eq!(
        Json::analizar(r#""é😀""#).unwrap().como_texto(),
        Some("é😀")
    );
    assert!(Json::analizar(r#""\ud83d""#).is_err());
    assert!(Json::analizar(r#""\udc00""#).is_err());
    assert!(Json::analizar("\"a\nb\"").is_err());
    assert!(Json::analizar(r#""\q""#).is_err());
    assert!(Json::analizar("\"abc").is_err());
    let s = Json::texto("</script>\u{1}").serializar();
    assert!(!s.contains('<') && s.contains("\\u0001"));
}

#[test]
fn estructura_invalida() {
    for malo in [
        "",
        "[1,]",
        "{\"a\"}",
        "{\"a\":1,}",
        "[1 2]",
        "nul",
        "{1:2}",
        "[1] x",
        "{\"a\":1",
    ] {
        assert!(Json::analizar(malo).is_err(), "{malo}");
    }
    assert!(Json::analizar(" [ ] ").is_ok());
}

#[test]
fn profundidad_limitada() {
    let ok = format!("{}{}", "[".repeat(60), "]".repeat(60));
    assert!(Json::analizar(&ok).is_ok());
    let mal = format!("{}{}", "[".repeat(5000), "]".repeat(5000));
    assert!(Json::analizar(&mal).unwrap_err().contains("anidamiento"));
}

#[test]
fn constructores() {
    let j = Json::objeto([
        ("n", 3usize.into()),
        ("ok", true.into()),
        ("l", vec![Json::Nulo].into()),
    ]);
    assert_eq!(j.serializar(), r#"{"l":[null],"n":3,"ok":true}"#);
}
