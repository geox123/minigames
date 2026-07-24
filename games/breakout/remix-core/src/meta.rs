//! The cross-run meta: what a player has unlocked, and what a run earns them.
//!
//! This is pure, deterministic bookkeeping and knows nothing about storage, the
//! shell or the clock. The run rules never see it — the core only ever receives
//! a [`Pool`], and this module is simply one way to build one. That keeps the
//! whole meta layer additive: it can grow without touching a single rule.
//!
//! A player starts with a small kit so a first run is never barren, and earns the
//! rest by playing: deeper descents, higher scores, wins, and wins at Ascension
//! tiers.

use super::{Boon, Kind, Pool};

/// A piece of content a player can unlock: a special brick kind, or a boon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Content {
    /// A special brick kind walls may include.
    Brick(Kind),
    /// A boon drafts may offer.
    Boon(Boon),
}

/// Every unlockable piece of content, bricks first then boons — the order the
/// collection screen shows them in, and the order of their bits.
pub const ALL: [Content; 11] = [
    Content::Brick(Kind::Armoured),
    Content::Brick(Kind::Explosive),
    Content::Brick(Kind::Mirror),
    Content::Brick(Kind::Mover),
    Content::Brick(Kind::Spawner),
    Content::Boon(Boon::Pierce),
    Content::Boon(Boon::Blast),
    Content::Boon(Boon::Widen),
    Content::Boon(Boon::Swift),
    Content::Boon(Boon::ExtraLife),
    Content::Boon(Boon::Fortune),
];

/// The kit a player begins with: one brick kind and three boons, so the first
/// wall has some variety and the first draft is always a full three.
const STARTING: [Content; 4] = [
    Content::Brick(Kind::Armoured),
    Content::Boon(Boon::Widen),
    Content::Boon(Boon::ExtraLife),
    Content::Boon(Boon::Fortune),
];

impl Content {
    /// This content's stable bit position in an [`Unlocked`] set.
    pub fn index(self) -> u32 {
        // `ALL` is the single source of truth for bit order.
        ALL.iter()
            .position(|c| *c == self)
            .expect("every Content is listed in ALL") as u32
    }

    /// A short name for the collection screen.
    pub fn label(self) -> &'static str {
        match self {
            Content::Brick(Kind::Armoured) => "ARMOURED",
            Content::Brick(Kind::Explosive) => "EXPLOSIVE",
            Content::Brick(Kind::Mirror) => "MIRROR",
            Content::Brick(Kind::Mover) => "MOVER",
            Content::Brick(Kind::Spawner) => "SPAWNER",
            Content::Brick(Kind::Normal) => "NORMAL",
            Content::Boon(boon) => boon.title(),
        }
    }

    /// How this content is earned, for the collection screen.
    pub fn condition(self) -> &'static str {
        match self {
            _ if STARTING.contains(&self) => "FROM THE START",
            Content::Brick(Kind::Explosive) => "REACH DEPTH 2",
            Content::Boon(Boon::Swift) => "SCORE 300 IN A RUN",
            Content::Brick(Kind::Mirror) => "REACH DEPTH 3",
            Content::Boon(Boon::Pierce) => "SCORE 600 IN A RUN",
            Content::Boon(Boon::Blast) => "WIN A RUN",
            Content::Brick(Kind::Mover) => "WIN AT ASCENSION TIER 1",
            Content::Brick(Kind::Spawner) => "WIN AT ASCENSION TIER 2",
            _ => "FROM THE START",
        }
    }

    /// Whether a run with this `outcome` earns this content.
    fn earned_by(self, outcome: Outcome) -> bool {
        match self {
            _ if STARTING.contains(&self) => true,
            Content::Brick(Kind::Explosive) => outcome.depth >= 2,
            Content::Boon(Boon::Swift) => outcome.score >= 300,
            Content::Brick(Kind::Mirror) => outcome.depth >= 3,
            Content::Boon(Boon::Pierce) => outcome.score >= 600,
            Content::Boon(Boon::Blast) => outcome.won,
            Content::Brick(Kind::Mover) => outcome.won && outcome.tier >= 1,
            Content::Brick(Kind::Spawner) => outcome.won && outcome.tier >= 2,
            _ => true,
        }
    }
}

/// What a finished run achieved, as the meta reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// The run beat the final guardian.
    pub won: bool,
    /// The depth it reached, 1-based.
    pub depth: u32,
    /// Its score.
    pub score: u32,
    /// The Ascension tier it played at (0 for a plain Run or Daily).
    pub tier: u32,
}

/// The content a player has unlocked, as a small bitset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unlocked(u32);

impl Default for Unlocked {
    fn default() -> Self {
        Self::starting()
    }
}

impl Unlocked {
    /// The kit every player begins with.
    pub fn starting() -> Self {
        let mut unlocked = Self(0);
        for content in STARTING {
            unlocked.unlock(content);
        }
        unlocked
    }

    /// Rebuilds a set from its saved bits. Bits outside the known content are
    /// ignored, and an empty save yields the starting kit rather than nothing.
    pub fn from_bits(bits: u32) -> Self {
        if bits == 0 {
            return Self::starting();
        }
        let known = ALL.iter().fold(0, |mask, c| mask | (1 << c.index()));
        Self(bits & known)
    }

    /// The bits to save.
    pub fn bits(self) -> u32 {
        self.0
    }

    /// Whether `content` is unlocked.
    pub fn has(self, content: Content) -> bool {
        self.0 & (1 << content.index()) != 0
    }

    /// Unlocks `content`, reporting whether it was newly earned.
    pub fn unlock(&mut self, content: Content) -> bool {
        let newly = !self.has(content);
        self.0 |= 1 << content.index();
        newly
    }

    /// Every unlocked piece of content, in [`ALL`] order.
    pub fn iter(self) -> impl Iterator<Item = Content> {
        ALL.into_iter().filter(move |c| self.has(*c))
    }

    /// The pool a run should draw on: exactly the unlocked bricks and boons.
    pub fn pool(self) -> Pool {
        let mut specials = Vec::new();
        let mut boons = Vec::new();
        for content in self.iter() {
            match content {
                Content::Brick(kind) => specials.push(kind),
                Content::Boon(boon) => boons.push(boon),
            }
        }
        Pool { specials, boons }
    }

    /// Records a finished run, unlocking whatever it earned and returning only
    /// the newly-earned content — so the shell can announce it. Recording the
    /// same outcome again returns nothing.
    pub fn record(&mut self, outcome: Outcome) -> Vec<Content> {
        let mut newly = Vec::new();
        for content in ALL {
            if content.earned_by(outcome) && self.unlock(content) {
                newly.push(content);
            }
        }
        newly
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_content_has_a_unique_bit() {
        let mut seen = Vec::new();
        for content in ALL {
            let index = content.index();
            assert!(index < 32, "the set must fit in its bits");
            assert!(!seen.contains(&index), "{content:?} shares a bit");
            seen.push(index);
        }
        assert_eq!(seen.len(), ALL.len());
    }

    #[test]
    fn a_player_starts_with_a_playable_kit() {
        let unlocked = Unlocked::starting();
        for content in STARTING {
            assert!(unlocked.has(content), "{content:?} should start unlocked");
        }
        let pool = unlocked.pool();
        assert_eq!(pool.specials.len(), 1, "one brick kind to begin with");
        assert_eq!(
            pool.boons.len(),
            3,
            "three boons, so a first draft is always full"
        );
    }

    #[test]
    fn the_set_round_trips_through_its_bits() {
        let mut unlocked = Unlocked::starting();
        unlocked.unlock(Content::Brick(Kind::Explosive));
        let restored = Unlocked::from_bits(unlocked.bits());
        assert_eq!(restored, unlocked);
        assert!(restored.has(Content::Brick(Kind::Explosive)));
    }

    #[test]
    fn an_empty_save_reads_back_as_the_starting_kit() {
        assert_eq!(Unlocked::from_bits(0), Unlocked::starting());
    }

    #[test]
    fn a_pool_offers_exactly_what_is_unlocked() {
        let mut unlocked = Unlocked::starting();
        unlocked.unlock(Content::Brick(Kind::Mirror));
        unlocked.unlock(Content::Boon(Boon::Blast));
        let pool = unlocked.pool();

        assert!(pool.specials.contains(&Kind::Mirror));
        assert!(pool.boons.contains(&Boon::Blast));
        assert!(
            !pool.specials.contains(&Kind::Spawner),
            "locked content is not offered"
        );
        assert!(!pool.boons.contains(&Boon::Pierce));

        // The full pool is still everything, so a full-content run is unaffected.
        let base = Pool::base();
        assert_eq!(base.specials.len(), 5);
        assert_eq!(base.boons.len(), 6);
    }

    #[test]
    fn a_deep_run_unlocks_the_depth_content() {
        let mut unlocked = Unlocked::starting();
        let newly = unlocked.record(Outcome {
            won: false,
            depth: 2,
            score: 0,
            tier: 0,
        });
        assert_eq!(newly, vec![Content::Brick(Kind::Explosive)]);
        assert!(unlocked.has(Content::Brick(Kind::Explosive)));
        assert!(
            !unlocked.has(Content::Brick(Kind::Mirror)),
            "depth 3 content stays locked"
        );
    }

    #[test]
    fn recording_the_same_outcome_twice_earns_nothing_new() {
        let mut unlocked = Unlocked::starting();
        let outcome = Outcome {
            won: false,
            depth: 3,
            score: 400,
            tier: 0,
        };
        let first = unlocked.record(outcome);
        assert!(!first.is_empty(), "the first recording earns content");
        let second = unlocked.record(outcome);
        assert!(second.is_empty(), "nothing is earned twice");
    }

    #[test]
    fn winning_at_a_high_tier_unlocks_the_deepest_content() {
        let mut unlocked = Unlocked::starting();
        let newly = unlocked.record(Outcome {
            won: true,
            depth: 3,
            score: 900,
            tier: 2,
        });
        for content in ALL {
            assert!(
                unlocked.has(content),
                "{content:?} should be unlocked by a tier-2 win"
            );
        }
        assert!(newly.contains(&Content::Brick(Kind::Spawner)));
        assert!(newly.contains(&Content::Boon(Boon::Blast)));
    }
}
