use std::sync::Arc;

use crossbeam_channel::Sender;
use gesture::GestureRecognizer;
use libremarkable::framebuffer::core::Framebuffer;

use crate::{
    channel::Receiver,
    display::DISPLAY_RECT,
    ui::{Draw, DrawContext},
    MainEvent,
};

pub enum RenderEvent {
    Execute(Arc<Box<dyn Draw + Send + Sync>>, bool),
    Exit,
}

impl RenderEvent {
    pub fn execute<F: Draw + Send + Sync + 'static>(
        f: F,
        replace_gesture_recognizer: bool,
    ) -> Self {
        RenderEvent::Execute(Arc::new(Box::new(f)), replace_gesture_recognizer)
    }

    pub fn execute_boxed(
        f: &Arc<Box<dyn Draw + Send + Sync + 'static>>,
        replace_gesture_recognizer: bool,
    ) -> Self {
        RenderEvent::Execute(f.clone(), replace_gesture_recognizer)
    }

    pub fn exit() -> Self {
        RenderEvent::Exit
    }
}

pub fn render_thread(
    event_tx: Sender<MainEvent>,
    command_rx: Receiver<RenderEvent>,
) -> impl FnOnce() + Send + 'static {
    move || {
        let mut framebuffer = Framebuffer::new();

        loop {
            match command_rx.recv() {
                Ok(event) => match event {
                    RenderEvent::Execute(f, replace_gesture_recognizer) => {
                        let DrawContext {
                            fb,
                            gesture_recognizer,
                            ..
                        } = f.draw(DrawContext {
                            fb: framebuffer,
                            rect: DISPLAY_RECT,
                            gesture_recognizer: GestureRecognizer::default(),
                        });

                        framebuffer = fb;

                        if replace_gesture_recognizer {
                            event_tx
                                .send(MainEvent::SetGestureRecognizer(Some(gesture_recognizer)))
                                .unwrap();
                        }
                    }
                    RenderEvent::Exit => break,
                },
                Err(e) => panic!("{e:}"),
            }
        }
    }
}
