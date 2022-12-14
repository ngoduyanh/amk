use super::{loops::{NUM_GAME_LOOPS, GameLoopKind}, manager::{MAIN_THREAD_ID, MAX_RUNNERS}};

pub struct Mode {
    thread_ids: [usize; NUM_GAME_LOOPS],
    relative_frequencies: [f64; NUM_GAME_LOOPS],
    pub(crate) thread_frequencies: [f64; MAX_RUNNERS + 1],
}

impl Mode {
    pub fn new() -> Self {
        Self {
            thread_ids: [MAIN_THREAD_ID; NUM_GAME_LOOPS],
            relative_frequencies: [1.0; NUM_GAME_LOOPS],
            thread_frequencies: [0.0; MAX_RUNNERS + 1]
        }
    }

    fn set(mut self, kind: GameLoopKind, thread_id: usize, relative_frequency: f64) -> Self {
        let index = kind.index();
        self.thread_ids[index] = thread_id;
        self.relative_frequencies[index] = relative_frequency;
        self
    }

    pub fn update(self, thread_id: usize, relative_frequency: f64) -> Self {
        self.set(GameLoopKind::Update, thread_id, relative_frequency)
    }

    pub fn render(self, thread_id: usize, relative_frequency: f64) -> Self {
        self.set(GameLoopKind::Render, thread_id, relative_frequency)
    }

    pub fn audio(self, thread_id: usize, relative_frequency: f64) -> Self {
        self.set(GameLoopKind::Audio, thread_id, relative_frequency)
    }

    pub fn frequency(mut self, thread_id: usize, frequency: f64) -> Self {
        self.thread_frequencies[thread_id] = frequency;
        self
    }

    pub(crate) fn get(&self, kind: GameLoopKind) -> (usize, f64) {
        let index = kind.index();
        (self.thread_ids[index], self.relative_frequencies[index])
    }
}
