use libremarkable::{
    epoll,
    evdev::InputEvent as EvInputEvent,
    input::{scan::SCANNED, InputDevice, InputDeviceState, InputEvent},
};
use shared::{button_flood_events, touch_flood_events, INPUT_BUFFER_SIZE};

use std::{any::Any, error::Error, os::unix::prelude::AsRawFd, thread::JoinHandle};

use crate::channel::{channel, SendError, Sender, TryRecvError};

use crate::MainEvent;

const EPOLL_TIMEOUT: i32 = 100;

#[derive(Debug, Copy, Clone)]
pub enum InputCommand {
    Stop,
    Grab,
    Ungrab,
    ClearBuffer,
}

pub struct InputHandles {
    pub gpio_command: Sender<InputCommand>,
    pub multitouch_command: Sender<InputCommand>,
    pub wacom_command: Sender<InputCommand>,

    pub gpio_handle: Option<JoinHandle<()>>,
    pub multitouch_handle: Option<JoinHandle<()>>,
    pub wacom_handle: Option<JoinHandle<()>>,
}

impl InputHandles {
    pub fn broadcast(&self, event: InputCommand) -> Result<(), SendError<InputCommand>> {
        self.gpio_command.send(event)?;
        self.multitouch_command.send(event)?;
        self.wacom_command.send(event)?;
        Ok(())
    }

    pub fn join(&mut self) -> Result<(), Box<dyn Any + Send>> {
        self.gpio_handle.take().unwrap().join()?;
        self.multitouch_handle.take().unwrap().join()?;
        self.wacom_handle.take().unwrap().join()?;
        Ok(())
    }
}

pub fn input_init(event_tx: Sender<MainEvent>) -> InputHandles {
    let (gpio_command, gpio_handle) = input_thread(
        InputDevice::GPIO,
        event_tx.clone(),
        libremarkable::input::gpio::decode,
        button_flood_events(),
    )
    .unwrap();

    let (multitouch_command, multitouch_handle) = input_thread(
        InputDevice::Multitouch,
        event_tx.clone(),
        libremarkable::input::multitouch::decode,
        touch_flood_events(),
    )
    .unwrap();

    let (wacom_command, wacom_handle) = input_thread(
        InputDevice::Wacom,
        event_tx.clone(),
        libremarkable::input::wacom::decode,
        touch_flood_events(),
    )
    .unwrap();

    InputHandles {
        gpio_command,
        multitouch_command,
        wacom_command,
        gpio_handle: Some(gpio_handle),
        multitouch_handle: Some(multitouch_handle),
        wacom_handle: Some(wacom_handle),
    }
}

pub fn input_thread<F, R, I>(
    device_type: InputDevice,
    event_tx: Sender<MainEvent>,
    callback: F,
    flood_events: I,
) -> Result<(Sender<InputCommand>, JoinHandle<()>), Box<dyn Error>>
where
    F: Fn(&EvInputEvent, &libremarkable::input::InputDeviceState) -> R + Send + 'static,
    R: IntoIterator<Item = InputEvent>,
    I: IntoIterator<Item = libremarkable::evdev::InputEvent> + Clone + Send + 'static,
{
    let mut device = SCANNED.get_device(device_type)?;
    let state = InputDeviceState::new(device_type);
    let (command_tx, command_rx) = channel();

    let mut v = [epoll::Event {
        events: (epoll::Events::EPOLLET | epoll::Events::EPOLLIN | epoll::Events::EPOLLPRI).bits(),
        data: 0,
    }];

    let epfd = epoll::create(false).unwrap();

    epoll::ctl(
        epfd,
        epoll::ControlOptions::EPOLL_CTL_ADD,
        device.as_raw_fd(),
        v[0],
    )
    .unwrap();

    let flood_events = flood_events.into_iter().collect::<Vec<_>>();
    let flood_events = std::iter::repeat(flood_events.clone())
        .take(INPUT_BUFFER_SIZE)
        .flatten()
        .collect::<Vec<_>>();

    let join_handle = std::thread::spawn(move || {
        println!("Starting epoll thread");

        'input: loop {
            'command: loop {
                match command_rx.try_recv() {
                    Ok(command) => match command {
                        InputCommand::Stop => break 'input,
                        InputCommand::Grab => {
                            device.grab().unwrap();
                            println!("Grabbed input.");
                        }
                        InputCommand::Ungrab => {
                            device.ungrab().unwrap();
                            println!("Ungrabbed input.");
                        }
                        InputCommand::ClearBuffer => {
                            if flood_events.len() == 0 {
                                println!("No flood events for device, skipping");
                            } else {
                                println!("Clearing buffer...");
                                device.send_events(&flood_events[..]).unwrap();
                            }
                        }
                    },
                    Err(e) => match e {
                        TryRecvError::Empty => break 'command,
                        TryRecvError::Disconnected => break 'input,
                    },
                }
            }

            match epoll::wait(epfd, EPOLL_TIMEOUT, &mut v[..]) {
                Ok(res) => {
                    if res == 0 {
                        continue;
                    }

                    for ev in device.fetch_events().unwrap() {
                        for event in callback(&ev, &state) {
                            if let Err(e) = event_tx.send(MainEvent::Input(event)) {
                                eprintln!("Failed to write InputEvent into the channel: {}", e);
                            }
                        }
                    }
                }
                Err(err) => {
                    println!("epoll_wait failed: {}", err);
                }
            };
        }

        println!("epoll thread finalizing");

        epoll::close(epfd).unwrap();
        println!("Closed descriptor.");

        println!("epoll thread done");
    });

    Ok((command_tx, join_handle))
}
