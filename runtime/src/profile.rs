/// A profile stores the list of <bandwidth, accuracy, configuration>. The
/// simple implementation uses a list and performs binary search for items.
use csv;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::path::Path;

/// Record is each individual rule in a profile.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Record<C> {
    pub bandwidth: f64,
    pub config: C,
    _accuracy: f64,
}

/// A `SimpleProfile` isn't parameterized by the config.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SimpleProfile {
    /// A list of bandwidths
    levels: Vec<f64>,

    /// The current config (serving as cache)
    current: usize,
}

impl SimpleProfile {
    /// Get current profile
    #[inline]
    pub fn current(&self) -> usize {
        self.current
    }

    /// Finds the index of the configuration that matches (equal or smaller
    /// than) the provided bandwidth.
    fn get_level_index(&self, bw: f64) -> usize {
        let pos = (&self.levels).binary_search_by(|v| {
            v.partial_cmp(&bw).expect("failed to compare bandwidth")
        });
        match pos {
            Ok(i) => i,
            // If error, it could be the first (only 1 profile) or the last
            // (fail to find).
            Err(i) => if i == 0 { 0 } else { i - 1 },
        }
    }

    /// Adjusts the profile with a configuration that satisfies the provided
    /// bandwidth, i.e., equal or smaller. Returns a tuple of bandwidth and
    /// configuration.
    pub fn adjust_level(&mut self, bw: f64) -> Option<usize> {
        let new_level = self.get_level_index(bw);
        if self.current != new_level {
            self.current = new_level;
            Some(new_level)
        } else {
            None
        }
    }

    /// Advances to next config. Returns the record if successful; otherwise,
    /// return None (when we cannot advance any more).
    pub fn advance_level(&mut self) -> Option<usize> {
        if self.current < self.levels.len() - 1 {
            self.current += 1;
            Some(self.current)
        } else {
            None
        }
    }

    /// Finds out the required rate for next configuration.
    pub fn next_rate(&self) -> Option<f64> {
        if self.current < self.levels.len() - 1 {
            Some(self.levels[self.current + 1])
        } else {
            None
        }
    }

    /// Am I current at maximum allowed configuration?
    pub fn is_max(&self) -> bool {
        self.current == self.levels.len() - 1
    }
}

/// Profile is each individual rule in a profile.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Profile<C> {
    /// `SimpleProfile` takes care of indexing, search for appropriate levels.
    simple_profile: SimpleProfile,

    /// A reference list with detailed configurations and bandwidth/accuracy.
    records: Vec<Record<C>>,
}

impl<C: Copy> Profile<C> {
    /// Returns the n-th configuration (we will simply do vector indexing).
    pub fn nth(&self, level: usize) -> C {
        self.records[level].config
    }

    /// Returns the initial configuration (we will simply take the first).
    pub fn init_config(&self) -> C {
        self.records
            .first()
            .expect("no configuration in profile")
            .config
    }

    /// Returns the best configuration (we will simply take the last).
    pub fn last_config(&self) -> C {
        self.records
            .last()
            .expect("no configuration in profile")
            .config
    }

    /// Returns the current configuration.
    pub fn current_config(&self) -> C {
        self.records[self.simple_profile.current()].config
    }

    /// Returns the current level.
    pub fn current_level(&self) -> usize {
        self.simple_profile.current()
    }
}

impl<C> Profile<C> {
    /// Creates a new profile using a vector containing all the records. For
    /// testing purpose.
    pub fn _with_vec(vec: Vec<Record<C>>) -> Profile<C> {
        let simple = vec.iter().map(|r| r.bandwidth).collect();
        let simple_profile = SimpleProfile {
            levels: simple,
            current: 0,
        };
        Profile {
            records: vec,
            simple_profile: simple_profile,
        }
    }
    pub fn simplify(&self) -> SimpleProfile {
        self.simple_profile.clone()
    }
}

impl<C: Debug + Copy> Profile<C> {
    /// Adjusts the profile with a configuration that satisfies the provided
    /// bandwidth, i.e., equal or smaller. Returns a tuple of bandwidth and
    /// configuration.
    pub fn adjust_config(&mut self, bw: f64) -> Option<Record<C>> {
        match self.simple_profile.adjust_level(bw) {
            Some(new_level) => {
                info!(
                    "updating to level {}, configuration {:?}",
                    new_level,
                    self.records[new_level]
                );
                Some(self.records[new_level])
            }
            None => None,
        }
    }

    /// Advances to next config. Returns the record if successful; otherwise,
    /// return None (when we cannot advance any more).
    pub fn advance_config(&mut self) -> Option<Record<C>> {
        match self.simple_profile.advance_level() {
            Some(new_level) => {
                info!(
                    "updating to level {}, configuration {:?}",
                    new_level,
                    self.records[new_level]
                );
                Some(self.records[new_level])
            }
            None => None,
        }
    }
}

impl<C: DeserializeOwned + Copy + Debug> Profile<C> {
    /// Creates a new `Profile` instance with a path pointing to the profile
    /// file (CSV). The columns in the file needs to match the config type.
    /// Because this is the loading phase, we bail early (use expect!).
    pub fn new<P: AsRef<Path>>(path: P) -> Profile<C> {
        let errmsg = format!("no profile file {:?}", path.as_ref());
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_path(path)
            .expect(&errmsg);
        let mut vec = Vec::new();
        for record in rdr.deserialize() {
            let record: Record<C> = record.expect("failed to parse the record");
            vec.push(record);
        }

        let simple = vec.iter().map(|r| r.bandwidth).collect();
        Profile {
            records: vec,
            simple_profile: SimpleProfile {
                levels: simple,
                current: 0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, Clone, Copy, Debug)]
    struct DummyConfig {
        pub v: usize,
    }

    fn create_profile(i: usize) -> Profile<DummyConfig> {
        let mut vec = Vec::new();
        // Populate sample test data
        // 1.0, 2.0, ...
        for i in 0..i {
            let c = DummyConfig { v: i };
            let record = Record {
                bandwidth: i as f64,
                config: c,
                _accuracy: 0.0,
            };
            vec.push(record);
        }
        Profile::_with_vec(vec)
    }

    #[test]
    fn test_profile_simple_get() {
        let mut profile = create_profile(4);
        assert_eq!(profile.init_config().v, 0);
        assert_eq!(profile.last_config().v, 3);
        assert_eq!(profile.current_config().v, 0);
        assert_eq!(profile.adjust_config(4.0).unwrap().config.v, 3);
        assert_eq!(profile.adjust_config(1.5).unwrap().config.v, 1);
    }

    #[test]
    fn test_profile_with_one_record() {
        let mut profile = create_profile(1);
        assert_eq!(profile.init_config().v, 0);;
        assert_eq!(profile.last_config().v, 0);
        assert_eq!(profile.current_config().v, 0);
        assert!(profile.adjust_config(1.5).is_none());
    }
}
