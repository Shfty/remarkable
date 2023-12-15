// TODO: [✓] Support for recognizing multiple gestures with one GestureRecognizer
//       [✓] Process tracking, launching
//           * Use drafts to determine executable name, run ps | grep to determine if running
//           * Spawn process if not running, send SIGCONT otherwise
//           * Send SIGSTOP to all registered applications when tray starts
//       [✓] Address zombie tray processes spawning on exit
//       [✓] Account for simple continuing to run after nao is suspended
//           * Causes UI to stay interactive
//           * Can use /proc/<pid>/stat to determine parent IDs
//             * 4th parameter is parent PID
//             * Parent -> Child linkage doesn't seem to be available
//               * Would be at /proc/<pid>/task/<tid>/children
//               * Instead, need to scan all processes and build a tree
//                 * Probably not as bad as it sounds - ps also scans all processes
//               * Need to be able to track launched processes without storing PID
//                 * PID is useless once process has exited
//                   * i.e. KOReader -> Bash -> Lua becomes untraceable
//                 * Can sandbox using SID or PID namespaces
//                   * https://unix.stackexchange.com/questions/124127/kill-all-descendant-processes
//       [✓] Reduce code duplication following first-working prototype
//       [✓] Account for inputs from KOReader falling through to xochitl
//           * Since events are no longer exclusive to tray, they go to all apps
//             * If apps are sleeping, events are queued and processed on continue
//           * Need to figure out how to properly filter events for inactive apps
//           * Idioms would suggest reading events via exclusive evdev, then emitting via uinput
//             * However, it's unclear if this provides facility for sending to a specific program
//           * Tested sending inputs back to the evdev device by ungrab / send / grab, works
//             * Allows filtering per-event
//             * But doesn't account for events going to multiple programs
//           * Flooding appears to be the only option for clearing the evdev touch queue
//             * Used by both remux and oxide
//           [ ] Fix regression following multi-threading changes
//       [✓] Cache resized icons to disk for faster startup
//           * Watch draft folder in a separate thread
//           * On add / remove / modify, update icons, send update to main thread
//           * Start main thread with no icons, render placeholders until update
//       [✓] Detect system-launched xochitl PID and use for stop / cont
//           * Want to avoid remux-style forced kill / restart
//       [ ] Setup systemd service, package for installation
//       [>] Formalize widgets and layout
//           * Immediate mode
//           * Use functional composition
//           * Draw function + gesture recognizer as a widget
//             * Will need to build both as part of the same process
//           * Set widget on MainLoop, direct input to it
//           * Allow widgets to request draws, etc
//           [✓] Functional combinators for rendering
//           [✓] Take GestureRecognizer as part of render context, build and include in output
//               * Will need the render thread to send recognizers to the main thread
//               * Alternately, add a layer of indirection,
//                 evaluate renderer and recognizer on main thread, dispatch from there
//           [ ] Layout prepass for operations that need to know size before drawing
//       [✓] Use .pid extension for PID files
//       [ ] Partial rendering for loaded icons, close burrons
//           * When an icon placeholder is visible and its file is loaded, redraw its rect
//           * When a close button disappears, redraw its rect instead of the whole panel
//       [✓] Clear input buffers on start to prevent undesired tray relaunches
//           * May have to do this inside wave
//           * Debug to make sure
//           * Probably wise to unify wave / tray input handling via shared crate to allow for this
//       [✓] Application killing functionality
//       [✓] Smarter 'is running' detection for close buttons
//           * Need to account for KOReader and nao spawning bash processes
//       [ ] Clear stopped draft if it's killed via the UI
//           * Will prevent relaunching on close when another app isn't launched first
//       [ ] Smarter icon scaling
//           * Use nearest neighbour + integer upsampling for icons smaller than ICON_SIZE
//             * TilEm icon
//           * Use lanczos3 downsampling for icons larger than ICON_SIZE
//           * Mipmap approach - generate progressively smaller copies and sample the closest
//             * Need to test and see how much slower sampling is
//               * Will require plotting individual pixels in a tight loop
//               * May be able to write to framebuffer from multiple threads
//                 * Tile / scanline based rendering possible?
//                 * Check framebuffer internals
//                   * Implementation may render it nonviable via locks etc
//               * Alternately, may be able to work around by drawing into intermediate
//                 rgb565le buffers and using partial restores to blit directly to framebuffer
//               * Can two async refreshes run concurrently?
//       [ ] Figure out rgb565le -> rgb8 conversion for screenshot manipulation
//           * Will allow for application preview tiles above launch icons
//       [ ] Wacom support
//           * Distance-based hover handling
//             * Darken highlight as pen approaches screen
//       [ ] Exclusive input handling for wave
//           * Prevent gestures from interfering with running program
//           * Act as event filter, pass through unhandled events
//           * Will need smart early-outs to prevent over-greediness
//           * Refined touch targets
//           * Hand off to tray on launch
//       [ ] Drag visualization
//           * Show touch trail until touch-end
//           * Contextual axis locking - e.g. for hscroll / vscroll areas
//       [ ] Rendering for wave
//           * Should be able to treat it as a quick launcher, similar to WebOS
//           * Icon shortcut for tray, other oft-used programs
//           * Bar or pie design
//           * Wave as icon bar, tray as card UI
//

pub mod channel;
pub mod display;
pub mod panel;

mod draft_program;
mod framebuffer;
mod input;
mod rect;
mod render;
mod ui;

use channel::channel;
use display::DISPLAY_HEIGHT;
use input::InputHandles;
use panel::PANEL_HEIGHT;

use gesture::GestureRecognizer;
use libremarkable::{
    cgmath::Point2,
    framebuffer::refresh::PartialRefreshMode,
    image::{ImageBuffer, Rgb},
    input::{multitouch::MultitouchEvent, InputEvent},
};
use raft::{Draft, Drafts};
use shared::{
    kill_recursive, path_temp_pid, path_temp_screenshot, processes, system_xochitl_process,
    TAP_HYSTERESIS,
};

use std::{sync::Arc, thread::JoinHandle, time::Duration};

use crate::{
    channel::{Receiver, Sender},
    display::DISPLAY_RECT,
    draft_program::{get_draft_icon, DraftPrograms, RunType},
    framebuffer::{Color, DisplayTemp, DitherMode, WaveformMode},
    input::{input_init, InputCommand},
    panel::PANEL_RECT,
    render::{render_thread, RenderEvent},
    ui::{
        circle_fill, clear, dump_region, horizontal, image, line, margin, margin_bottom,
        margin_horizontal, margin_left, margin_top, offset_absolute, offset_relative, overlay,
        recognize_gesture, rect_border, rect_stroke, restore_region, set_rect, text_aligned, unit,
        vertical_fixed, Draw, DrawContext, DrawFn, OverlayTrait, ThenTrait,
    },
};

pub const ICON_SIZE: i32 = (DISPLAY_HEIGHT as i32 / 4) / 3;
pub const ICON_SPACING: i32 = ICON_SIZE / 4;
pub const FONT_SIZE: f32 = 42.0;

pub const ROWS: usize = 2;
pub const COLUMNS: usize = 7;
pub const ROW_WIDTH: i32 =
    (ICON_SIZE as i32 * COLUMNS as i32) + (ICON_SPACING as i32 * (COLUMNS as i32 - 1));
pub const ROW_HEIGHT: i32 = ICON_SIZE as i32 + FONT_SIZE as i32 * 2;
pub const ROW_MARGIN: i32 = (DISPLAY_RECT.width as i32 - ROW_WIDTH) / 2;

pub const KILL_SLEEP_DURATION: Duration = std::time::Duration::from_millis(100);

pub enum MainEvent {
    LoadIcon(String, ImageBuffer<Rgb<u8>, Vec<u8>>),
    SetGestureRecognizer(Option<GestureRecognizer>),
    SetDraw(Option<Arc<Box<dyn Draw + Send + Sync>>>),
    Redraw,
    Input(InputEvent),
    Run(Draft),
    StopInput,
    StopRenderer,
    Exit,
}

impl MainEvent {
    pub fn set_draw<D: Draw + Send + Sync + 'static>(draw: Option<D>) -> Self {
        if let Some(draw) = draw {
            MainEvent::SetDraw(Some(Arc::new(Box::new(draw))))
        } else {
            MainEvent::SetDraw(None)
        }
    }
}

fn main() {
    println!("tray startup");

    println!("Loading drafts...");
    let drafts = Arc::new(DraftPrograms::new(
        Drafts::new().expect("Failed to parse draft files"),
    ));

    // Cache the system xochitl PID to disk if it exists
    if let Some(xochitl_proc) = system_xochitl_process() {
        println!("System xochitl process: {xochitl_proc:#?}");
        std::fs::write(
            path_temp_pid("xochitl"),
            xochitl_proc.stat.process_id.to_string(),
        )
        .unwrap();
    }

    // Stop running draft processes from this session, pick one to resume on close
    let stopped_drafts = drafts.stop_draft_programs();
    let stopped_draft = stopped_drafts.get(0).cloned();

    // Create an MPSC channel to receive input events
    println!("Initializing MPSC channels...");
    let (event_tx, event_rx) = channel::<MainEvent>();
    let (render_tx, render_rx) = channel::<RenderEvent>();

    // Start event channels
    println!("Starting event channels...");
    let input_handles = input_init(event_tx.clone());

    input_handles.broadcast(InputCommand::Grab).unwrap();

    // Start render thread
    println!("Starting renderer...");
    let render_handle = std::thread::spawn(render_thread(event_tx.clone(), render_rx));

    render_tx
        .send(RenderEvent::execute(
            set_rect(PANEL_RECT).then(dump_region(move |data| {
                let path = path_temp_screenshot("panel");
                println!("Saving panel screenshot...");
                std::fs::write(path, data).unwrap();
            })),
            false,
        ))
        .unwrap();

    if let Some(draft) = stopped_drafts.get(0) {
        println!("Dumping full screenshot...");

        let draft = draft.clone();
        render_tx
            .send(RenderEvent::execute(
                set_rect(DISPLAY_RECT).then(dump_region(move |data| {
                    let file_name = draft.file_name().unwrap().to_str().unwrap();
                    let path = path_temp_screenshot(file_name);

                    println!("Saving full screenshot...");
                    std::fs::write(path, data).unwrap();
                })),
                false,
            ))
            .unwrap()
    }

    // Start icon loading thread
    {
        let event_tx = event_tx.clone();
        let drafts = drafts.clone();
        std::thread::spawn(move || {
            let mut loaded = false;
            for (id, draft) in drafts.drafts() {
                if let Ok(icon) = get_draft_icon(draft) {
                    event_tx
                        .send(MainEvent::LoadIcon(id.clone(), icon))
                        .unwrap();
                    loaded = true;
                }
            }

            if loaded {
                event_tx.send(MainEvent::Redraw).unwrap();
            }
        });
    }

    println!("Initializing gesture recognizer...");

    event_tx
        .send(MainEvent::set_draw(Some(tray(
            event_tx.clone(),
            drafts.clone(),
            stopped_draft.clone(),
        ))))
        .unwrap();

    MainLoop {
        event_rx,

        input_handles,

        render_handle: Some(render_handle),
        render_tx,

        drafts,
        stopped_drafts,

        gesture_recognizer: None,
        draw: None,
    }
    .run();
}

struct MainLoop {
    event_rx: Receiver<MainEvent>,

    input_handles: InputHandles,

    render_tx: Sender<RenderEvent>,
    render_handle: Option<JoinHandle<()>>,

    drafts: Arc<DraftPrograms>,
    stopped_drafts: Vec<Draft>,

    gesture_recognizer: Option<GestureRecognizer>,
    draw: Option<Arc<Box<dyn Draw + Send + Sync>>>,
}

impl MainLoop {
    pub fn run(mut self) {
        // Enter event loop
        println!("Entering event loop...");
        while let Ok(event) = self.event_rx.recv() {
            match event {
                MainEvent::LoadIcon(key, icon) => {
                    self.drafts.set_icon(key, icon);
                }
                MainEvent::SetGestureRecognizer(gesture_recognizer) => {
                    // Reverse priority of callbacks to ensure frontmost elements check first
                    self.gesture_recognizer =
                        gesture_recognizer.map(GestureRecognizer::reverse_callback_priority);
                }
                MainEvent::SetDraw(draw) => {
                    self.draw = draw;
                    if let Some(draw) = &self.draw {
                        self.render_tx
                            .send(RenderEvent::execute_boxed(draw, true))
                            .unwrap();
                    }
                }
                MainEvent::Redraw => {
                    if let Some(draw) = &self.draw {
                        self.render_tx
                            .send(RenderEvent::execute_boxed(draw, true))
                            .unwrap();
                    }
                }
                MainEvent::Input(input) => match input {
                    InputEvent::MultitouchEvent { event } => {
                        if let Some(gesture_recognizer) = &mut self.gesture_recognizer {
                            match event {
                                MultitouchEvent::Press { finger } => {
                                    gesture_recognizer.finger_press(finger);
                                }
                                MultitouchEvent::Release { finger } => {
                                    gesture_recognizer.finger_release(finger);
                                }
                                MultitouchEvent::Move { finger } => {
                                    gesture_recognizer.finger_move(finger);
                                }
                                _ => (),
                            }
                        }
                    }
                    _ => (),
                },
                MainEvent::Run(draft) => {
                    // Restart stopped draft program if it's still running
                    match self.drafts.run_draft_program(&draft) {
                        RunType::Continue => {
                            if let Some(stopped_draft) = self.stopped_drafts.get(0) {
                                if stopped_draft.call == draft.call {
                                    println!(
                                        "No application switch, restoring partial framebuffer..."
                                    );
                                    let path = path_temp_screenshot("panel");
                                    if let Ok(panel_screenshot) = std::fs::read(path) {
                                        self.render_tx
                                            .send(RenderEvent::execute(
                                                set_rect(PANEL_RECT)
                                                    .then(restore_region(panel_screenshot))
                                                    .then(partial_refresh()),
                                                false,
                                            ))
                                            .unwrap();
                                    } else {
                                        println!("Warning: No full screenshot for continued draft, clearing framebuffer...");
                                        self.render_tx
                                            .send(RenderEvent::execute(
                                                clear().then(full_refresh()),
                                                false,
                                            ))
                                            .unwrap();
                                    }

                                    continue;
                                }
                            }

                            println!("Application switched, restoring full framebuffer...");
                            let path = path_temp_screenshot(draft.file_name().unwrap());
                            if let Ok(full_screenshot) = std::fs::read(path) {
                                self.render_tx
                                    .send(RenderEvent::execute(
                                        set_rect(DISPLAY_RECT)
                                            .then(restore_region(full_screenshot))
                                            .then(full_refresh()),
                                        false,
                                    ))
                                    .unwrap();
                            } else {
                                println!("Warning: No full screenshot for continued draft, clearing framebuffer...");
                                self.render_tx
                                    .send(RenderEvent::execute(clear().then(full_refresh()), false))
                                    .unwrap();
                            }
                        }
                        _ => (),
                    }
                }
                MainEvent::StopInput => {
                    println!("Stopping input");

                    println!("Ungrabbing input devices");
                    self.input_handles.broadcast(InputCommand::Ungrab).unwrap();

                    println!("Clearing event queues");
                    self.input_handles
                        .broadcast(InputCommand::ClearBuffer)
                        .unwrap();

                    println!("Stopping input threads");
                    self.input_handles.broadcast(InputCommand::Stop).unwrap();

                    self.input_handles.join().unwrap();

                    println!("Input stopped");
                }
                MainEvent::StopRenderer => {
                    println!("Stopping renderer");
                    self.render_tx.send(RenderEvent::exit()).unwrap();
                    self.render_handle.take().unwrap().join().unwrap();
                    println!("Renderer stopped");
                }
                MainEvent::Exit => {
                    println!("tray exiting");
                    break;
                }
            }
        }
    }
}

pub fn partial_refresh() -> impl DrawFn {
    crate::ui::partial_refresh(
        PartialRefreshMode::Async,
        WaveformMode::WAVEFORM_MODE_GC16_FAST,
        DisplayTemp::TEMP_USE_REMARKABLE_DRAW,
        DitherMode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
        0,
        false,
    )
}

pub fn full_refresh() -> impl DrawFn {
    crate::ui::full_refresh(
        WaveformMode::WAVEFORM_MODE_GC16_FAST,
        DisplayTemp::TEMP_USE_REMARKABLE_DRAW,
        DitherMode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
        0,
        false,
    )
}

pub fn tray(
    event_tx: Sender<MainEvent>,
    drafts: Arc<DraftPrograms>,
    stopped_draft: Option<Draft>,
) -> impl DrawFn + Clone {
    move |ctx: DrawContext| {
        unit()
            .overlay(
                unit()
                    .then(margin_bottom(PANEL_HEIGHT))
                    .then(recognize_gesture(gesture::recognize_press({
                        let event_tx = event_tx.clone();
                        let stopped_draft = stopped_draft.clone();
                        move |_| {
                            println!("Tapped, exiting");
                            event_tx.send(MainEvent::StopInput).unwrap();
                            if let Some(draft) = &stopped_draft {
                                event_tx.send(MainEvent::Run(draft.clone())).unwrap();
                            }
                            event_tx.send(MainEvent::StopRenderer).unwrap();
                            event_tx.send(MainEvent::Exit).unwrap();
                        }
                    }))),
            )
            .overlay(
                unit()
                    .then(margin_top(DISPLAY_HEIGHT as i32 - PANEL_HEIGHT))
                    .then(drafts_panel(
                        event_tx.clone(),
                        drafts.clone(),
                        stopped_draft.clone(),
                    )),
            )
            .draw(ctx)
    }
}

/// Draw an icon panel for the provided set of draft programs
pub fn drafts_panel<'a>(
    event_tx: Sender<MainEvent>,
    drafts: Arc<DraftPrograms>,
    stopped_draft: Option<Draft>,
) -> impl Draw + 'a {
    unit()
        .then(recognize_gesture({
            let event_tx = event_tx.clone();
            gesture::recognize_drag(move |delta| {
                if delta.y < -TAP_HYSTERESIS {
                    println!("Swiped, exiting");
                    event_tx.send(MainEvent::StopInput).unwrap();
                    if let Some(draft) = &stopped_draft {
                        event_tx.send(MainEvent::Run(draft.clone())).unwrap();
                    }
                    event_tx.send(MainEvent::StopRenderer).unwrap();
                    event_tx.send(MainEvent::Exit).unwrap();

                    true
                } else {
                    false
                }
            })
        }))
        .then(rect_border(2, Color::WHITE, Color::BLACK))
        .then(margin_horizontal(ROW_MARGIN))
        .then(margin_top(ROW_MARGIN))
        .then(draft_icons(event_tx, drafts))
        .then(set_rect(PANEL_RECT))
        .then(partial_refresh())
}

/// Draw a horizontal set of icons for the provided draft programs
pub fn draft_icons(event_tx: Sender<MainEvent>, drafts: Arc<DraftPrograms>) -> impl DrawFn {
    move |mut ctx: DrawContext| {
        let draft_icons = drafts.draft_icons();
        let draft_icons = drafts
            .drafts()
            .keys()
            .map(|key| (drafts.drafts().get(key).unwrap(), draft_icons.get(key)))
            .map(|(draft, icon)| draft_program(event_tx.clone(), drafts.clone(), draft, icon))
            .collect::<Vec<_>>();

        for (i, row) in draft_icons.chunks(COLUMNS).enumerate() {
            ctx = overlay(
                offset_relative(Point2::new(0, ROW_HEIGHT * i as i32))
                    .then(horizontal(ICON_SPACING as i32, row)),
            )(ctx);
        }

        ctx
    }
}

pub fn draft_icon<'a>(icon: Option<&'a ImageBuffer<Rgb<u8>, Vec<u8>>>) -> impl DrawFn + 'a {
    move |ctx: DrawContext| {
        if let Some(icon) = &icon {
            offset_relative(Point2::new(
                (ICON_SIZE as i32 - icon.width() as i32) / 2,
                (ICON_SIZE as i32 - icon.height() as i32) / 2,
            ))
            .then(image(icon))
            .draw(ctx)
        } else {
            spinner(16, 4, Color::BLACK).draw(ctx)
        }
    }
}

pub fn close_button(
    event_tx: Sender<MainEvent>,
    draft_programs: Arc<DraftPrograms>,
    draft: Draft,
) -> impl DrawFn {
    move |ctx: DrawContext| {
        if draft_programs
            .draft_procs()
            .unwrap()
            .into_iter()
            .any(|(candidate, _)| candidate.file_name() == draft.file_name())
        {
            unit()
                .then(margin_left(ICON_SIZE - 32))
                .then(margin_bottom(ICON_SIZE - 32))
                .then(recognize_gesture({
                    let draft_programs = draft_programs.clone();
                    let draft = draft.clone();
                    let event_tx = event_tx.clone();
                    gesture::recognize_tap(TAP_HYSTERESIS, move |_| {
                        if let Some((_, proc)) = draft_programs
                            .draft_procs()
                            .unwrap()
                            .into_iter()
                            .find(|(candidate, _)| candidate.file_name() == draft.file_name())
                        {
                            kill_recursive(&proc);
                            std::thread::sleep(KILL_SLEEP_DURATION);

                            event_tx.send(MainEvent::Redraw).unwrap();
                        }
                    })
                }))
                .then(rect_border(2, Color::WHITE, Color::BLACK))
                .then(offset_absolute(Point2::new(0.5, 0.5)))
                .overlay(line(
                    Point2::new(-10, -10),
                    Point2::new(10, 10),
                    3,
                    Color::BLACK,
                ))
                .overlay(line(
                    Point2::new(10, -10),
                    Point2::new(-10, 10),
                    3,
                    Color::BLACK,
                ))
                .draw(ctx)
        } else {
            ctx
        }
    }
}

/// Draw a titled icon
pub fn draft_program<'a>(
    event_tx: Sender<MainEvent>,
    draft_programs: Arc<DraftPrograms>,
    draft: &'a Draft,
    icon: Option<&'a ImageBuffer<Rgb<u8>, Vec<u8>>>,
) -> impl DrawFn + 'a {
    move |mut ctx: DrawContext| {
        let event_tx = event_tx.clone();

        // Collect string widgets
        let word_strings = draft
            .name
            .split_ascii_whitespace()
            //.map(|word| text_aligned(word, FONT_SIZE, Point2::new(0.5, 0.0), Color::BLACK))
            .map(|word| text_aligned(word, FONT_SIZE, Point2::new(0.5, 0.0), Color::BLACK))
            .collect::<Vec<_>>();

        // Draw icon
        ctx = crate::ui::set_width(ICON_SIZE as u32)
            .overlay(
                crate::ui::set_height(ICON_SIZE as u32)
                    .then(crate::ui::recognize_gesture(gesture::recognize_tap(
                        TAP_HYSTERESIS,
                        {
                            let event_tx = event_tx.clone();
                            let draft = draft.clone();
                            move |_| {
                                println!("Sending run / exit events");
                                event_tx.send(MainEvent::StopInput).unwrap();
                                event_tx.send(MainEvent::Run(draft.clone())).unwrap();
                                event_tx.send(MainEvent::StopRenderer).unwrap();
                                event_tx.send(MainEvent::Exit).unwrap();
                            }
                        },
                    )))
                    .then(margin(-1))
                    .then(rect_stroke(2, Color::BLACK))
                    .overlay(draft_icon(icon))
                    .overlay(close_button(
                        event_tx,
                        draft_programs.clone(),
                        draft.clone(),
                    )),
            )
            .overlay(
                margin_top(ICON_SIZE as i32 + ICON_SPACING as i32)
                    .then(offset_relative(Point2::new(ICON_SIZE as i32 / 2, 0)))
                    .then(vertical_fixed(FONT_SIZE as i32 - 8, &word_strings)),
            )
            .draw(ctx);

        ctx
    }
}

// Draw a progress indicator in the center of the provided rect
pub fn spinner(ofs: i32, rad: u32, color: Color) -> impl Draw {
    crate::ui::offset_absolute(Point2::new(0.5, 0.5))
        .overlay(offset_relative(Point2::new(-ofs, 0)).then(circle_fill(rad, color)))
        .overlay(circle_fill(rad, color))
        .overlay(offset_relative(Point2::new(ofs, 0)).then(circle_fill(rad, color)))
}
