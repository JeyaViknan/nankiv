//! SF Symbols for the interface, drawn by macOS.
//!
//! The interface asks for a symbol by the name it has in Apple's SF Symbols
//! app, at the size and weight it will be shown, and gets back an image the
//! system drew. nankiv ships none of the artwork: SF Symbols are licensed for
//! use on Apple platforms, the same repository builds for Windows, and so the
//! symbols have to come from the Mac the app is running on.
//!
//! Anywhere they cannot — Windows, macOS before 11, a symbol this Mac does not
//! have — the answer is `None` and the interface draws its own.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolRequest {
    /// As the SF Symbols app shows it, e.g. `gearshape` or `person.2`.
    pub name: String,
    pub point_size: f64,
    pub weight: Weight,
    /// Device pixels per point.
    pub scale: f64,
}

/// The nine weights of San Francisco, which the symbols share.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Weight {
    Ultralight,
    Thin,
    Light,
    Regular,
    Medium,
    Semibold,
    Bold,
    Heavy,
    Black,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SymbolImage {
    /// Size in points, which the interface lays out in CSS pixels.
    pub width: f64,
    pub height: f64,
    /// A PNG at `scale`, as a data URL the interface uses as a mask.
    pub url: String,
}

/// Only names shaped like SF Symbols reach AppKit. The call is harmless with
/// any string, but there is no reason to hand it one that cannot be a symbol.
pub fn plausible(request: &SymbolRequest) -> bool {
    let name = &request.name;
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.')
        && (4.0..=128.0).contains(&request.point_size)
        && (1.0..=4.0).contains(&request.scale)
}

/// Parses what the bridge returns: "width height base64png".
fn parse(raw: &str) -> Option<SymbolImage> {
    let mut parts = raw.splitn(3, ' ');
    let width: f64 = parts.next()?.parse().ok()?;
    let height: f64 = parts.next()?.parse().ok()?;
    let png = parts.next()?;
    (width > 0.0 && height > 0.0 && !png.is_empty()).then(|| SymbolImage {
        width,
        height,
        url: format!("data:image/png;base64,{png}"),
    })
}

#[cfg(target_os = "macos")]
pub fn render(request: &SymbolRequest) -> Option<SymbolImage> {
    use std::ffi::{c_char, CStr, CString};

    extern "C" {
        fn nankiv_symbol(
            name: *const c_char,
            point_size: f64,
            weight: i32,
            scale: f64,
        ) -> *mut c_char;
        fn nankiv_free_string(string: *mut c_char);
    }

    if !plausible(request) {
        return None;
    }
    let name = CString::new(request.name.as_str()).ok()?;
    // SAFETY: `name` is a valid C string for the duration of the call. The
    // bridge returns either null or a string it allocated, which is copied
    // here and then handed back to it to free, exactly once.
    unsafe {
        let raw = nankiv_symbol(
            name.as_ptr(),
            request.point_size,
            request.weight as i32,
            request.scale,
        );
        if raw.is_null() {
            return None;
        }
        let text = CStr::from_ptr(raw).to_str().ok().map(str::to_owned);
        nankiv_free_string(raw);
        parse(&text?)
    }
}

#[cfg(not(target_os = "macos"))]
pub fn render(_request: &SymbolRequest) -> Option<SymbolImage> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(name: &str) -> SymbolRequest {
        SymbolRequest {
            name: name.into(),
            point_size: 13.5,
            weight: Weight::Regular,
            scale: 2.0,
        }
    }

    #[test]
    fn only_symbol_shaped_names_are_asked_for() {
        assert!(plausible(&request("gearshape")));
        assert!(plausible(&request("person.2")));
        assert!(plausible(&request("tray.and.arrow.down")));
        for bad in [
            "",
            "Gearshape",
            "../etc",
            "gear shape",
            "a/b",
            &"x".repeat(65),
        ] {
            assert!(!plausible(&request(bad)), "{bad:?}");
        }
    }

    #[test]
    fn sizes_stay_within_reason() {
        let mut r = request("gearshape");
        r.point_size = 1000.0;
        assert!(!plausible(&r));
        let mut r = request("gearshape");
        r.scale = 0.0;
        assert!(!plausible(&r));
    }

    #[test]
    fn weights_are_in_font_order() {
        // The bridge indexes AppKit's weights by this number.
        assert_eq!(Weight::Ultralight as i32, 0);
        assert_eq!(Weight::Regular as i32, 3);
        assert_eq!(Weight::Black as i32, 8);
    }

    #[test]
    fn reads_what_the_bridge_returns() {
        assert_eq!(
            parse("17.5 16 iVBORw0KGgo="),
            Some(SymbolImage {
                width: 17.5,
                height: 16.0,
                url: "data:image/png;base64,iVBORw0KGgo=".into(),
            })
        );
        assert_eq!(parse("0 16 iVBOR"), None);
        assert_eq!(parse("17 16"), None);
        assert_eq!(parse("wide 16 iVBOR"), None);
    }

    /// The interface's icon names and the SF Symbols they are, one list for
    /// both sides.
    fn interface_symbols() -> Vec<String> {
        let map: std::collections::BTreeMap<String, String> =
            serde_json::from_str(include_str!("../../src/components/symbols.json"))
                .expect("symbols.json");
        map.into_values().collect()
    }

    #[test]
    fn the_interface_asks_only_for_plausible_names() {
        for name in interface_symbols() {
            assert!(plausible(&request(&name)), "{name}");
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_draws_every_symbol_the_interface_uses() {
        for name in interface_symbols() {
            let image = render(&request(&name)).unwrap_or_else(|| panic!("{name} missing"));
            assert!(image.width > 0.0 && image.height > 0.0, "{name}");
            assert!(
                image.url.starts_with("data:image/png;base64,iVBOR"),
                "{name}"
            );
        }
        assert_eq!(render(&request("not.a.real.symbol")), None);
    }
}
