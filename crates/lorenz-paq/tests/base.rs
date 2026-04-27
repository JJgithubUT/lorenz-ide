use lorenz_paq::*;

#[test]
fn sha256_coincide_con_los_vectores_oficiales() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
    // varios bloques y el borde de 55/56/64 bytes
    let mil = vec![b'a'; 1_000_000];
    assert_eq!(
        sha256_hex(&mil),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
    for n in [55usize, 56, 63, 64, 65] {
        assert_eq!(sha256_hex(&vec![b'x'; n]).len(), 64);
    }
}

#[test]
fn las_versiones_se_leen_y_se_ordenan() {
    assert_eq!(Version::parsear("1.2.3").unwrap(), Version::nueva(1, 2, 3));
    assert_eq!(Version::parsear("2").unwrap(), Version::nueva(2, 0, 0));
    assert_eq!(Version::parsear("1.5").unwrap().to_string(), "1.5.0");
    assert!(Version::parsear("1.2.3.4").is_err());
    assert!(Version::parsear("uno").is_err());
    assert!(Version::parsear("").is_err());
    assert!(Version::nueva(1, 10, 0) > Version::nueva(1, 9, 9));
}

#[test]
fn los_requisitos_filtran_versiones() {
    let r = |t: &str| Requisito::parsear(t).unwrap();
    let v = |a, b, c| Version::nueva(a, b, c);
    assert!(r("*").cumple(v(9, 9, 9)));
    assert!(r("1.2.0").cumple(v(1, 2, 0)) && !r("1.2.0").cumple(v(1, 2, 1)));
    assert!(r(">=1.0").cumple(v(1, 0, 0)) && !r(">=1.0").cumple(v(0, 9, 0)));
    assert!(r(">1").cumple(v(1, 0, 1)) && !r(">1").cumple(v(1, 0, 0)));
    assert!(r("<2").cumple(v(1, 9, 9)) && !r("<2").cumple(v(2, 0, 0)));
    assert!(r("<=2").cumple(v(2, 0, 0)));
    assert!(
        r("^1.2").cumple(v(1, 9, 0))
            && !r("^1.2").cumple(v(2, 0, 0))
            && !r("^1.2").cumple(v(1, 1, 0))
    );
    assert!(r("^0.3").cumple(v(0, 3, 5)) && !r("^0.3").cumple(v(0, 4, 0)));
    assert_eq!(r(">=1.0").to_string(), ">=1.0.0");
    assert!(Requisito::parsear(">=x").is_err());
}
