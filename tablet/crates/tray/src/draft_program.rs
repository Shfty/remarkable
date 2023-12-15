use std::{collections::BTreeMap, error::Error, path::PathBuf, process::Command};

use libremarkable::{
    cgmath::{Vector3, VectorSpace},
    image::{ColorType, ImageBuffer, Rgb},
};
use proc::{Proc, State};
use raft::{Draft, Drafts};
use shared::{
    cont_recursive, path_temp_icon, path_temp_pid, path_temp_pids, processes, stop_recursive,
};
use std::sync::{Mutex, MutexGuard};

use crate::ICON_SIZE;

#[derive(Debug, Copy, Clone)]
pub enum RunType {
    Continue,
    Launch,
}

pub type DraftId = String;

#[derive(Debug, Default)]
pub struct DraftPrograms {
    drafts: BTreeMap<DraftId, Draft>,
    icons: Mutex<BTreeMap<DraftId, ImageBuffer<Rgb<u8>, Vec<u8>>>>,
}

impl DraftPrograms {
    pub fn new(drafts: Drafts) -> Self {
        let drafts = drafts
            .take()
            .into_iter()
            .map(|draft| (draft.name.clone(), draft))
            .collect::<BTreeMap<_, _>>();

        let icons = drafts
            .iter()
            .filter_map(|(key, draft)| {
                if let Some(icon) = &draft.icon {
                    let mut cache_path = path_temp_icon(
                        PathBuf::from(icon)
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string(),
                    );
                    cache_path.set_extension("png");

                    if cache_path.exists() {
                        println!("Loading cached icon {cache_path:?}");
                        let image = libremarkable::image::open(cache_path).unwrap().to_rgb8();

                        Some((key.clone(), image))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<BTreeMap<_, _>>();
        let icons = Mutex::new(icons);

        DraftPrograms { drafts, icons }
    }

    pub fn drafts(&self) -> &BTreeMap<String, Draft> {
        &self.drafts
    }

    pub fn draft_icons(&self) -> MutexGuard<BTreeMap<String, ImageBuffer<Rgb<u8>, Vec<u8>>>> {
        self.icons.lock().unwrap()
    }

    pub fn set_icon(&self, key: String, icon: ImageBuffer<Rgb<u8>, Vec<u8>>) {
        self.draft_icons().insert(key, icon);
    }

    pub fn draft_procs<'a>(&'a self) -> Result<Vec<(&'a Draft, Proc)>, std::io::Error> {
        Ok(std::fs::read_dir(path_temp_pids())?
            .flat_map(|result| {
                let result = result.unwrap();

                let file_type = result.file_type().unwrap();
                if !file_type.is_file() {
                    return None;
                }

                let mut file_name = PathBuf::from(result.file_name());
                file_name.set_extension("");

                let (_, draft) = self
                    .drafts()
                    .iter()
                    .find(|(_, draft)| draft.name == file_name.to_str().unwrap())
                    .unwrap();

                let pid = std::fs::read_to_string(result.path())
                    .unwrap()
                    .parse::<usize>()
                    .unwrap();

                if let Some(proc) = processes().find(|proc| proc.stat.process_id == pid) {
                    Some((draft, proc))
                } else {
                    println!(
                        "Warning: PID {pid:} present in temp dir but not running, deleting record"
                    );
                    std::fs::remove_file(result.path()).unwrap();
                    None
                }
            })
            .collect::<Vec<_>>())
    }

    pub fn stop_draft_programs(&self) -> Vec<Draft> {
        let running_draft_procs = self
            .draft_procs()
            .unwrap()
            .into_iter()
            .filter(|(_, proc)| match proc.stat.state {
                State::Running | State::Sleeping | State::Delay => true,
                _ => false,
            })
            .collect::<Vec<_>>();

        if running_draft_procs.len() > 1 {
            println!("Warning: More than one draft application is running");
        }

        for (_, process) in &running_draft_procs {
            stop_recursive(process);
        }

        running_draft_procs
            .into_iter()
            .map(|(draft, _)| draft.clone())
            .collect::<Vec<_>>()
    }

    pub fn run_draft_program(&self, draft: &Draft) -> RunType {
        if let Some((_, proc)) = self
            .draft_procs()
            .unwrap()
            .into_iter()
            .filter(|(_, proc)| match proc.stat.state {
                State::Traced => true,
                _ => false,
            })
            .find(|(candidate, _)| candidate.name == draft.name)
        {
            // If the process still exists and is sleeping, continue it
            cont_recursive(&proc);
            RunType::Continue
        } else {
            // If the process isn't running, launch it and add its PID to the temp directory
            println!("Launching {:#?}", draft);
            let pid = Command::new(&draft.call).spawn().unwrap().id() as usize;
            std::fs::write(path_temp_pid(&draft.name), pid.to_string()).unwrap();
            RunType::Launch
        }
    }
}

pub fn get_draft_icon(
    draft: &Draft,
) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>, Box<dyn Error + Send + Sync + 'static>> {
    let mut cache_path = path_temp_icon(draft.file_name().unwrap());
    cache_path.set_extension("png");

    let image = if cache_path.exists() {
        return Err("Cached icon, already loaded")?;
    } else {
        let icon = draft.icon.as_ref().ok_or("Draft has no icon")?;
        let image = libremarkable::image::open(icon)?;
        let image = image.resize(
            ICON_SIZE as u32,
            ICON_SIZE as u32,
            libremarkable::image::imageops::FilterType::Lanczos3,
        );
        let image = image.into_rgba8();
        let image = ImageBuffer::<Rgb<u8>, _>::from_raw(
            image.width(),
            image.height(),
            image
                .pixels()
                .flat_map(|pixel| {
                    let color = Vector3::new(
                        pixel.0[0] as f32 / u8::MAX as f32,
                        pixel.0[1] as f32 / u8::MAX as f32,
                        pixel.0[2] as f32 / u8::MAX as f32,
                    );
                    let alpha = pixel.0[3] as f32 / u8::MAX as f32;
                    let color = color.lerp(Vector3::new(1.0, 1.0, 1.0), 1.0 - alpha);

                    [
                        (color.x * u8::MAX as f32) as u8,
                        (color.y * u8::MAX as f32) as u8,
                        (color.z * u8::MAX as f32) as u8,
                    ]
                })
                .collect::<Vec<_>>(),
        )
        .unwrap();

        println!("Saving icon to {cache_path:?}");
        libremarkable::image::save_buffer(
            cache_path,
            &image,
            image.width(),
            image.height(),
            ColorType::Rgb8,
        )
        .unwrap();

        image
    };

    Ok(image)
}
