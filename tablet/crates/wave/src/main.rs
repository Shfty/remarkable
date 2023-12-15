use libremarkable::{
    cgmath,
    input::{ev::EvDevContext, multitouch::MultitouchEvent, InputDevice, InputEvent},
};

use shared::TAP_HYSTERESIS;

use gesture::{recognize_drag, GestureRecognizer};

use std::sync::mpsc::channel;

fn main() -> ! {
    println!("wave startup");

    // Create an MPSC channel to receive input events
    let (input_tx, input_rx) = channel::<InputEvent>();

    // Start event channels
    println!("Starting event channel...");

    let mut multitouch = EvDevContext::new(InputDevice::Multitouch, input_tx);

    multitouch.start();

    let mut gesture_recognizer =
        GestureRecognizer::default().with_callback(gesture::recognize_starting_zone(
            cgmath::Point2::new(0, libremarkable::dimensions::DISPLAYHEIGHT - 128),
            cgmath::Vector2::new(libremarkable::dimensions::DISPLAYWIDTH, 128),
            recognize_drag(move |delta| {
                if delta.y > TAP_HYSTERESIS {
                    true
                } else {
                    false
                }
            }),
        ));

    // Enter event loop
    println!("Entering event loop...");
    while let Ok(event) = input_rx.recv() {
        match event {
            InputEvent::MultitouchEvent { event } => {
                println!("{event:?}");
                let res = match event {
                    MultitouchEvent::Press { finger } => gesture_recognizer.finger_press(finger),
                    MultitouchEvent::Release { finger } => {
                        gesture_recognizer.finger_release(finger)
                    }
                    MultitouchEvent::Move { finger } => gesture_recognizer.finger_move(finger),
                    _ => vec![],
                };

                if res.len() > 0 {
                    multitouch.stop();
                    println!("Gesture triggered, spawning tray process");
                    std::process::Command::new("/home/root/tray")
                        .spawn()
                        .unwrap()
                        .wait()
                        .unwrap();
                    multitouch.start();
                }
            }
            _ => (),
        }
    }

    panic!("Event loops closed unexpectedly");
}
