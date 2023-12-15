use std::path::{Path, PathBuf};

use nix::{sys::signal::kill, unistd::Pid};

use proc::{proc_fs, Proc, State};
use raft::Draft;

pub const TEMP_DIR: &'static str = "/tmp/parchment";
pub const TEMP_DIR_SCREENSHOTS: &'static str = "screenshots";
pub const TEMP_DIR_ICONS: &'static str = "icons";
pub const TEMP_DIR_PIDS: &'static str = "processes";

pub const TAP_HYSTERESIS: f32 = 32.0;
pub const INPUT_BUFFER_SIZE: usize = 512 * 8;

pub fn path_temp_screenshots() -> PathBuf {
    let mut path = PathBuf::from(TEMP_DIR);
    path.push(TEMP_DIR_SCREENSHOTS);
    path
}

pub fn path_temp_screenshot<P: AsRef<Path>>(filename: P) -> PathBuf {
    let mut path = path_temp_screenshots();
    path.push(filename);
    path
}

pub fn path_temp_icons() -> PathBuf {
    let mut path = PathBuf::from(TEMP_DIR);
    path.push(TEMP_DIR_ICONS);
    path
}

pub fn path_temp_icon<P: AsRef<Path>>(filename: P) -> PathBuf {
    let mut path = path_temp_icons();
    path.push(filename);
    path
}

pub fn path_temp_pids() -> PathBuf {
    let mut path = PathBuf::from(TEMP_DIR);
    path.push(TEMP_DIR_PIDS);
    path
}

pub fn path_temp_pid<P: AsRef<Path>>(filename: P) -> PathBuf {
    let mut path = path_temp_pids();
    path.push(filename);
    path.set_extension("pid");
    path
}

pub fn stop_recursive(proc: &Proc) {
    println!("Stopping process {:?}", proc.stat.filename);
    kill(
        Pid::from_raw(proc.stat.process_id as i32),
        nix::sys::signal::Signal::SIGSTOP,
    )
    .unwrap();
    for proc in processes().filter(is_child_process_of(proc.stat.process_id)) {
        stop_recursive(&proc);
    }
}

pub fn cont_recursive(proc: &Proc) {
    for proc in processes().filter(is_child_process_of(proc.stat.process_id)) {
        cont_recursive(&proc);
    }
    println!("Continuing process {:?}", proc.stat.filename);
    kill(
        Pid::from_raw(proc.stat.process_id as i32),
        nix::sys::signal::Signal::SIGCONT,
    )
    .unwrap();
}

pub fn kill_recursive(proc: &Proc) {
    let pid = proc.stat.process_id;
    for proc in processes().filter(is_child_process_of(pid)) {
        kill_recursive(&proc);
    }
    println!("Killing process {:?}", proc.stat.filename);
    kill(
        Pid::from_raw(proc.stat.process_id as i32),
        nix::sys::signal::Signal::SIGKILL,
    )
    .unwrap();
}

pub fn processes() -> impl Iterator<Item = Proc> {
    proc_fs().unwrap().flatten().map(|(_, proc)| proc)
}

pub fn system_xochitl_process() -> Option<Proc> {
    processes().find(|proc| proc.cmdline == "/usr/bin/xochitl --system")
}

pub fn has_session(session_id: usize) -> impl Fn(&Proc) -> bool {
    move |proc| proc.stat.session_id == session_id
}

pub fn is_running(proc: &Proc) -> bool {
    match &proc.stat.state {
        State::Running | State::Sleeping | State::Delay => true,
        _ => false,
    }
}

pub fn is_stopped() -> impl Fn(&Proc) -> bool {
    move |proc| match &proc.stat.state {
        State::Traced => true,
        _ => false,
    }
}

pub fn is_draft<'a, I: IntoIterator<Item = &'a Draft> + Clone>(
    drafts: I,
) -> impl FnMut(Proc) -> Option<(&'a Draft, Proc)> {
    move |proc| {
        if let Some(draft) = drafts.clone().into_iter().find(|draft| {
            draft.file_name().unwrap().to_str().unwrap() == proc.stat.filename.as_str()
        }) {
            Some((draft, proc))
        } else {
            None
        }
    }
}

pub fn not_system_process(proc: &Proc) -> bool {
    proc.stat.filename != "wave" && proc.stat.filename != "tray"
}

pub fn is_child_process_of(pid: usize) -> impl Fn(&Proc) -> bool {
    move |proc| proc.stat.parent_process_id == pid
}

pub fn button_flood_events() -> [libremarkable::evdev::InputEvent; 2] {
    [
        libremarkable::evdev::InputEvent::new_now(
            libremarkable::evdev::EventType::SYNCHRONIZATION,
            1,
            0,
        ),
        libremarkable::evdev::InputEvent::new_now(
            libremarkable::evdev::EventType::SYNCHRONIZATION,
            0,
            1,
        ),
    ]
}

pub fn touch_flood_events() -> [libremarkable::evdev::InputEvent; 4] {
    [
        libremarkable::evdev::InputEvent::new_now(
            libremarkable::evdev::EventType::ABSOLUTE,
            libremarkable::evdev::AbsoluteAxisType::ABS_DISTANCE.0,
            1,
        ),
        libremarkable::evdev::InputEvent::new_now(
            libremarkable::evdev::EventType::SYNCHRONIZATION,
            0,
            1,
        ),
        libremarkable::evdev::InputEvent::new_now(
            libremarkable::evdev::EventType::ABSOLUTE,
            libremarkable::evdev::AbsoluteAxisType::ABS_DISTANCE.0,
            2,
        ),
        libremarkable::evdev::InputEvent::new_now(
            libremarkable::evdev::EventType::SYNCHRONIZATION,
            0,
            1,
        ),
    ]
}
