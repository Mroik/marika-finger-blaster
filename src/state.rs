pub struct State {
    pub current: usize,
    pub buffer: String,
}

impl Default for State {
    fn default() -> Self {
        State {
            current: 0,
            buffer: String::new(),
        }
    }
}
