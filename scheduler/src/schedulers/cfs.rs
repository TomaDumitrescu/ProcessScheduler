use std::num::NonZeroUsize;
use std::collections::VecDeque;

pub use crate::scheduler::{
	Pid, Process, ProcessState, Scheduler, SchedulingDecision, StopReason, Syscall, SyscallResult,
};

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
		return self.pid;
	}

	fn state(&self) -> ProcessState{
		return self.state;
	}

	fn timings(&self) -> (usize, usize, usize) {
		return self.timings;
	}

	fn priority(&self) -> i8 {
		return self.priority;
	}

	fn extra(&self) -> String {
		return self.extra.clone();
	}
}

pub struct CFS {
	pub ready_proc: VecDeque<ProcessInfo>,
	pub waiting_queue: VecDeque<ProcessInfo>,
	pub sleep_queue: VecDeque<ProcessInfo>,
	pub timeslice: NonZeroUsize,
	pub minimum_remaining_timeslice: usize,
	pub init_pid: usize,
	pub current_time: usize,
	pub panic_state: bool,
}

impl Scheduler for CFS {
	fn next(&mut self) -> crate::SchedulingDecision {
		self.current_time += 1;

		if self.panic_state == true {
			return crate::SchedulingDecision::Panic;
		}

		if self.ready_proc.is_empty() {
			if !self.waiting_queue.is_empty() && self.sleep_queue.is_empty() {
				return crate::SchedulingDecision::Deadlock;
			} else if !self.sleep_queue.is_empty() {
				let proc = self.sleep_queue.pop_back().unwrap();
				let time_to_sleep: usize = proc.sleep_time;
				self.sleep_queue.push_back(proc);
				return crate::SchedulingDecision::Sleep(NonZeroUsize::new(time_to_sleep).unwrap());
			} else {
				return crate::SchedulingDecision::Done;
			}
		}

		let mut proc = self.ready_proc.pop_front().unwrap();
		let current_pid = proc.pid;

		if proc.state == ProcessState::Ready {
			proc.state = ProcessState::Running;
			self.ready_proc.push_front(proc);
		}

		return crate::SchedulingDecision::Run {
			pid: current_pid,
			timeslice: self.timeslice,
		};
	}

	fn stop(&mut self, reason: crate::StopReason) -> crate::SyscallResult {
		self.current_time += 1;

		match reason {
			StopReason::Syscall { syscall, remaining } => {
				match syscall {
					Syscall::Fork(priority) => {
						self.init_pid += 1;

						let new_proc = ProcessInfo{
							pid: Pid::new(self.init_pid),
							state: ProcessState::Ready,
							timings: (0, 0, 0),
							priority,
							sleep_time: 0,
							extra: String::new(),
						};

						let pid_return = new_proc.pid;
						self.ready_proc.push_back(new_proc);

						return SyscallResult::Pid(pid_return);
					},

					Syscall::Wait(event_num) => {
						let mut act_proc = self.ready_proc.pop_front().unwrap();

						act_proc.timings.1 += 1;
						act_proc.state = ProcessState::Waiting { event: Some(event_num) };
						self.waiting_queue.push_back(act_proc);

						return SyscallResult::Success;
					},

					Syscall::Sleep(t) => {
						let mut act_proc = self.ready_proc.pop_front().unwrap();

						act_proc.timings.1 += 1;
						act_proc.sleep_time = t;
						act_proc.state = ProcessState::Waiting{ event: None };

						self.sleep_queue.push_back(act_proc);
						return SyscallResult::Success;
					},

					Syscall::Signal(event_num) => {
						let mut act_process = self.ready_proc.pop_front().unwrap();
						act_process.state = ProcessState::Ready;
						let mut len = self.waiting_queue.len();
						let mut idx = 0;

						while idx < len {
							match self.waiting_queue[idx].state {
								ProcessState::Waiting { event: Some(src_num) } => {
									if src_num == event_num {
										let mut proc = self.waiting_queue.remove(idx).unwrap();
										proc.state = ProcessState::Ready;

										self.ready_proc.push_back(proc);
										len -= 1;
										idx -= 1;
									}
								},

								_ => { },
							}

							idx += 1;
						}

						return SyscallResult::Success;
					},

					Syscall::Exit => {
						self.ready_proc.pop_front();

						return SyscallResult::Success;
					},
				}
			},

			StopReason::Expired => {
				match self.ready_proc.pop_front() {
					Some(mut proc) => {
						let proc_times = proc.timings;
						let remaining = proc_times.0 - proc_times.1 - proc_times.2;
						if remaining >= self.minimum_remaining_timeslice {
							proc.state = ProcessState::Ready;
							self.ready_proc.push_back(proc);
						}

						return SyscallResult::Success;
					},

					None => SyscallResult::NoRunningProcess,
				}
			},
		}
	}

	fn list(&mut self) -> Vec<&dyn crate::Process> {
		let mut combine_procs: Vec<&dyn crate::Process> = Vec::new();

		combine_procs.extend(self.sleep_queue.iter().map(|proc| proc as &dyn Process));
		combine_procs.extend(self.waiting_queue.iter().map(|proc| proc as &dyn Process));
		combine_procs.extend(self.ready_proc.iter().map(|proc| proc as &dyn Process));

		combine_procs.sort_by_key(|proc| proc.pid());


		return combine_procs;
	}
}
