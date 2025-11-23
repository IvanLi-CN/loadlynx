#[derive(Clone, Debug)]
pub struct Config {
    pub tail_default: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tail_default: 200,
        }
    }
}
