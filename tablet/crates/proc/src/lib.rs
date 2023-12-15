use std::{collections::BTreeMap, error::Error, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Running,
    Sleeping,
    Delay,
    Zombie,
    Traced,
    Unknown(String),
}

impl FromStr for State {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "R" => Ok(State::Running),
            "S" => Ok(State::Sleeping),
            "D" => Ok(State::Delay),
            "Z" => Ok(State::Zombie),
            "T" => Ok(State::Traced),
            s => Ok(State::Unknown(s.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Stat {
    pub process_id: usize,
    pub filename: String,
    pub state: State,
    pub parent_process_id: usize,
    pub process_group: usize,
    pub session_id: usize,
    pub tty_number: usize,
    pub tty_process_group: isize,
    pub flags: usize,
    pub minor_faults: usize,
    pub minor_faults_children: usize,
    pub major_faults: usize,
    pub major_faults_children: usize,
    pub user_time: usize,
    pub kernel_time: usize,
    pub user_time_children: usize,
    pub kernel_time_children: usize,
    pub priority: isize,
    pub nice: isize,
    pub num_threads: usize,
    pub it_real_value: (),
    pub start_time: usize,
    pub virtual_memory_size: usize,
    pub resident_set_memory_size: usize,
    pub resident_set_memory_limit: usize,
    pub start_code: usize,
    pub end_code: usize,
    pub start_stack: usize,
    pub esp: usize,
    pub eip: usize,
    pub pending_signals: usize,
    pub blocked_signals: usize,
    pub ignored_signals: usize,
    pub caught_signals: usize,
    pub placeholder_0: (),
    pub placeholder_1: (),
    pub placeholder_2: (),
    pub exit_signal: usize,
    pub task_cpu: usize,
    pub realtime_priority: usize,
    pub scheduling_policy: usize,
    pub block_io_ticks: usize,
    pub guest_time: usize,
    pub guest_time_children: usize,
    pub start_data: usize,
    pub end_data: usize,
    pub start_brk: usize,
    pub arg_start: usize,
    pub arg_end: usize,
    pub env_start: usize,
    pub env_end: usize,
    pub exit_code: usize,
}

impl PartialEq for Stat {
    fn eq(&self, other: &Self) -> bool {
        self.process_id.eq(&other.process_id)
    }
}

impl PartialOrd for Stat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.process_id.partial_cmp(&other.process_id)
    }
}

impl FromStr for Stat {
    type Err = Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split_whitespace();

        let pid = parts.next().unwrap().parse()?;

        let mut tcomm = parts.next().unwrap().to_owned();
        if tcomm.starts_with("(") && !tcomm.ends_with(")") {
            loop {
                let next = parts.next().unwrap();
                tcomm += " ";
                tcomm += next;
                if next.ends_with(")") || next.ends_with("]") {
                    break;
                }
            }
        }
        tcomm.remove(0);
        tcomm.pop().unwrap();

        Ok(Stat {
            process_id: pid,
            filename: tcomm,
            state: parts.next().unwrap().parse().unwrap(),
            parent_process_id: parts.next().unwrap().parse().unwrap(),
            process_group: parts.next().unwrap().parse().unwrap(),
            session_id: parts.next().unwrap().parse().unwrap(),
            tty_number: parts.next().unwrap().parse().unwrap(),
            tty_process_group: parts.next().unwrap().parse().unwrap(),
            flags: parts.next().unwrap().parse().unwrap(),
            minor_faults: parts.next().unwrap().parse().unwrap(),
            minor_faults_children: parts.next().unwrap().parse().unwrap(),
            major_faults: parts.next().unwrap().parse().unwrap(),
            major_faults_children: parts.next().unwrap().parse().unwrap(),
            user_time: parts.next().unwrap().parse().unwrap(),
            user_time_children: parts.next().unwrap().parse().unwrap(),
            kernel_time: parts.next().unwrap().parse().unwrap(),
            kernel_time_children: parts.next().unwrap().parse().unwrap(),
            priority: parts.next().unwrap().parse().unwrap(),
            nice: parts.next().unwrap().parse().unwrap(),
            num_threads: parts.next().unwrap().parse().unwrap(),
            it_real_value: {
                parts.next().unwrap();
            },
            start_time: parts.next().unwrap().parse().unwrap(),
            virtual_memory_size: parts.next().unwrap().parse().unwrap(),
            resident_set_memory_size: parts.next().unwrap().parse().unwrap(),
            resident_set_memory_limit: parts.next().unwrap().parse().unwrap(),
            start_code: parts.next().unwrap().parse().unwrap(),
            end_code: parts.next().unwrap().parse().unwrap(),
            start_stack: parts.next().unwrap().parse().unwrap(),
            esp: parts.next().unwrap().parse().unwrap(),
            eip: parts.next().unwrap().parse().unwrap(),
            pending_signals: parts.next().unwrap().parse().unwrap(),
            blocked_signals: parts.next().unwrap().parse().unwrap(),
            ignored_signals: parts.next().unwrap().parse().unwrap(),
            caught_signals: parts.next().unwrap().parse().unwrap(),
            placeholder_0: {
                parts.next().unwrap();
            },
            placeholder_1: {
                parts.next().unwrap();
            },
            placeholder_2: {
                parts.next().unwrap();
            },
            exit_signal: parts.next().unwrap().parse().unwrap(),
            task_cpu: parts.next().unwrap().parse().unwrap(),
            realtime_priority: parts.next().unwrap().parse().unwrap(),
            scheduling_policy: parts.next().unwrap().parse().unwrap(),
            block_io_ticks: parts.next().unwrap().parse().unwrap(),
            guest_time: parts.next().unwrap().parse().unwrap(),
            guest_time_children: parts.next().unwrap().parse().unwrap(),
            start_data: parts.next().unwrap().parse().unwrap(),
            end_data: parts.next().unwrap().parse().unwrap(),
            start_brk: parts.next().unwrap().parse().unwrap(),
            arg_start: parts.next().unwrap().parse().unwrap(),
            arg_end: parts.next().unwrap().parse().unwrap(),
            env_start: parts.next().unwrap().parse().unwrap(),
            env_end: parts.next().unwrap().parse().unwrap(),
            exit_code: parts.next().unwrap().parse().unwrap(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Proc {
    pub stat: Stat,
    pub cmdline: String,
}

impl PartialEq for Proc {
    fn eq(&self, other: &Self) -> bool {
        self.stat.eq(&other.stat)
    }
}

impl PartialOrd for Proc {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.stat.partial_cmp(&other.stat)
    }
}

pub type Pid = usize;
pub type ProcFs = BTreeMap<Pid, Proc>;

#[derive(Debug)]
enum ProcFsError {
    NotADirectory,
    NotAPidDirectory,
}

impl std::fmt::Display for ProcFsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcFsError::NotADirectory => f.write_str("Not a directory"),
            ProcFsError::NotAPidDirectory => f.write_str("Not a PID directory"),
        }
    }
}

impl Error for ProcFsError {}

pub fn proc_fs() -> Result<impl Iterator<Item = Result<(Pid, Proc), Box<dyn Error>>>, std::io::Error>
{
    Ok(
        std::fs::read_dir("/proc")?.map::<Result<_, Box<dyn Error>>, _>(|result| {
            let result = result?;

            let file_type = result.file_type()?;
            if !file_type.is_dir() {
                return Err(Box::new(ProcFsError::NotADirectory));
            }

            let file_name = result.file_name();
            let file_name = file_name.to_str().unwrap();
            if !file_name.chars().all(char::is_numeric) {
                return Err(Box::new(ProcFsError::NotAPidDirectory));
            }

            let pid: Pid = file_name.parse()?;
            let mut path = result.path();
            path.push("stat");

            let stat = std::fs::read_to_string(path)?;
            let stat = stat.parse::<Stat>()?;

            let mut path = result.path();
            path.push("cmdline");

            let cmdline = std::fs::read_to_string(path)?
                .replace("\0", " ")
                .trim()
                .to_string();

            Ok((pid, Proc { stat, cmdline }))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let proc_fs = proc_fs().unwrap().collect::<Vec<_>>();
        println!("{proc_fs:#?}");
    }
}
