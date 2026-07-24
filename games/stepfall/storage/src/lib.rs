//! Small cross-platform persistence for HAILFALL: a handful of numbers that work
//! the same natively and in the browser.
//!
//! Everything is a plain number in a numbered slot, so the browser side needs no
//! string marshalling and no `sapp_jsutils`: just an indexed number-in/number-out
//! pair the accompanying `stepfall-storage.js` maps onto `localStorage`. Natively
//! the same slots live in a small file.
//!
//! This is the only STEPFALL crate that uses `unsafe`, and only for the two FFI
//! calls in the wasm backend; everything else stays safe.

/// Slot for the best Onslaught score.
const ONSLAUGHT_BEST: usize = 0;
/// Slot for the day a Daily best belongs to.
const DAILY_DAY: usize = 1;
/// Slot for the best score reached on that day.
const DAILY_BEST: usize = 2;

/// How many slots the store holds — room to spare for Phase B's unlocks.
const SLOTS: usize = 8;

/// Reads the best Onslaught score, or 0 if none is saved.
pub fn onslaught_best() -> u32 {
    backend::get(ONSLAUGHT_BEST) as u32
}

/// Saves `score` as the best Onslaught score.
pub fn set_onslaught_best(score: u32) {
    backend::set(ONSLAUGHT_BEST, score as f64);
}

/// The best score reached on calendar day `day`, or 0 if the saved Daily best is
/// for a different day (a fresh day starts from nothing).
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

#[cfg(target_arch = "wasm32")]
mod backend {
    unsafe extern "C" {
        fn stepfall_storage_get(slot: i32) -> f64;
        fn stepfall_storage_set(slot: i32, value: f64);
    }

    pub fn get(slot: usize) -> f64 {
        // Safety: the function is provided by stepfall-storage.js and only reads a
        // number out of localStorage.
        unsafe { stepfall_storage_get(slot as i32) }
    }

    pub fn set(slot: usize, value: f64) {
        // Safety: the function is provided by stepfall-storage.js and only writes a
        // number into localStorage.
        unsafe { stepfall_storage_set(slot as i32, value) }
    }
}

// One test, not several: the slots live in a single file, so parallel tests would
// race on it. This one exercises both bests in sequence and restores the real save.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn the_bests_round_trip_and_the_daily_belongs_to_its_day() {
        // Borrow the real save and put it back, so running the suite never
        // clobbers a player's progress.
        let (original_onslaught, original_day, original_daily) = (
            onslaught_best(),
            backend::get(DAILY_DAY) as u32,
            backend::get(DAILY_BEST) as u32,
        );

        // The Onslaught best round-trips.
        set_onslaught_best(4_242);
        assert_eq!(onslaught_best(), 4_242, "a saved score reads back exactly");
        set_onslaught_best(0);
        assert_eq!(onslaught_best(), 0, "an unsaved score reads back as zero");

        // The Daily best belongs to its day.
        let day = 20_000; // some day well clear of any real save
        set_daily_best(day, 900);
        assert_eq!(daily_best(day), 900, "the day's best reads back");
        assert_eq!(
            daily_best(day + 1),
            0,
            "a different day starts from nothing"
        );

        // Put the real save back exactly as it was.
        set_onslaught_best(original_onslaught);
        backend::set(DAILY_DAY, f64::from(original_day));
        backend::set(DAILY_BEST, f64::from(original_daily));
        assert_eq!(
            onslaught_best(),
            original_onslaught,
            "the real save is restored"
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use super::SLOTS;
    use std::fs;

    const FILE: &str = "stepfall-save.txt";

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
