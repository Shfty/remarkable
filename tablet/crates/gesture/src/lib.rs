use libremarkable::{cgmath, cgmath::InnerSpace, input::multitouch::Finger};
use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

#[derive(Debug, Copy, Clone)]
pub enum EventType {
    Press,
    Move,
    Release,
}

#[derive(Debug, Default)]
pub struct FingerHistory(Vec<(EventType, Finger)>);

impl Deref for FingerHistory {
    type Target = Vec<(EventType, Finger)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FingerHistory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<(EventType, Finger)>> for FingerHistory {
    fn from(finger_history: Vec<(EventType, Finger)>) -> Self {
        FingerHistory(finger_history)
    }
}

impl FingerHistory {
    fn finger_delta(&self) -> Option<cgmath::Vector2<f32>> {
        let first_pos = self.first()?.1.pos;
        let last_pos = self.last()?.1.pos;

        Some(
            cgmath::Point2::<f32>::new(first_pos.x as f32, first_pos.y as f32)
                - cgmath::Point2::<f32>::new(last_pos.x as f32, last_pos.y as f32),
        )
    }
}

#[derive(Default)]
pub struct GestureRecognizer {
    active_fingers: BTreeMap<i32, FingerHistory>,
    callbacks: Vec<Box<dyn GestureCallback + Send + Sync>>,
}

pub trait GestureCallback: FnMut(&FingerHistory) -> Option<()> {}
impl<F> GestureCallback for F where F: FnMut(&FingerHistory) -> Option<()> {}

impl GestureRecognizer {
    pub fn with_callback<F>(mut self, f: F) -> Self
    where
        F: GestureCallback + Send + Sync + 'static,
    {
        self.callbacks.push(Box::new(f));
        self
    }

    pub fn with_recognizer(mut self, gesture_recognizer: Self) -> Self {
        self.callbacks.extend(gesture_recognizer.callbacks);
        self
    }

    pub fn finger_press(&mut self, finger: Finger) -> Vec<i32> {
        self.active_fingers
            .insert(finger.tracking_id, vec![(EventType::Press, finger)].into());
        self.check_gesture()
    }

    pub fn finger_release(&mut self, finger: Finger) -> Vec<i32> {
        let finger_history = self.active_fingers.entry(finger.tracking_id).or_default();
        finger_history.push((EventType::Release, finger));
        let res = self.check_gesture();
        self.active_fingers.remove(&finger.tracking_id);
        res
    }

    pub fn finger_move(&mut self, finger: Finger) -> Vec<i32> {
        let finger_history = self.active_fingers.entry(finger.tracking_id).or_default();
        finger_history.push((EventType::Move, finger));
        self.check_gesture()
    }

    fn check_gesture(&mut self) -> Vec<i32> {
        let finished_gestures = self
            .active_fingers
            .iter()
            .flat_map(|(finger_id, finger_history)| {
                for callback in &mut self.callbacks {
                    if callback(finger_history).is_some() {
                        return Some(*finger_id);
                    }
                }

                None
            })
            .collect::<Vec<_>>();

        for finger_id in &finished_gestures {
            self.active_fingers.remove(&finger_id);
        }

        finished_gestures
    }

    pub fn reverse_callback_priority(mut self) -> Self {
        self.callbacks.reverse();
        self
    }
}

pub fn recognize_starting_zone(
    position: cgmath::Point2<u16>,
    size: cgmath::Vector2<u16>,
    mut next: impl GestureCallback + Send + Sync,
) -> impl GestureCallback + Send + Sync {
    move |finger_history: &FingerHistory| {
        let start = finger_history.first()?.1.pos;
        if start.x >= position.x
            && start.x <= position.x + size.x
            && start.y >= position.y
            && start.y < position.y + size.y
        {
            next(finger_history)
        } else {
            None
        }
    }
}

pub fn recognize_tap(
    hysteresis: f32,
    mut callback: impl FnMut(cgmath::Point2<u16>) + Clone,
) -> impl GestureCallback + Clone {
    move |finger_history: &FingerHistory| {
        if finger_history.len() < 2 {
            return None;
        }

        if let Some((EventType::Press, _)) = finger_history.first() {
            ()
        } else {
            return None;
        }

        let finger = if let Some((EventType::Release, last)) = finger_history.last() {
            last
        } else {
            return None;
        };

        if finger_history.finger_delta()?.magnitude() < hysteresis {
            (callback)(finger.pos);
            Some(())
        } else {
            None
        }
    }
}

pub fn recognize_press(
    mut callback: impl FnMut(cgmath::Point2<u16>) + Clone,
) -> impl GestureCallback + Clone {
    move |finger_history: &FingerHistory| {
        let pos = if finger_history.len() == 1 {
            let (event_type, finger) = finger_history[0];
            if matches!(event_type, EventType::Press) {
                Some(finger.pos)
            } else {
                None
            }
        } else {
            None
        }?;

        callback(pos);
        Some(())
    }
}

pub fn recognize_release(
    mut callback: impl FnMut(cgmath::Point2<u16>) + Clone,
) -> impl GestureCallback + Clone {
    move |finger_history: &FingerHistory| {
        let pos = if let Some((event_type, finger)) = finger_history.last() {
            if matches!(event_type, EventType::Release) {
                Some(finger.pos)
            } else {
                None
            }
        } else {
            None
        }?;

        callback(pos);
        Some(())
    }
}

pub fn recognize_drag(
    mut callback: impl FnMut(cgmath::Vector2<f32>) -> bool + Clone,
) -> impl GestureCallback + Clone {
    move |finger_history: &FingerHistory| {
        let finger_delta = finger_history.finger_delta()?;
        if callback(finger_delta) {
            Some(())
        } else {
            None
        }
    }
}
