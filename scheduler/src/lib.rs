//! A scheduler library.
//!
//! This library provides the traits and structures necessary
//! to implement a process scheduler.
//!

use std::num::NonZeroUsize;

mod scheduler;

use schedulers::{RoundRobin, RoundRobinPQ, CFS};

pub use crate::scheduler::{
    Pid, Process, ProcessState, Scheduler, SchedulingDecision, StopReason, Syscall, SyscallResult,
};

mod schedulers;

use std::collections::VecDeque;


/// Returns a structure that implements the `Scheduler` trait with a round robin scheduler policy
///
/// * `timeslice` - the time quanta that a process can run before it is preempted
/// * `minimum_remaining_timeslice` - when a process makes a system call, the scheduler
///                                 has to decode whether to schedule it again for the
///                                 remaining time of its quanta, or to schedule a new
///                                 process. The scheduler will schedule the process
///                                 again of the remaining quanta is greater or equal to
///                                 the `minimum_remaining_timeslice` value.
#[allow(unused_variables)]
pub fn round_robin(timeslice: NonZeroUsize, minimum_remaining_timeslice: usize) -> impl Scheduler {
    RoundRobin {
        ready_q: VecDeque::new(),
        wait_q: VecDeque::new(),
        sleep_q: VecDeque::new(),
        timeslice: timeslice,
        minimum_remaining_timeslice: minimum_remaining_timeslice,
        init_pid: 0,
        panic_state: true,
        sleep_time: 0,
        default_timeslice: timeslice,
    }
}

/// Returns a structure that implements the `Scheduler` trait with a priority queue scheduler policy
/// * `timeslice` - the time quanta that a process can run before it is preempted
/// * `minimum_remaining_timeslice` - when a process makes a system call, the scheduler
///                                 has to decode whether to schedule it again for the
///                                 remaining time of its quanta, or to schedule a new
///                                 process. The scheduler will schedule the process
///                                 again of the remaining quanta is greater or equal to
///                                 the `minimum_remaining_timeslice` value.
#[allow(unused_variables)]
pub fn priority_queue(
    timeslice: NonZeroUsize,
    minimum_remaining_timeslice: usize,
) -> impl Scheduler {
    RoundRobinPQ {
        ready_q: VecDeque::new(),
        wait_q: VecDeque::new(),
        sleep_q: VecDeque::new(),
        timeslice: timeslice,
        minimum_remaining_timeslice: minimum_remaining_timeslice,
        init_pid: 0,
        panic_state: false,
        sleep_time: 0,
        default_timeslice: timeslice,
    }
}

/// Returns a structure that implements the `Scheduler` trait with a simplified [cfs](https://opensource.com/article/19/2/fair-scheduling-linux) scheduler policy
/// * `cpu_time` - the total time units that the cpu has for an iteration, this is used to compute
///                    the `timeslice` of each process.
/// * `minimum_remaining_timeslice` - when a process makes a system call, the scheduler
///                                 has to decode whether to schedule it again for the
///                                 remaining time of its quanta, or to schedule a new
///                                 process. The scheduler will schedule the process
///                                 again of the remaining quanta is greater or equal to
///                                 the `minimum_remaining_timeslice` value.
#[allow(unused_variables)]
pub fn cfs(cpu_time: NonZeroUsize, minimum_remaining_timeslice: usize) -> impl Scheduler {
    CFS {
        timeslice: cpu_time,
        minimum_remaining_timeslice: minimum_remaining_timeslice,
    }
}
