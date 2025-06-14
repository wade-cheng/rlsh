use std::cmp;
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::fs::File;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    BG,
    FG,
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BG => write!(f, "Background"),
            Self::FG => write!(f, "Foreground"),
        }
    }
}

pub struct Job {
    pid: u32,
    state: State,
    cmdline: String,
}

struct JobData {
    jobs: HashMap<usize, Job>,
    fg_job: Option<usize>,
    max_jid: Option<usize>,
}

// List to manage jobs
#[derive(Clone)]
pub struct JobList(Arc<Mutex<JobData>>);

impl JobList {
    // Creates a new empty job list
    pub fn new() -> Self {
        JobList(Arc::new(Mutex::new(JobData {
            jobs: HashMap::new(),
            fg_job: None,
            max_jid: None,
        })))
    }

    // Adds a new value to the job list with the following pid, state, and cmdline and returns its jid
    pub fn add(&self, pid: u32, state: State, cmdline: String) -> Result<usize, &'static str> {
        let JobList(arc) = self;
        let mut job_list = arc.lock().unwrap();

        // Calculate jid of new job
        let jid = match job_list.max_jid {
            None => 0,
            Some(id) => id + 1,
        };

        // Update foreground
        if let State::FG = state {
            if let Some(_) = job_list.fg_job {
                return Err("Can't add a foreground job if a foreground job already exists");
            } else {
                job_list.fg_job = Some(jid)
            }
        }

        // update the max jid in the list
        job_list.max_jid = Some(jid);

        // Create job
        let job = Job {
            pid,
            state,
            cmdline,
        };

        // throw error if insert triggers an override
        if let Some(_) = job_list.jobs.insert(jid, job) {
            return Err("Inserted job with duplicate jid");
        }

        Ok(jid)
    }

    // Deletes a job from the job list
    pub fn delete(&self, jid: usize) -> bool {
        let JobList(arc) = self;
        let mut job_list = arc.lock().unwrap();

        //remove from job list
        let remove_status = job_list.jobs.remove(&jid);

        // update max jid
        if let Some(id) = job_list.max_jid {
            if jid == id {
                job_list.max_jid = match job_list.jobs.keys().reduce(cmp::max) {
                    Some(x) => Some(*x),
                    None => None,
                };
            }
        }

        // update foreground job
        if let Some(id) = job_list.fg_job {
            if id == jid {
                job_list.fg_job = None
            }
        }

        // return if successful remove
        remove_status.is_some()
    }

    // Returns the state of any one job
    pub fn get_state(&self, jid: usize) -> Option<State> {
        let JobList(arc) = self;
        let job_list = arc.lock().unwrap();

        let job = job_list.jobs.get(&jid)?;
        Some(job.state)
    }

    // gets the pid associated by a pid
    pub fn get_pid(&self, jid: usize) -> Option<u32> {
        let JobList(arc) = self;
        let job_list = arc.lock().unwrap();

        let job = job_list.jobs.get(&jid)?;
        Some(job.pid)
    }

    // Gets the cmdline of a job
    pub fn get_cmdline(&self, jid: usize) -> Option<String> {
        let JobList(arc) = self;
        let job_list = arc.lock().unwrap();

        let job = job_list.jobs.get(&jid)?;
        Some(job.cmdline.clone())
    }

    // prints out the job list to the file specified by outfile or stdout if it is None
    // If the file exists it is truncated before writing
    // If it does not exist it is created
    pub fn list_jobs(&self, outfile: Option<String>) -> io::Result<()> {
        match outfile {
            None => {
                let stdout = io::stdout().lock();
                self.print_jobs(stdout)
            }
            Some(path) => {
                let file = File::create(path)?;
                self.print_jobs(file)
            }
        }
    }

    // Prints all jobs in the job list in a random order to the specified writer
    fn print_jobs<W: Write>(&self, mut writer: W) -> io::Result<()> {
        let JobList(arc) = self;
        let job_list = arc.lock().unwrap();

        for (jid, job) in job_list.jobs.iter() {
            write!(
                writer,
                "[{jid}] ({}) {} {}",
                job.pid, job.state, job.cmdline
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adding_jobs() {
        let list = JobList::new();
        let result = list.add(1, State::FG, "one".to_string());
        assert_eq!(Ok(0), result);
        let result = list.add(2, State::BG, "two".to_string());
        assert_eq!(Ok(1), result);
        let result = list.add(3, State::FG, "three".to_string());
        assert_eq!(
            Err("Can't add a foreground job if a foreground job already exists"),
            result
        );
        let result = list.add(3, State::BG, "three".to_string());
        assert_eq!(Ok(2), result);
    }

    #[test]
    fn get_jobs() {
        let list = JobList::new();
        list.add(1, State::FG, "one".to_string()).unwrap();
        list.add(2, State::BG, "two".to_string()).unwrap();
        list.add(3, State::BG, "three".to_string()).unwrap();
        assert_eq!(Some(1), list.get_pid(0));
        assert_eq!(Some(State::FG), list.get_state(0));
        assert_eq!(Some("one".to_string()), list.get_cmdline(0));
        assert_eq!(Some(2), list.get_pid(1));
        assert_eq!(Some(State::BG), list.get_state(1));
        assert_eq!(Some("two".to_string()), list.get_cmdline(1));
        assert_eq!(Some(3), list.get_pid(2));
        assert_eq!(Some(State::BG), list.get_state(2));
        assert_eq!(Some("three".to_string()), list.get_cmdline(2));
        assert_eq!(None, list.get_pid(3));
    }

    #[test]
    fn delete_jobs() {
        let list = JobList::new();
        list.add(1, State::FG, "one".to_string()).unwrap();
        list.add(2, State::BG, "two".to_string()).unwrap();
        list.add(3, State::BG, "three".to_string()).unwrap();
        assert_eq!(false, list.delete(3));
        assert_eq!(true, list.delete(1));
        assert_eq!(None, list.get_pid(1));
        assert_eq!(Ok(3), list.add(4, State::BG, "four".to_string()));
        assert_eq!(true, list.delete(3));
        assert_eq!(None, list.get_pid(3));
        assert_eq!(true, list.delete(2));
        assert_eq!(None, list.get_pid(2));
        assert_eq!(Ok(1), list.add(5, State::BG, "four".to_string()));
    }
}
