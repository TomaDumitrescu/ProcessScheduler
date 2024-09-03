use std::num::NonZeroUsize;
use std::collections::VecDeque;

pub use crate::scheduler::{
	Pid, Process, ProcessState, Scheduler, SchedulingDecision, StopReason, Syscall, SyscallResult,
};

#[derive(Clone)]
pub struct ProcessInfo {
	pub pid: Pid,
	pub state: ProcessState,
	pub timings: (usize, usize, usize),
	pub priority: i8,
	pub sleep_time: usize,
	pub extra: String,
}

impl Process for ProcessInfo {
	fn pid(&self) -> Pid {
		self.pid
	}

	fn state(&self) -> ProcessState {
		self.state
	}

	fn timings(&self) -> (usize, usize, usize) {
		self.timings
	}

	fn priority(&self) -> i8 {
		self.priority
	}

	fn extra(&self) -> String {
		self.extra.clone()
	}
}

/// Round Robin scheduler struct
pub struct RoundRobin {
	pub ready_q: VecDeque<ProcessInfo>,
	pub wait_q: VecDeque<ProcessInfo>,
	pub sleep_q: VecDeque<ProcessInfo>,
	pub timeslice: NonZeroUsize,
	pub minimum_remaining_timeslice: usize,
	pub init_pid: usize,
	pub panic_state: bool,
	pub sleep_time: usize,
	pub default_timeslice: NonZeroUsize,
}

impl RoundRobin {
	/// Verify if the proc pid 1 is the last to exit
	fn panic_verify(&self) -> bool {
		let mut init_proc: bool = false;
		for p in self.ready_q.iter() {
			if p.pid() == 1 {
				init_proc = true;
			}
		}

		for p in self.wait_q.iter() {
			if p.pid() == 1 {
				init_proc = true;
			}
		}

		for p in self.sleep_q.iter() {
			if p.pid() == 1 {
				init_proc = true;
			}
		}

		!init_proc && self.panic_state
	}

	/// Verifies if there are only waiting processes
	fn deadlock_verify(&self) -> bool {
		if self.ready_q.is_empty() && !self.wait_q.is_empty()
			&& self.sleep_q.is_empty() {
			// because of if
			return true;
		}

		false
	}
}

impl Scheduler for RoundRobin {
	/// Using RoundRobin fields, a scheduling decision is returned
	fn next(&mut self) -> crate::SchedulingDecision {
		// the scheduler should be in a valid state
		if self.panic_verify() {
			return crate::SchedulingDecision::Panic;
		}

		// the scheduler should not run in an infinite loop
		if self.deadlock_verify() {
			return crate::SchedulingDecision::Deadlock;
		}

		// no processes to plan, then close the abstractized processor
		if self.ready_q.is_empty() && self.wait_q.is_empty()
			&& self.sleep_q.is_empty() {
			return crate::SchedulingDecision::Done;
		}

		// scheduler sleep time is added to waiting processes
		for p in self.wait_q.iter_mut() {
			p.timings.0 += self.sleep_time;
		}

		// the scheduler sleep time should be minimum for efficiency
		let mut min_sleep_time: usize = 1000000;
		for p in self.sleep_q.iter_mut() {
			// retaining previous scheduler sleep_time (possibly 0)
			p.timings.0 += self.sleep_time;

			// eventually find some ready processes to put in the queue
			if p.sleep_time == 0 {
				// process state changed here
				p.state = ProcessState::Ready;
				// no multiple mutable references allowed
				self.ready_q.push_back((*p).clone());
			}

			if min_sleep_time > p.sleep_time {
				min_sleep_time = p.sleep_time;
			}
		}

		// p.sleep_time = 0, then p is in ready_q
		self.sleep_q.retain(|p| p.sleep_time > 0);

		// no ready processes to plen, then put scheduler in sleep state
		if self.ready_q.is_empty() {
			for p in self.sleep_q.iter_mut() {
				// simulate sleeping time
				if p.sleep_time >= min_sleep_time {
					p.sleep_time -= min_sleep_time;
				} else {
					p.sleep_time = 0;
				}
			}

			// sleeping time to add to waiting processes
			self.sleep_time = min_sleep_time;
			return crate::SchedulingDecision::Sleep(NonZeroUsize::new(min_sleep_time).unwrap());
		}

		// if code reaches here, then at least one process is ready or running
		let mut proc = self.ready_q.pop_front().unwrap();
		let current_pid = proc.pid;

		// move from running to intermediary ready
		if self.ready_q.is_empty() {
			proc.state = ProcessState::Ready;
		}

		// case for the first fork or a single running process
		if proc.state == ProcessState::Ready {
			proc.state = ProcessState::Running;
			// running process is always on the front of the ready queue
			self.ready_q.push_front(proc);

			return crate::SchedulingDecision::Run {
				pid: current_pid,
				timeslice: self.timeslice,
			};
		}

		// plan the next ready process
		if proc.state == ProcessState::Running {
			let mut next_proc = self.ready_q.pop_front().unwrap();
			proc.state = ProcessState::Ready;
			self.ready_q.push_back(proc);
			next_proc.state = ProcessState::Running;
			self.ready_q.push_front(next_proc.clone());

			return crate::SchedulingDecision::Run {
				pid: next_proc.pid,
				timeslice: self.timeslice,
			};
		}

		// no other processes to plan
		crate::SchedulingDecision::Done
	}

	/// Simulates a syscall and time using remaining and self.timeslice variables
	fn stop(&mut self, reason: crate::StopReason) -> crate::SyscallResult {
		match reason {
			StopReason::Syscall { syscall, remaining } => {
				// time update for sleeping processes
				for p in self.sleep_q.iter_mut() {
					p.timings.0 += self.timeslice.get() - remaining;

					// required because sleep_time cannot be negative and it should be usize
					if p.sleep_time as i32 - ((self.timeslice.get() - remaining) as i32) < 0 {
						p.sleep_time = 0;
						// added again at ready_q time update
						p.timings.0 -= self.timeslice.get() - remaining;
						p.state = ProcessState::Ready;

						// because multiple mutable references are not allowed
						self.ready_q.push_back((*p).clone());
					} else {
						// safe susbstraction
						p.sleep_time -= self.timeslice.get() - remaining;
					}
				}

				// reset sleep_time, because other scheduler state is considered
				self.sleep_time = 0;
				self.sleep_q.retain(|p| p.sleep_time > 0);

				if !self.ready_q.is_empty() {
					let mut act_proc = self.ready_q.pop_front().unwrap();
					// the current running process generated the syscall
					act_proc.timings.1 += 1;
					// only the running process increases the execution time
					act_proc.timings.2 += self.timeslice.get() - remaining - 1;
					self.ready_q.push_front(act_proc);
				}

				// time update for ready_q
				for p in self.ready_q.iter_mut() {
					p.timings.0 += self.timeslice.get() - remaining;
				}

				// time update for wait_q
				for p in self.wait_q.iter_mut() {
					p.timings.0 += self.timeslice.get() - remaining;
				}

				// reset timeslice to default RoundRobin 5 value
				self.timeslice = self.default_timeslice;

				match syscall {
					Syscall::Fork(priority) => {
						// increasing pids
						self.init_pid += 1;

						// instantiate the new process
						let new_proc = ProcessInfo {
							pid: Pid::new(self.init_pid),
							state: ProcessState::Ready,
							timings: (0, 0, 0),
							priority: priority,
							sleep_time: 0,
							extra: String::new(),
						};


						// this allows unwrap
						if !self.ready_q.is_empty() {
							let mut prev_proc = self.ready_q.pop_front().unwrap();

							// move from running to ready state to be again planified
							if remaining >= self.minimum_remaining_timeslice {
								prev_proc.state = ProcessState::Ready;
							}

							self.ready_q.push_front(prev_proc);
						}

						let pid_return = new_proc.pid;
						self.ready_q.push_back(new_proc);

						// change the timeslice
						if remaining >= self.minimum_remaining_timeslice {
							self.timeslice = NonZeroUsize::new(remaining).unwrap();
						} else {
							self.timeslice = self.default_timeslice;
						}

						return SyscallResult::Pid(pid_return);
					},

					Syscall::Wait(event_num) => {
						// a process should generate this syscall however
						if self.ready_q.is_empty() {
							return SyscallResult::NoRunningProcess;
						}

						let mut act_proc = self.ready_q.pop_front().unwrap();

						act_proc.state = ProcessState::Waiting { event: Some(event_num) };
						self.wait_q.push_back(act_proc);

						return SyscallResult::Success;
					},

					Syscall::Sleep(t) => {
						if self.ready_q.is_empty() {
							return SyscallResult::NoRunningProcess;
						}

						let mut act_proc = self.ready_q.pop_front().unwrap();

						// actualize the sleep_time field
						act_proc.sleep_time = t;
						// the sleep state from the project documentation
						act_proc.state = ProcessState::Waiting{ event: None };

						self.sleep_q.push_back(act_proc);
						return SyscallResult::Success;
					},

					Syscall::Signal(event_num) => {
						if self.ready_q.is_empty() {
							return SyscallResult::NoRunningProcess;
						}

						let mut act_process = self.ready_q.pop_front().unwrap();
						// do not planify again the process that sent the signal
						if remaining < self.minimum_remaining_timeslice {
							act_process.state = ProcessState::Ready;
						}

						// algorithm to move the waiting process to ready_queue
						let mut len = self.wait_q.len();
						let mut idx = 0;
						let mut removed;

						while idx < len {
							removed = false;
							// find the event the process is waiting for using match
							match self.wait_q[idx].state {
								ProcessState::Waiting { event: Some(src_num) } => {
									if src_num == event_num {
										let mut proc = self.wait_q.remove(idx).unwrap();
										proc.state = ProcessState::Ready;

										self.ready_q.push_back(proc);
										len -= 1;
										removed = true;
									}
								},

								_ => { },
							}

							idx += 1;
							// elements moved to the left, so increase idx
							// to not skip elements
							if removed {
								idx -= 1;
							}
						}

						// change the timeslice
						if act_process.state == ProcessState::Ready {
							self.timeslice = self.default_timeslice;
							self.ready_q.push_back(act_process);
						} else {
							self.timeslice = NonZeroUsize::new(remaining).unwrap();
							act_process.state = ProcessState::Ready;
							self.ready_q.push_front(act_process);
						}

						return SyscallResult::Success;
					},

					Syscall::Exit => {
						// a process should send a syscall
						if self.ready_q.is_empty() {
							return SyscallResult::NoRunningProcess;
						}

						let proc = self.ready_q.pop_front().unwrap();

						// ok if the last process is the one with pid 1 and calls exit
						if proc.pid() == 1 && self.ready_q.is_empty() && self.wait_q.is_empty()
							&& self.sleep_q.is_empty() {
							self.panic_state = false;
						}

						return SyscallResult::Success;
					},
				}
			},

			// the elapsed time is self.timeslice
			StopReason::Expired => {
				for p in self.sleep_q.iter_mut() {
					p.timings.0 += self.timeslice.get();
					if p.sleep_time as i32 - (self.timeslice.get() as i32) <= 0 {
						p.state = ProcessState::Ready;
						p.timings.0 -= self.timeslice.get();
						p.sleep_time = 0;
						self.ready_q.push_back((*p).clone());
					} else {
						p.sleep_time -= self.timeslice.get();
					}
				}

				self.sleep_q.retain(|p| p.sleep_time > 0);

				let mut act_proc = self.ready_q.pop_front().unwrap();
				act_proc.timings.2 += self.timeslice.get();
				self.ready_q.push_front(act_proc);

				for p in self.ready_q.iter_mut() {
					p.timings.0 += self.timeslice.get();
				}

				for p in self.wait_q.iter_mut() {
					p.timings.0 += self.timeslice.get();
				}

				self.timeslice = self.default_timeslice;

				match self.ready_q.pop_front() {
					Some(mut proc) => {
						proc.state = ProcessState::Ready;
						self.ready_q.push_back(proc);

						return SyscallResult::Success;
					},

					None => SyscallResult::NoRunningProcess,
				}
			},
		}
	}

	/// Used to display the processes in a pretty format
	fn list(&mut self) -> Vec<&dyn crate::Process> {
		let mut combine_procs: Vec<&dyn crate::Process> = Vec::new();

		// add all processes to a vector
		combine_procs.extend(self.sleep_q.iter().map(|proc| proc as &dyn Process));
		combine_procs.extend(self.wait_q.iter().map(|proc| proc as &dyn Process));
		combine_procs.extend(self.ready_q.iter().map(|proc| proc as &dyn Process));

		// sort the vector using the pid as the field
		combine_procs.sort_by_key(|proc| proc.pid());

		combine_procs
	}
}
