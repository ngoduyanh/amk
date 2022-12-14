use std::{
    sync::mpsc::{Receiver, TryRecvError},
    time::{Duration, Instant},
};

use winit::event_loop::ControlFlow;

use crate::utils::sync::{new_clock_sync, ClockSync};

use super::{
    loop_impl::{AudioLoop, EventLoop, RenderLoop, UpdateLoop},
    loops::{GameLoop, GameLoopContainer, GameLoopKind, NUM_GAME_LOOPS},
    mode::Mode,
    msg::ELGLMMsg,
    runner::Runner,
};

pub type WinitEventLoop = winit::event_loop::EventLoop<()>;

pub const MAX_RUNNERS: usize = NUM_GAME_LOOPS;
pub const MAIN_THREAD_ID: usize = MAX_RUNNERS;

pub struct GameLoopManager {
    runners: [Option<Runner>; MAX_RUNNERS],
    // main thread stuff
    loops: GameLoopContainer,
    exec_mode: Mode,
    event_loop: EventLoop,
    clock_sync: Box<dyn ClockSync>,
}

pub trait DropData {
    fn do_nothing(&self) {}
}

impl GameLoopManager {
    pub fn new(
        event_loop: EventLoop,
        update_loop: UpdateLoop,
        render_loop: RenderLoop,
        audio_loop: AudioLoop,
    ) -> Self {
        let mut loops = GameLoopContainer::new();
        loops.insert(GameLoopKind::Update, Box::new(update_loop), 1.0);
        loops.insert(GameLoopKind::Render, Box::new(render_loop), 1.0);
        loops.insert(GameLoopKind::Audio, Box::new(audio_loop), 1.0);
        Self {
            runners: Default::default(),
            clock_sync: new_clock_sync(),
            exec_mode: Mode::new(),
            loops,
            event_loop,
        }
    }

    pub fn new_moded(
        event_loop: EventLoop,
        update_loop: UpdateLoop,
        render_loop: RenderLoop,
        audio_loop: AudioLoop,
        exec_mode: Mode,
    ) -> Self {
        let mut manager = Self::new(event_loop, update_loop, render_loop, audio_loop);
        manager.set_mode(exec_mode);
        manager
    }

    fn request_loop(&mut self, kind: GameLoopKind) -> Box<dyn GameLoop> {
        let (thread_id, _) = self.exec_mode.get(kind);
        if thread_id == MAIN_THREAD_ID {
            self.loops.get(kind).unwrap()
        } else {
            self.runners[thread_id].as_ref().unwrap().request_loop(kind)
        }
    }

    fn get_or_create_runner(&mut self, thread_id: usize) {
        if self.runners[thread_id].is_none() {
            self.runners[thread_id] = Some(Runner::new());
        }
    }

    fn send_loop(
        &mut self,
        kind: GameLoopKind,
        gl: Box<dyn GameLoop>,
        thread_id: usize,
        relative_frequency: f64,
    ) {
        if thread_id == MAIN_THREAD_ID {
            self.loops.data[kind.index()] = Some(gl)
        } else {
            self.get_or_create_runner(thread_id);
            self.runners[thread_id]
                .as_ref()
                .unwrap()
                .send_loop(kind, gl, relative_frequency);
        }
    }

    fn set_thread_frequency(&mut self, thread_id: usize, frequency: f64) {
        self.get_or_create_runner(thread_id);
        self.runners[thread_id]
            .as_ref()
            .unwrap()
            .set_frequency(frequency);
    }

    fn set_relative_frequency(
        &mut self,
        kind: GameLoopKind,
        thread_id: usize,
        relative_frequency: f64,
    ) {
        if thread_id == MAIN_THREAD_ID {
            self.loops.set_relative_frequency(kind, relative_frequency)
        } else {
            self.runners[thread_id]
                .as_ref()
                .unwrap()
                .set_relative_frequency(kind, relative_frequency)
        }
    }

    fn set_mode(&mut self, new_mode: Mode) {
        for kind in [
            GameLoopKind::Update,
            GameLoopKind::Render,
            GameLoopKind::Audio,
        ] {
            let (new_thread_id, new_relative_frequency) = new_mode.get(kind);
            let (old_thread_id, _) = self.exec_mode.get(kind);

            if new_thread_id != old_thread_id {
                let gl = self.request_loop(kind);
                self.send_loop(kind, gl, new_thread_id, new_relative_frequency);
            } else {
                self.set_relative_frequency(kind, new_thread_id, new_relative_frequency);
            }
        }

        for i in 0..MAX_RUNNERS {
            self.set_thread_frequency(i, new_mode.thread_frequencies[i]);
        }

        self.exec_mode = new_mode;
    }

    pub fn run(mut self, window_loop: WinitEventLoop, elglm_receiver: Receiver<ELGLMMsg>) -> ! {
        window_loop.run(move |evt, _, cf| {
            *cf = if self.loops.empty() {
                ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(100))
            } else {
                ControlFlow::Poll
            };
            match evt {
                winit::event::Event::MainEventsCleared => {
                    loop {
                        match elglm_receiver.try_recv() {
                            Err(TryRecvError::Empty) => break,
                            r => match r.unwrap() {
                                ELGLMMsg::SetMode(mode) => self.set_mode(mode),
                                ELGLMMsg::Stop => {
                                    *cf = ControlFlow::Exit;
                                }
                            },
                        }
                    }
                    self.loops.run().expect("Error running game loops");
                    self.clock_sync
                        .sync(self.exec_mode.thread_frequencies[MAIN_THREAD_ID]);
                }
                e => self.event_loop.run(e),
            }
        });
    }
}
