//! A fixed-timestep accumulator.
//!
//! Games simulate in fixed steps so their behaviour is deterministic and
//! frame-rate-independent; the shell banks real frame time and hands out whole
//! steps. This banks the time and caps a single frame's contribution so one long
//! stall (a dragged window, a backgrounded tab) can't make the game try to
//! simulate minutes at once.

/// Accumulates real time into fixed steps of `timestep` seconds.
pub struct Accumulator {
    timestep: f32,
    max_frame: f32,
    banked: f32,
}

impl Accumulator {
    /// A new accumulator for steps of `timestep` seconds, banking at most
    /// `max_frame` seconds of real time per frame.
    pub fn new(timestep: f32, max_frame: f32) -> Self {
        Self {
            timestep,
            max_frame,
            banked: 0.0,
        }
    }

    /// Banks `frame_dt` real seconds and returns how many fixed steps are now
    /// due. Call `game.step()` that many times.
    pub fn steps(&mut self, frame_dt: f32) -> u32 {
        self.banked = (self.banked + frame_dt).min(self.max_frame);
        let mut count = 0;
        while self.banked >= self.timestep {
            self.banked -= self.timestep;
            count += 1;
        }
        count
    }

    /// Drops any banked time, so a pause or freeze doesn't fast-forward on
    /// resume.
    pub fn reset(&mut self) {
        self.banked = 0.0;
    }
}
