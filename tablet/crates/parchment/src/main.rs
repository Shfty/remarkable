use shared::{
    cont_recursive, kill_recursive, path_temp_icons, path_temp_pids, path_temp_screenshots,
    processes, system_xochitl_process, TEMP_DIR,
};
use std::process::Command;

fn main() {
    println!("parchment startup");

    // Kill any leftover processes
    if let Ok(dir) = std::fs::read_dir(path_temp_pids()) {
        for result in dir {
            let result = result.unwrap();
            let file_type = result.file_type().unwrap();
            if !file_type.is_file() {
                continue;
            }

            let file_name = result.file_name();
            let pid = std::fs::read_to_string(result.path())
                .unwrap()
                .parse::<usize>()
                .unwrap();

            if let Some(proc) = processes()
                .filter(|proc| Some(proc) != system_xochitl_process().as_ref())
                .find(|proc| proc.stat.process_id == pid)
            {
                println!("Killing leftover {:?} process with PID {}", file_name, pid);
                cont_recursive(&proc);
                kill_recursive(&proc);
            }
        }
    }

    // Clear temporary directory and recreate it
    std::fs::remove_dir_all(TEMP_DIR).ok();
    std::fs::create_dir_all(TEMP_DIR).unwrap();
    std::fs::create_dir_all(path_temp_screenshots()).unwrap();
    std::fs::create_dir_all(path_temp_icons()).unwrap();
    std::fs::create_dir_all(path_temp_pids()).unwrap();

    // Start wave
    Command::new("./wave").spawn().unwrap().wait().unwrap();
}
