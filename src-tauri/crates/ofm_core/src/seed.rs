//! The game's one seed, and the streams of randomness derived from it.
//!
//! Every random choice the game makes on the day path — a random event, an injury,
//! which way a training session falls, how a match plays out — should come from a
//! generator derived from [`Game::seed`], so the same save played with the same
//! inputs gives the same world, and a game saved and reloaded mid-season carries on
//! exactly as it would have. A call to `rand::rng()` breaks that, because it draws
//! from the operating system and leaves nothing to replay.
//!
//! One seed, stored once on the game, with a stream derived per purpose rather than
//! one generator shared by everything: a stream drawn from a shared generator
//! shifts whenever any other subsystem draws one more number, so adding a feature
//! would silently re-roll every other feature's results.

use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;

use crate::game::Game;
use crate::stable_hash::stable_hash;

/// Mixed into the id of a save that predates seeds, so its seed is not simply a
/// hash of the id that something else might also be computing.
const UNSEEDED_SAVE_SALT: u64 = 0x5eed_5a7e_0000_0001;

impl Game {
    /// A random generator for one purpose on one day.
    ///
    /// `tag` names the purpose and must carry whatever tells two uses on the same
    /// day apart — `"match/<home>/<away>"`, not `"match"` — because the same tag on
    /// the same date is the same stream by design.
    ///
    /// Nothing here depends on a library's choice of default. The key is expanded
    /// from [`stable_hash`], which promises the same answer on every toolchain, and
    /// the generator is ChaCha12 by name rather than `StdRng`, which rand reserves
    /// the right to change. So a saved career replays the same way after a
    /// dependency bump: `rng_for_is_pinned_to_its_first_draws` fails first if that
    /// stops being true.
    ///
    /// What this makes repeatable is *a save*: the same save, played with the same
    /// inputs, gives the same days. It does not make a *seed* regenerate a game,
    /// because world generation still mints ids with `Uuid::new_v4`, and tags carry
    /// ids — two worlds generated from one seed have different ids and so different
    /// streams.
    pub fn rng_for(&self, tag: &str, date: &str) -> ChaCha12Rng {
        let per_purpose = stable_hash(tag.as_bytes(), self.seed);
        expand_to_rng(stable_hash(date.as_bytes(), per_purpose), per_purpose)
    }
}

impl Game {
    /// [`rng_for`](Self::rng_for) for today's date, which is what nearly every caller
    /// on the day path wants.
    pub fn rng_today(&self, tag: &str) -> ChaCha12Rng {
        let today = self.clock.current_date.format("%Y-%m-%d").to_string();
        self.rng_for(tag, &today)
    }
}

/// A generator for something described by `key` alone, for the builders of news and
/// messages that are handed only what the story is about and have no game to ask.
///
/// The same key is the same stream on every replay, so a builder that draws from it
/// writes the same story each time; different keys are different streams. Like
/// [`variant_for`] it is wording, so it carries no game seed.
pub fn rng_from_key(key: &str) -> ChaCha12Rng {
    expand_to_rng(
        stable_hash(key.as_bytes(), KEYED_STREAM_SALT),
        KEYED_STREAM_SALT,
    )
}

/// Keeps a keyed stream from being the same stream as a `rng_for` on the same text.
const KEYED_STREAM_SALT: u64 = 0x7a51_0000_7a51_0002;

/// Four hashes chained from `word`, each of the one before, make the 32-byte ChaCha key, so
/// every byte of it depends on everything `word` was made from.
fn expand_to_rng(mut word: u64, chain_salt: u64) -> ChaCha12Rng {
    let mut key = [0u8; 32];
    for chunk in key.chunks_exact_mut(8) {
        chunk.copy_from_slice(&word.to_le_bytes());
        word = stable_hash(&word.to_le_bytes(), chain_salt);
    }
    ChaCha12Rng::from_seed(key)
}

/// Which of `count` phrasings a message is written in, chosen from what the message is.
///
/// Wording is the one random choice that needs no seed: nothing depends on which of two
/// greetings an inbox item uses, and a pick made from `key` — the message's own id, which
/// names the event, the player and the day — is the same on every replay with no
/// generator to thread through every builder. Different events still get different
/// phrasings, because different keys hash differently.
///
/// `count` must be non-zero.
pub fn variant_for(key: &str, count: usize) -> usize {
    (stable_hash(key.as_bytes(), VARIANT_SALT) % count as u64) as usize
}

/// Keeps a phrasing pick from being the same number as any other use of `stable_hash`
/// on the same string.
const VARIANT_SALT: u64 = 0x7a51_0000_7a51_0001;

/// The seed a save written before seeds existed is given when it is first loaded.
///
/// Derived from the save's own id, so the same old save is given the same seed on
/// every machine and every load rather than a fresh random one; that is what lets
/// a bug report against an old save be replayed. It is stored once, on the first
/// load, and read back from then on.
pub fn seed_for_unseeded_save(save_id: &str) -> u64 {
    stable_hash(save_id.as_bytes(), UNSEEDED_SAVE_SALT)
}

#[cfg(test)]
mod tests {
    use domain::manager::Manager;
    use rand::RngExt;

    use super::*;
    use crate::clock::GameClock;
    use crate::world::start_date_for_year;

    fn draws(mut rng: ChaCha12Rng) -> Vec<u32> {
        (0..8).map(|_| rng.random()).collect()
    }

    fn game_with_seed(seed: u64) -> Game {
        let clock = GameClock::new(start_date_for_year(2032).expect("a valid start year"));
        let manager = Manager::new(
            "manager".to_string(),
            "A".to_string(),
            "B".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let mut game = Game::new(clock, manager, vec![], vec![], vec![], vec![]);
        game.seed = seed;
        game
    }

    /// Given a game and a purpose on a day,
    /// When a generator is asked for twice,
    /// Then both give the same numbers in the same order.
    #[test]
    fn the_same_seed_tag_and_date_give_the_same_stream() {
        let game = game_with_seed(7);
        assert_eq!(
            draws(game.rng_for("training", "2032-07-01")),
            draws(game.rng_for("training", "2032-07-01"))
        );
    }

    /// Given two fixtures on the same day,
    /// When each asks for its own generator,
    /// Then they do not share a stream.
    #[test]
    fn two_purposes_on_the_same_day_do_not_share_a_stream() {
        let game = game_with_seed(7);
        assert_ne!(
            draws(game.rng_for("match/fixture-a", "2032-07-01")),
            draws(game.rng_for("match/fixture-b", "2032-07-01"))
        );
    }

    /// Given the same purpose on two days,
    /// When each asks for a generator,
    /// Then they differ, so a daily roll is not the same roll every day.
    #[test]
    fn the_same_purpose_on_another_day_is_another_stream() {
        let game = game_with_seed(7);
        assert_ne!(
            draws(game.rng_for("training", "2032-07-01")),
            draws(game.rng_for("training", "2032-07-02"))
        );
    }

    /// Given two games with different seeds,
    /// When both ask for the same purpose on the same day,
    /// Then the streams differ: the seed is what makes a world its own.
    #[test]
    fn a_different_game_seed_is_a_different_stream() {
        assert_ne!(
            draws(game_with_seed(1).rng_for("training", "2032-07-01")),
            draws(game_with_seed(2).rng_for("training", "2032-07-01"))
        );
    }

    /// Given a story described by a key,
    /// When a generator is made for it, twice,
    /// Then both give the same numbers, and another story's differ.
    #[test]
    fn a_keyed_stream_is_the_same_for_the_same_story_and_another_for_another() {
        assert_eq!(
            draws(rng_from_key("news/roundup/eng-d1/4/2032-09-01")),
            draws(rng_from_key("news/roundup/eng-d1/4/2032-09-01"))
        );
        assert_ne!(
            draws(rng_from_key("news/roundup/eng-d1/4/2032-09-01")),
            draws(rng_from_key("news/roundup/eng-d1/5/2032-09-08"))
        );
    }

    /// Given a message key and some phrasings,
    /// When a phrasing is picked, twice,
    /// Then it is the same one and is always one of them — and across many keys every
    ///      phrasing is used, so the wording still varies.
    #[test]
    fn a_phrasing_is_picked_from_the_message_and_is_always_the_same() {
        for count in [2, 3] {
            let picks: Vec<usize> = (0..60)
                .map(|n| variant_for(&format!("low_morale_p{n}_2032-07-01"), count))
                .collect();
            assert_eq!(
                picks,
                (0..60)
                    .map(|n| variant_for(&format!("low_morale_p{n}_2032-07-01"), count))
                    .collect::<Vec<_>>()
            );
            for variant in 0..count {
                assert!(
                    picks.contains(&variant),
                    "phrasing {variant} of {count} never used"
                );
            }
            assert!(picks.iter().all(|&pick| pick < count));
        }
    }

    /// Given a known seed, purpose and day,
    /// When the generator is drawn from,
    /// Then the first numbers are these — the values a saved career's future is made of.
    ///
    /// A golden test: it fails the moment the algorithm, the key derivation or the hash
    /// changes, which is the point. Re-pin it only with a save-format bump that says a
    /// replay of older saves is knowingly not preserved.
    #[test]
    fn rng_for_is_pinned_to_its_first_draws() {
        let game = game_with_seed(7);

        assert_eq!(
            draws(game.rng_for("match/home/away", "2032-07-01")),
            vec![
                1_120_400_549,
                2_779_441_353,
                2_431_824_407,
                2_062_034_164,
                2_293_205_624,
                4_017_691_427,
                115_278_181,
                1_475_461_962
            ]
        );
    }

    /// Given a save that predates seeds,
    /// When it is given a seed,
    /// Then the same save always gets the same one, and different saves differ.
    #[test]
    fn an_unseeded_save_is_given_the_same_seed_every_time() {
        assert_eq!(
            seed_for_unseeded_save("save-one"),
            seed_for_unseeded_save("save-one")
        );
        assert_ne!(
            seed_for_unseeded_save("save-one"),
            seed_for_unseeded_save("save-two")
        );
    }

    /// The value is load-bearing: a stored seed decides a whole saved career, so
    /// changing the derivation would quietly re-roll every old save's future.
    /// Pinned rather than described.
    #[test]
    fn the_seed_given_to_an_old_save_is_pinned() {
        assert_eq!(
            seed_for_unseeded_save("save-one"),
            15_686_556_740_451_502_136
        );
    }
}
