/*
 *
 * CFS NOT IMPLEMENTED!
 *
 */

use std::num::NonZeroUsize;
use std::collections::VecDeque;

pub use crate::scheduler::{
	Process, ProcessState, Scheduler, SchedulingDecision, StopReason, SyscallResult,
};

pub struct CFS {
	pub timeslice: NonZeroUsize,
	pub minimum_remaining_timeslice: usize,
}


impl Scheduler for CFS {
	fn next(&mut self) -> crate::SchedulingDecision {
		crate::SchedulingDecision::Done
	}

	fn stop(&mut self, _reason: crate::StopReason) -> crate::SyscallResult {
		crate::SyscallResult::Success
	}

	fn list(&mut self) -> Vec<&dyn crate::Process> {
		VecDeque::new().into()
	}
}
