//! One-number persistence that works the same natively and in the browser.
//!
//! It stores a single value — RIFT's best depth reached — as a plain number, so
//! the browser side needs no string marshalling and no `sapp_jsutils`: just two
//! number-in/number-out functions the accompanying `rift-storage.js` provides
//! via `localStorage`. Natively the same value lives in a small file.
//!
//! This is the only Breakout crate that uses `unsafe`, and only for the two FFI
//! calls below; everything else stays safe.

/// Reads the saved best depth, or 0 if none is saved.
pub fn best_depth() -> u32 {
    backend::get() as u32
}

/// Saves `depth` as the best depth reached.
pub fn set_best_depth(depth: u32) {
    backend::set(depth as f64);
}

#[cfg(target_arch = "wasm32")]
mod backend {
    unsafe extern "C" {
        fn rift_storage_get() -> f64;
        fn rift_storage_set(value: f64);
    }

    pub fn get() -> f64 {
        // Safety: the function is provided by rift-storage.js and only reads a
        // number out of localStorage.
        unsafe { rift_storage_get() }
    }

    pub fn set(value: f64) {
        // Safety: the function is provided by rift-storage.js and only writes a
        // number into localStorage.
        unsafe { rift_storage_set(value) }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use std::fs;

    const FILE: &str = "rift-best.txt";

    pub fn get() -> f64 {
        fs::read_to_string(FILE)
            .ok()
            .and_then(|text| text.trim().parse::<f64>().ok())
            .unwrap_or(0.0)
    }

    pub fn set(value: f64) {
        let _ = fs::write(FILE, (value as u32).to_string());
    }
}
