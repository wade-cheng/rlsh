use std::{cell::RefCell, cmp};

const MAXJOBS: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    BG,
    FG,
    ST,
    NT,
}

#[derive(Clone, Copy)]
pub struct Job<'a> {
    pid: usize,
    state: State,
    cmdline: &'a str,
}

struct JobData<'a> {
    jobs: [Job<'a>; MAXJOBS],
    fg_job: Option<usize>,
    max_jid: Option<usize>,
}

// List to manage jobs
pub struct JobList<'a>(JobData<'a>);

impl<'a> JobList<'a> {
    // Creates a new empty job list
    pub fn new() -> Self {
        JobList(JobData {
            jobs: [Job {
                pid: 0,
                state: State::NT,
                cmdline: "",
            }; MAXJOBS],
            fg_job: None,
            max_jid: None,
        })
    }

    // Gets the job with the assiciated jid
    pub fn get(&self, jid: usize) -> Option<Job> {
        let JobList(job_data) = self;
        let job =job_data.jobs.get(jid)?;
        if job.state == State::NT {
            return None;
        }
        Some(*job)
    }

    // Adds a new value to the job list with the following pid, state, and cmdline and returns its jid
    pub fn add(&mut self, pid: usize, state: State, cmdline: &'a str) -> Result<usize, &'static str> {
        if let State::NT = state {
            return Err("Invalid state for new job");
        }

        let JobList(job_data) = self;

        // Calculate jid of new job
        let jid = match job_data.max_jid {
            None => 0,
            Some(id) => {
                if id + 1 >= MAXJOBS {
                  return Err("Too many jobs");
                }
                id + 1
            },
        };

        // Update foreground
        if let State::FG = state {
            if let Some(_) = job_data.fg_job {
                return Err(
                    "Can't add a foreground job if a foreground job already exists",
                );
            } else {
                job_data.fg_job = Some(jid)
            }
        }

        // update the max jid in the list
        job_data.max_jid = Some(jid);

        // throw error if insert triggers an override
        if job_data.jobs[jid].state != State::NT {
            return Err("Inserted job with duplicate jid");
        }

        // Create job
        job_data.jobs[jid] = Job {
            pid,
            state,
            cmdline,
        };

        Ok(jid)
    }

    // Deletes a job from the job list
    pub fn delete(&mut self, jid: usize) -> bool {
        let JobList(job_data) = self;

        // out of bounds check
        if jid >= MAXJOBS {
            return false;
        } 

        
        let job = &mut job_data.jobs[jid];

        // double remove check
        if let State::NT = job.state {
            return false;
        }

        job.state = State::NT;

        // update max jid
        if let Some(id) = job_data.max_jid {
            if jid == id {
                job_data.max_jid = job_data.jobs.iter().rposition(|job| job.state != State::NT);
            }
        }

        // update foreground job
        if let Some(id) = job_data.fg_job {
            if id == jid {
                job_data.fg_job = None
            }
        }

        // return if successful remove
        true
    }

    // gets the jid of the current forground job
    pub fn fg_job(&self) -> Option<usize> {
        let JobList(job_data) = self;
        job_data.fg_job
    }

    // Returns the jid associated with any pid in the job list
    pub fn pid_to_jid(&self, pid: usize) -> Option<usize> {
        let JobList(job_data) = self;
        let (jid, _) = job_data.jobs.iter().enumerate().find(|(_, job)| job.state != State::NT && job.pid == pid)?;
        Some(jid)
    }

    // Returns the state of any one job
    pub fn get_state(&self, jid: usize) -> Option<State> {
        let job = self.get(jid)?;
        Some(job.state)
    }

    // Alters the state of a given job
    // returns true if the job with jid now has state
    pub fn set_state(&mut self, jid: usize, state: State) -> bool {
        let JobList(job_data) = self;

        // checks for valid jid
        if jid >= MAXJOBS || job_data.jobs[jid].state == State::NT {
            return false;
        }

        let job = &mut job_data.jobs[jid];

        // checks if valid state
        if state == State::FG {
            if let Some(x) = job_data.fg_job {
                return jid == x;
            } else {
                job_data.fg_job = Some(jid);
            }
        }

        // if state doesn't change do nothing
        if state != job.state {
            // If removing foreground job update variable
            if job.state == State::FG {
                job_data.fg_job = None;
            }

            // update state
            job.state = state;
        }

        true
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
        let mut list = JobList::new();
        let result = list.add(1, State::FG, "one");
        assert_eq!(Ok(0), result);
        let result = list.add(2, State::BG, "two");
        assert_eq!(Ok(1), result);
        let result = list.add(3, State::FG, "three");
        assert_eq!(
            Err("Can't add a foreground job if a foreground job already exists"),
            result
        );
        let result = list.add(3, State::BG, "three");
        assert_eq!(Ok(2), result);
    }

    #[test]
    fn get_jobs() {
        let mut list = JobList::new();
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
        let mut list = JobList::new();
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
        let mut list = JobList::new();
        list.add(1, State::BG, "one").unwrap();
        assert_eq!(None, list.fg_job());
        list.add(2, State::FG, "two").unwrap();
        assert_eq!(Some(1), list.fg_job());
    }

    #[test]
    fn pid_to_jid_test() {
        let mut list = JobList::new();
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
        let mut list = JobList::new();
        list.add(1, State::FG, "one").unwrap();
        assert_eq!(true, list.set_state(0, State::BG));
        assert_eq!(Some(State::BG), list.get_state(0));
        assert_eq!(true, list.set_state(0, State::FG));
        assert_eq!(Some(State::FG), list.get_state(0));
        list.add(2, State::BG, "two").unwrap();
        assert_eq!(false, list.set_state(1, State::FG));
        assert_eq!(Some(State::BG), list.get_state(1));
        assert_eq!(true, list.set_state(0, State::BG));
        assert_eq!(Some(State::BG), list.get_state(0));
        assert_eq!(true, list.set_state(1, State::FG));
        assert_eq!(Some(State::FG), list.get_state(1));
    }
}
