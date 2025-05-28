use std::{cell::RefCell, cmp, collections::HashMap};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    BG,
    FG,
    ST,
}

#[derive(Clone, Copy)]
pub struct Job<'a> {
    pid: usize,
    state: State,
    cmdline: &'a str,
}

struct JobData<'a> {
    jobs: HashMap<usize, Job<'a>>,
    fg_job: Option<usize>,
    max_jid: Option<usize>,
}

// List to manage jobs
pub struct JobList<'a>(RefCell<JobData<'a>>);

impl<'a> JobList<'a> {
    // Creates a new empty job list
    pub fn new() -> Self {
        JobList(RefCell::new(JobData {
            jobs: HashMap::new(),
            fg_job: None,
            max_jid: None,
        }))
    }

    // Gets the job with the assiciated jid
    pub fn get(&self, jid: usize) -> Option<Job> {
        let JobList(cell) = self;
        let job_list = cell.borrow();
        match job_list.jobs.get(&jid) {
            Some(job) => Some(*job),
            None => None,
        }
    }

    // Adds a new value to the job list with the following pid, state, and cmdline and returns its jid
    pub fn add(&self, pid: usize, state: State, cmdline: &'a str) -> Result<usize, String> {
        let JobList(cell) = self;
        let mut job_list = cell.borrow_mut();

        // Calculate jid of new job
        let jid = match job_list.max_jid {
            None => 0,
            Some(id) => id + 1,
        };

        // Update foreground
        if let State::FG = state {
            if let Some(_) = job_list.fg_job {
                return Err(
                    "Can't add a foreground job if a foreground job already exists".to_string(),
                );
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
            return Err("Inserted job with duplicate jid".to_string());
        }

        Ok(jid)
    }

    // Deletes a job from the job list
    pub fn delete(&self, jid: usize) -> bool {
        let JobList(cell) = self;
        let mut job_list = cell.borrow_mut();

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

    // gets the jid of the current forground job
    pub fn fg_job(&self) -> Option<usize> {
        let JobList(cell) = self;
        let job_list = cell.borrow();
        job_list.fg_job
    }

    // Returns the jid associated with any pid in the job list
    pub fn pid_to_jid(&self, pid: usize) -> Option<usize> {
        let JobList(cell) = self;
        let job_list = cell.borrow();

        let (jid, _) = job_list.jobs.iter().find(|(_, job)| job.pid == pid)?;
        Some(*jid)
    }

    // Returns the state of any one job
    pub fn get_state(&self, jid: usize) -> Option<State> {
        let job = self.get(jid)?;
        Some(job.state)
    }

    // Alters the state of a given job
    pub fn set_state(&self, jid: usize, state: State) -> bool {
        let JobList(cell) = self;
        let mut job_list = cell.borrow_mut();

        if state == State::FG {
            if let Some(x) = job_list.fg_job {
                if jid != x {
                    return false;
                }
            } else {
                job_list.fg_job = Some(jid);
            }
        }

        let temp = match job_list.jobs.get_mut(&jid) {
            None => Some(false),
            Some(job) => {
                if state == job.state {
                    Some(true)
                } else if State::FG == job.state {
                    job.state = state;
                    None
                } else {
                    job.state = state;
                    Some(true)
                }
            }
        };

        match temp {
            None => {
                job_list.fg_job = None;
                true
            }
            Some(b) => b,
        }
    }

    // gets the pid associated by a pid
    pub fn get_pid(&self, jid: usize) -> Option<usize> {
        let job = self.get(jid)?;
        Some(job.pid)
    }

    // Gets the cmdline of a job
    pub fn get_cmdline(&self, jid: usize) -> Option<&str> {
        let job = self.get(jid)?;
        Some(job.cmdline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adding_jobs() {
        let list = JobList::new();
        let result = list.add(1, State::FG, "one");
        assert_eq!(Ok(0), result);
        let result = list.add(2, State::BG, "two");
        assert_eq!(Ok(1), result);
        let result = list.add(3, State::FG, "three");
        assert_eq!(
            Err("Can't add a foreground job if a foreground job already exists".to_string()),
            result
        );
        let result = list.add(3, State::BG, "three");
        assert_eq!(Ok(2), result);
    }

    #[test]
    fn get_jobs() {
        let list = JobList::new();
        list.add(1, State::FG, "one").unwrap();
        list.add(2, State::BG, "two").unwrap();
        list.add(3, State::BG, "three").unwrap();
        assert_eq!(Some(1), list.get_pid(0));
        assert_eq!(Some(State::FG), list.get_state(0));
        assert_eq!(Some("one"), list.get_cmdline(0));
        assert_eq!(Some(2), list.get_pid(1));
        assert_eq!(Some(State::BG), list.get_state(1));
        assert_eq!(Some("two"), list.get_cmdline(1));
        assert_eq!(Some(3), list.get_pid(2));
        assert_eq!(Some(State::BG), list.get_state(2));
        assert_eq!(Some("three"), list.get_cmdline(2));
        assert_eq!(None, list.get_pid(3));
    }

    #[test]
    fn delete_jobs() {
        let list = JobList::new();
        list.add(1, State::FG, "one").unwrap();
        list.add(2, State::BG, "two").unwrap();
        list.add(3, State::BG, "three").unwrap();
        assert_eq!(false, list.delete(3));
        assert_eq!(true, list.delete(1));
        assert_eq!(None, list.get_pid(1));
        assert_eq!(Ok(3), list.add(4, State::BG, "four"));
        assert_eq!(true, list.delete(3));
        assert_eq!(None, list.get_pid(3));
        assert_eq!(true, list.delete(2));
        assert_eq!(None, list.get_pid(2));
        assert_eq!(Ok(1), list.add(5, State::BG, "four"));
    }

    #[test]
    fn fg_jobs() {
        let list = JobList::new();
        list.add(1, State::BG, "one").unwrap();
        assert_eq!(None, list.fg_job());
        list.add(2, State::FG, "two").unwrap();
        assert_eq!(Some(1), list.fg_job());
    }

    #[test]
    fn pid_to_jid_test() {
        let list = JobList::new();
        list.add(1, State::FG, "one").unwrap();
        list.add(2, State::BG, "two").unwrap();
        list.add(3, State::BG, "three").unwrap();
        assert_eq!(None, list.pid_to_jid(0));
        assert_eq!(Some(0), list.pid_to_jid(1));
        assert_eq!(Some(1), list.pid_to_jid(2));
        assert_eq!(Some(2), list.pid_to_jid(3));
    }

    #[test]
    fn state_sets() {
        let list = JobList::new();
        list.add(1, State::FG, "one").unwrap();
        assert_eq!(true, list.set_state(0, State::BG));
        assert_eq!(true, list.set_state(0, State::FG));
        list.add(2, State::BG, "two").unwrap();
        assert_eq!(false, list.set_state(1, State::FG));
        assert_eq!(true, list.set_state(0, State::BG));
        assert_eq!(true, list.set_state(0, State::FG));
    }
}
