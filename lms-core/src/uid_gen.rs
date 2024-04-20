use lazy_static::lazy_static;
use rand::rngs::ThreadRng;
use rand::Rng;

lazy_static! {
    static ref PUSH_CHARS: Vec<char> =
        "-0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyz"
            .chars()
            .collect();
}

pub struct UidGenerator {
    last_rand_chars: [usize; 12],
    last_push_time: u128,
    rand_gen: ThreadRng,
}

impl Default for UidGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl UidGenerator {
    pub fn new() -> Self {
        let rand_gen = rand::thread_rng();
        UidGenerator {
            last_rand_chars: [0; 12],
            last_push_time: 0,
            rand_gen,
        }
    }

    pub fn generate(mut self, now: u128) -> String {
        let duplicate_time = now == self.last_push_time;
        self.last_push_time = now;

        let mut time_stamp_chars: [char; 8] = ['0'; 8];
        let mut temp_now = now;
        for i in (0..8).rev() {
            time_stamp_chars[i] = PUSH_CHARS[(temp_now % 64) as usize];
            temp_now /= 64;
        }
        debug_assert!(temp_now == 0);

        let mut result = time_stamp_chars.iter().collect::<String>();

        if !duplicate_time {
            for i in 0..12 {
                self.last_rand_chars[i] = self.rand_gen.gen_range(0..64);
            }
        } else {
            self = self.increment_array();
        }

        for &rand_char_idx in self.last_rand_chars.iter() {
            result.push(PUSH_CHARS[rand_char_idx]);
        }
        debug_assert_eq!(result.len(), 20);

        result
    }

    fn increment_array(mut self) -> Self {
        for i in (0..12).rev() {
            if self.last_rand_chars[i] != 63 {
                self.last_rand_chars[i] += 1;
                return self;
            }
            self.last_rand_chars[i] = 0;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // assures that no assertion fails

    #[test]
    fn test_uid_generator() {
        let uid_gen = UidGenerator::new();
        let now = 1632499259;
        let uid = uid_gen.generate(now);
        assert_eq!(uid.len(), 20);
    }

    #[test]
    fn test_uid_uniqueness() {
        let now = 1632499259;
        let mut uids = HashSet::new();
        for _ in 0..1000 {
            let uid_gen = UidGenerator::new();
            let uid = uid_gen.generate(now);
            uids.insert(uid);
        }
        assert_eq!(uids.len(), 1000);
    }

    #[test]
    fn test_uid_increment() {
        let uid_gen = UidGenerator::new();
        let now = 1632499259;
        let uid1 = uid_gen.generate(now);
        let uid_gen = UidGenerator::new();
        let uid2 = uid_gen.generate(now);
        assert_ne!(uid1, uid2);
    }
}
