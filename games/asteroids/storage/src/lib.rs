//! Small cross-platform persistence for ACCRETE: a handful of numbers that work the
//! same natively and in the browser.
//!
//! Everything is a plain number in a numbered slot, so the browser side needs no
//! string marshalling and no `sapp_jsutils`: just an indexed number-in/number-out pair
//! the accompanying `asteroids-storage.js` maps onto `localStorage`. Natively the same
//! slots live in a small file.
//!
//! This is the only ACCRETE crate that uses `unsafe`, and only for the two FFI calls
//! in the wasm backend; everything else stays safe. Phase A keeps just the two bests;
//! the meta's unlock set and Ascension tier take further slots in Phase B.

/// Slot for the best Maelstrom score.
const MAELSTROM_BEST: usize = 0;
/// Slot for the day a Daily best belongs to.
const DAILY_DAY: usize = 1;
/// Slot for the best score reached on that day.
const DAILY_BEST: usize = 2;
/// Slot for the bitset of unlocked ship options (Phase B's meta).
const UNLOCKED: usize = 3;

/// How many slots the store holds — room to spare for Phase B.
const SLOTS: usize = 8;

/// Reads the best Maelstrom score, or 0 if none is saved.
pub fn maelstrom_best() -> u32 {
    backend::get(MAELSTROM_BEST) as u32
}

/// Saves `score` as the best Maelstrom score.
pub fn set_maelstrom_best(score: u32) {
    backend::set(MAELSTROM_BEST, score as f64);
}

/// The best score reached on calendar day `day`, or 0 if the saved Daily best is for a
/// different day (a fresh day starts from nothing).
pub fn daily_best(day: u32) -> u32 {
    if backend::get(DAILY_DAY) as u32 == day {
        backend::get(DAILY_BEST) as u32
    } else {
        0
    }
}

/// Saves `score` as the best for calendar day `day`.
pub fn set_daily_best(day: u32, score: u32) {
    backend::set(DAILY_DAY, day as f64);
    backend::set(DAILY_BEST, score as f64);
}

/// The saved bitset of unlocked ship options, or 0 if nothing is saved. The meaning of
/// the bits belongs to the game's `meta`, which reads 0 as a fresh player — this crate
/// only keeps the number.
pub fn unlocked_bits() -> u32 {
    backend::get(UNLOCKED) as u32
}

/// Saves the bitset of unlocked ship options.
pub fn set_unlocked_bits(bits: u32) {
    backend::set(UNLOCKED, f64::from(bits));
}

#[cfg(target_arch = "wasm32")]
mod backend {
    unsafe extern "C" {
        fn asteroids_storage_get(slot: i32) -> f64;
        fn asteroids_storage_set(slot: i32, value: f64);
    }

    pub fn get(slot: usize) -> f64 {
        // Safety: the function is provided by asteroids-storage.js and only reads a
        // number out of localStorage.
        unsafe { asteroids_storage_get(slot as i32) }
    }

    pub fn set(slot: usize, value: f64) {
        // Safety: the function is provided by asteroids-storage.js and only writes a
        // number into localStorage.
        unsafe { asteroids_storage_set(slot as i32, value) }
    }
}

// One test, not several: the slots live in a single file, so parallel tests would race
// on it. This one exercises both bests in sequence and restores the real save.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn the_bests_round_trip_and_the_daily_belongs_to_its_day() {
        // Borrow the real save and put it back, so running the suite never clobbers a
        // player's progress.
        let (original_maelstrom, original_day, original_daily) = (
            maelstrom_best(),
            backend::get(DAILY_DAY) as u32,
            backend::get(DAILY_BEST) as u32,
        );

        // The Maelstrom best round-trips.
        set_maelstrom_best(4_242);
        assert_eq!(maelstrom_best(), 4_242, "a saved score reads back exactly");
        set_maelstrom_best(0);
        assert_eq!(maelstrom_best(), 0, "an unsaved score reads back as zero");

        // The Daily best belongs to its day.
        let day = 20_000; // some day well clear of any real save
        set_daily_best(day, 900);
        assert_eq!(daily_best(day), 900, "the day's best reads back");
        assert_eq!(
            daily_best(day + 1),
            0,
            "a different day starts from nothing"
        );

        // The unlocked bitset round-trips.
        let original_unlocked = unlocked_bits();
        set_unlocked_bits(0b10_1101);
        assert_eq!(
            unlocked_bits(),
            0b10_1101,
            "the unlock bits read back exactly"
        );

        // Put the real save back exactly as it was.
        set_maelstrom_best(original_maelstrom);
        backend::set(DAILY_DAY, f64::from(original_day));
        backend::set(DAILY_BEST, f64::from(original_daily));
        set_unlocked_bits(original_unlocked);
        assert_eq!(
            maelstrom_best(),
            original_maelstrom,
            "the real save is restored"
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use super::SLOTS;
    use std::fs;

    const FILE: &str = "asteroids-save.txt";

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
