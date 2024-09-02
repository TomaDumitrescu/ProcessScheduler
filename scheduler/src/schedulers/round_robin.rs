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

pub struct RoundRobin {
	pub ready_q: VecDeque<ProcessInfo>,
	pub wait_q: VecDeque<ProcessInfo>,
	pub sleep_q: VecDeque<ProcessInfo>,
	pub timeslice: NonZeroUsize,
	pub minimum_remaining_timeslice: usize,
	pub init_pid: usize,
	pub current_time: usize,
	pub panic_state: bool,
	pub sleep_time: usize,
}

impl RoundRobin {
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

	fn deadlock_verify(&self) -> bool {
		if self.ready_q.is_empty() && !self.wait_q.is_empty()
			&& self.sleep_q.is_empty() {
				return true;
		}

		false
	}
}

impl Scheduler for RoundRobin {
	fn next(&mut self) -> crate::SchedulingDecision {
		self.current_time += 1;

		if self.panic_verify() {
			return crate::SchedulingDecision::Panic;
		}

		if self.deadlock_verify() {
			return crate::SchedulingDecision::Deadlock;
		}

		if self.ready_q.is_empty() && self.wait_q.is_empty()
			&& self.sleep_q.is_empty() {
				return crate::SchedulingDecision::Done;
		}

		let mut min_sleep_time: usize = 1000000;
		for p in self.sleep_q.iter_mut() {
			p.timings.0 += self.sleep_time;
			let time_to_sleep: usize = p.sleep_time;

			if p.sleep_time == 0 {
				self.ready_q.push_back((*p).clone());
			}

			if min_sleep_time > p.sleep_time {
				min_sleep_time = p.sleep_time;
			}
		}

		self.sleep_q.retain(|p| p.sleep_time > 0);

		if self.ready_q.is_empty() {
			for p in self.sleep_q.iter_mut() {
				if p.sleep_time >= min_sleep_time {
					p.sleep_time -= min_sleep_time;
				} else {
					p.sleep_time = 0;
				}
			}

			self.sleep_time = min_sleep_time;
			return crate::SchedulingDecision::Sleep(NonZeroUsize::new(min_sleep_time).unwrap());
		}

		let mut proc = self.ready_q.pop_front().unwrap();
		let current_pid = proc.pid;

		if proc.state == ProcessState::Running {
			if self.ready_q.is_empty() {
				return crate::SchedulingDecision::Done;
			}

			let mut next_proc = self.ready_q.pop_front().unwrap();
			if next_proc.state == ProcessState::Ready {
				proc.state = ProcessState::Ready;
				self.ready_q.push_back(proc);
				next_proc.state = ProcessState::Running;
				self.ready_q.push_front(next_proc.clone());

				return crate::SchedulingDecision::Run {
					pid: next_proc.pid,
					timeslice: self.timeslice,
				};
			}

			return crate::SchedulingDecision::Done;
		} else {
			proc.state = ProcessState::Running;
			self.ready_q.push_front(proc);
		}

		crate::SchedulingDecision::Run {
			pid: current_pid,
			timeslice: self.timeslice,
		}
	}

	fn stop(&mut self, reason: crate::StopReason) -> crate::SyscallResult {
		self.current_time += 1;

		match reason {
			StopReason::Syscall { syscall, remaining } => {
				// Check to have a process already
				// Simulate time
				for p in self.sleep_q.iter_mut() {
					p.timings.0 += self.timeslice.get() - remaining;
					if p.sleep_time as i32 - ((self.timeslice.get() - remaining) as i32) < 0 {
						p.sleep_time = 0; 
					} else {
						p.sleep_time -= self.timeslice.get() - remaining;
					}

					p.timings.0 += self.sleep_time;
				}

				self.sleep_time = 0;

				if !self.ready_q.is_empty() {
					let mut act_proc = self.ready_q.pop_front().unwrap();
					act_proc.timings.1 += 1;
					act_proc.timings.2 += self.timeslice.get() - remaining - 1;
					self.ready_q.push_front(act_proc);
				}

				for p in self.ready_q.iter_mut() {
					p.timings.0 += self.timeslice.get() - remaining;
				}

				for p in self.wait_q.iter_mut() {
					p.timings.0 += self.timeslice.get() - remaining;
				}

				self.timeslice = NonZeroUsize::new(5).unwrap();

				match syscall {
					Syscall::Fork(priority) => {
						self.init_pid += 1;

						let new_proc = ProcessInfo {
							pid: Pid::new(self.init_pid),
							state: ProcessState::Ready,
							timings: (0, 0, 0),
							priority: priority,
							sleep_time: 0,
							extra: String::new(),
						};

						if !self.ready_q.is_empty() {
							let mut prev_proc = self.ready_q.pop_front().unwrap();

							if remaining >= self.minimum_remaining_timeslice {
								prev_proc.state = ProcessState::Ready;
							}

							self.ready_q.push_front(prev_proc);
						}

						let pid_return = new_proc.pid;
						self.ready_q.push_back(new_proc);

						if remaining != 0 {
							self.timeslice = NonZeroUsize::new(remaining as usize).unwrap();
						}

						return SyscallResult::Pid(pid_return);
					},

					Syscall::Wait(event_num) => {
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

						act_proc.sleep_time = t;
						act_proc.state = ProcessState::Waiting{ event: None };

						self.sleep_q.push_back(act_proc);
						return SyscallResult::Success;
					},

					Syscall::Signal(event_num) => {
						if self.ready_q.is_empty() {
							return SyscallResult::NoRunningProcess;
						}

						let mut act_process = self.ready_q.pop_front().unwrap();
						if remaining >= self.minimum_remaining_timeslice {
							act_process.state = ProcessState::Ready;
						}

						let mut len = self.wait_q.len();
						let mut idx = 0;

						while idx < len {
							match self.wait_q[idx].state {
								ProcessState::Waiting { event: Some(src_num) } => {
									if src_num == event_num {
										let mut proc = self.wait_q.remove(idx).unwrap();
										proc.state = ProcessState::Ready;

										self.ready_q.push_back(proc);
										len -= 1;
										if idx > 0 {
											idx -= 1;
										}
									}
								},

								_ => { },
							}

							idx += 1;
						}

						return SyscallResult::Success;
					},

					Syscall::Exit => {
						if self.ready_q.is_empty() {
							return SyscallResult::NoRunningProcess;
						}

						let proc = self.ready_q.pop_front().unwrap();

						if proc.pid() == 1 && self.ready_q.is_empty() && self.wait_q.is_empty()
							&& self.sleep_q.is_empty() {
							self.panic_state = false;
						}

						return SyscallResult::Success;
					},
				}
			},

			StopReason::Expired => {
				for p in self.sleep_q.iter_mut() {
					p.timings.0 += self.timeslice.get();
					if p.sleep_time as i32 - (self.timeslice.get() as i32) < 0 {
						p.sleep_time = 0;
					} else {
						p.sleep_time -= self.timeslice.get();
					}
				}

				if !self.ready_q.is_empty() {
					let mut act_proc = self.ready_q.pop_front().unwrap();
					act_proc.timings.2 += self.timeslice.get();
					self.ready_q.push_front(act_proc);
				}

				for p in self.ready_q.iter_mut() {
					p.timings.0 += self.timeslice.get();
				}

				for p in self.wait_q.iter_mut() {
					p.timings.0 += self.timeslice.get();
				}

				self.timeslice = NonZeroUsize::new(5).unwrap();

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

	fn list(&mut self) -> Vec<&dyn crate::Process> {
		let mut combine_procs: Vec<&dyn crate::Process> = Vec::new();

		combine_procs.extend(self.sleep_q.iter().map(|proc| proc as &dyn Process));
		combine_procs.extend(self.wait_q.iter().map(|proc| proc as &dyn Process));
		combine_procs.extend(self.ready_q.iter().map(|proc| proc as &dyn Process));

		combine_procs.sort_by_key(|proc| proc.pid());


		return combine_procs;
	}
}
