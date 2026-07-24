//! Small cross-platform persistence for RIFT: a handful of numbers that work the
//! same natively and in the browser.
//!
//! Everything is a plain number in a numbered slot, so the browser side needs no
//! string marshalling and no `sapp_jsutils`: just an indexed number-in/number-out
//! pair the accompanying `rift-storage.js` maps onto `localStorage`. Natively the
//! same slots live in a small file.
//!
//! This is the only Breakout crate that uses `unsafe`, and only for the two FFI
//! calls in the wasm backend; everything else stays safe.

/// Slot for the best depth reached in a Run.
const BEST_DEPTH: usize = 0;
/// Slot for the day a Daily best belongs to.
const DAILY_DAY: usize = 1;
/// Slot for the best depth reached on that day.
const DAILY_BEST: usize = 2;
/// Slot for the highest Ascension tier reached.
const ASCENSION_TIER: usize = 3;
/// Slot for the bitset of unlocked content.
const UNLOCKED: usize = 4;

/// How many slots the store holds (room to spare for later).
const SLOTS: usize = 8;

/// Reads the best depth reached in a Run, or 0 if none is saved.
pub fn best_depth() -> u32 {
    backend::get(BEST_DEPTH) as u32
}

/// Saves `depth` as the best depth reached in a Run.
pub fn set_best_depth(depth: u32) {
    backend::set(BEST_DEPTH, depth as f64);
}

/// The best depth reached on calendar day `day`, or 0 if the saved Daily best is
/// for a different day (a fresh day starts from nothing).
pub fn daily_best(day: u32) -> u32 {
    if backend::get(DAILY_DAY) as u32 == day {
        backend::get(DAILY_BEST) as u32
    } else {
        0
    }
}

/// Saves `depth` as the best for calendar day `day`.
pub fn set_daily_best(day: u32, depth: u32) {
    backend::set(DAILY_DAY, day as f64);
    backend::set(DAILY_BEST, depth as f64);
}

/// The highest Ascension tier reached (0 if none).
pub fn ascension_tier() -> u32 {
    backend::get(ASCENSION_TIER) as u32
}

/// Saves `tier` as the highest Ascension tier reached.
pub fn set_ascension_tier(tier: u32) {
    backend::set(ASCENSION_TIER, tier as f64);
}

/// The saved bitset of unlocked content, or 0 if nothing is saved. The meaning
/// of the bits belongs to the game's meta, which reads 0 as "a fresh player" and
/// hands back the starting kit — this crate only keeps the number.
pub fn unlocked_bits() -> u32 {
    backend::get(UNLOCKED) as u32
}

/// Saves the bitset of unlocked content.
pub fn set_unlocked_bits(bits: u32) {
    backend::set(UNLOCKED, bits as f64);
}

#[cfg(target_arch = "wasm32")]
mod backend {
    unsafe extern "C" {
        fn rift_storage_get(slot: i32) -> f64;
        fn rift_storage_set(slot: i32, value: f64);
    }

    pub fn get(slot: usize) -> f64 {
        // Safety: the function is provided by rift-storage.js and only reads a
        // number out of localStorage.
        unsafe { rift_storage_get(slot as i32) }
    }

    pub fn set(slot: usize, value: f64) {
        // Safety: the function is provided by rift-storage.js and only writes a
        // number into localStorage.
        unsafe { rift_storage_set(slot as i32, value) }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn the_unlock_bitset_round_trips() {
        // Borrow the real save and put it back, so running the suite never
        // clobbers a player's progress.
        let original = unlocked_bits();

        set_unlocked_bits(0b1010_1010);
        assert_eq!(unlocked_bits(), 0b1010_1010, "saved bits read back exactly");

        set_unlocked_bits(0);
        assert_eq!(unlocked_bits(), 0, "an unsaved set reads back as zero");

        set_unlocked_bits(original);
        assert_eq!(unlocked_bits(), original, "the real save is left as it was");
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use super::SLOTS;
    use std::fs;

    const FILE: &str = "rift-save.txt";

    fn read() -> Vec<f64> {
        let mut slots = vec![0.0; SLOTS];
        if let Ok(text) = fs::read_to_string(FILE) {
            for (slot, word) in text.split_whitespace().take(SLOTS).enumerate() {
                slots[slot] = word.parse().unwrap_or(0.0);
            }
        }
        slots
    }

    pub fn get(slot: usize) -> f64 {
        read().get(slot).copied().unwrap_or(0.0)
    }

    pub fn set(slot: usize, value: f64) {
        let mut slots = read();
        if slot < slots.len() {
            slots[slot] = value;
        }
        let line = slots
            .iter()
            .map(|n| (*n as u32).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let _ = fs::write(FILE, line);
    }
}
