pub struct State {
    pub current: usize,
    pub buffer: String,
    pub state: StateEnum,
}

pub enum StateEnum {
    Typing,
    Results(u8),
}

impl Default for State {
    fn default() -> Self {
        State {
            current: 0,
            buffer: String::new(),
            state: StateEnum::Typing,
        }
    }
}
