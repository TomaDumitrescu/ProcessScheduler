## Copyright 2024 ~ Dumitrescu Toma-Ioan

## Faculty of Automatic Control and Computer Science

## Description

The program resembles a process scheduler with different process
planing algorithms - Round Robin where processes have equal priorities,
Round Robin and Completely Fair Scheduler. The algorithms are defined
in scheduler/src/schedulers by implementing the trait Scheduler multiple
times. The system is represented by a virtual uniprocessor with an abstract
time and simulates the behavior of the processes and basic syscalls (fork,
wait, exit, sleep, signal) in a real 1-core OS. After a syscall, the time
is tracked using remaining and the actual timeslice of the scheduler and
a scheduling decision is taken, the decision being identified using an
enum (SchedulingDecision).

Testing command example:
TIMESLICE=5 REMAINING=2 cargo test --bin "runner" workers -q --features="round-robin"


## Implementation details

The main difficulty comes from the syntax, so the thinking was translated in Rust
using the ProcessScheduler project and Rust language documentations.

Scheduler @ Simple Round Robin:

ProcessInfo struct - original Process struct fields
ProcessInfo has Process trait functions implemented

RoundRobin struct - 3 vector deques for each state
a process can be. The ready_q has on front only the
running process, timeslice for a process till
expiring, minimum_remaining_timeslices - decides if the
process runned long enough or if it should be replanned,
init_pid starting from 0 to represent the new processes,
panic_state assures the program to not panic if the last
process is the one with pid 1 and exited, sleep_time is
used when the scheduler is put in a sleep state with
sleep_time = changed_timeslice to simulate elapsed time

panic_verify and deadlock_verify are RoundRobin specific
functions for code modularity

RoundRobin ~ next function:

General idea: the following algorithm will return a SchedulingDecision
enum value from the list {Run {pid, timeslice}, Sleep(time_units),
Deadlock, Panic, Done}, each having a well defined set of requirements.
The scheduler will be tested if it meets the requirements for the
previous decisions, in the order: Panic, Deadlock, Sleep, Run, Done.

- verify if the scheduler state is valid (not deadlock or panic) or
if there are process to plan
- add sleep_time for each waiting or sleeping process (if the
scheduler was in a sleep state, then it was no ready process, so
it should not count there. If it wasn't in a sleep state, it
will add only zeroes)
- check if the processes set meet the requirements for the
sleep scheduling decision (no ready process available ad the
sleeping queue will always contain at least one element since
we've already check deadlock). Iterate through sleeping processes
and choose the one with the minimum sleeping time (for efficiency)
and schedule a sleep of that value, after decresing all sleep_time
fields of sleeping processes with min_sleep_time. Also, the timeslice
will be changed to min_sleep_time.
- plan the first ready process from front to back in the queue
- decide to close the program

At each - point, if the requirements are met, then the next - points
will not be accessible (every - from the RoundRobin next function has
a returning point if the code is executed there).

Some process states are changed after updating elapsed slept time or
as a logic artifice to reduce the number of code lines.

iter_mut() used over processes vectors to have the possibility to modify
processes fields in the loop
retain() used to filter the sleep_q elements

clone() used when moving a sleeping process to ready queue because no
multiple mutable references of a variable are allowed in Rust

NonZeroUsize::new(val) used for generating a NonZeroUsize for the
Sleep scheduling decision (because of the skeleton)

pop_front() (that returns Option) and push_back() to interact with
processes from VecDeque

crate:: used to specify the path from the root of the current crate

RoundRobin ~ stop function:

General idea: it is called by the processor when a running process
expires on the current scheduler timeslice or when the running
process executes a syscall. It is based on the following algorithm:

- find the reason StopReason using a match (syscall or expired timeslice)
Case 1: reason is a syscall
    - the elapsed time is evidently the current scheduler timeslice
minus remaining
    - simulate time for every process and change the states if necessary
    - the current running process will increase the execution time by the
elapsed time, syscall time by 1 and other process will have the total time increased
by elapsed time value
    - reset the timeslice to the default value (initial timeslice at the RoundRobin
instantiation) and the slept_time variable of the scheduler to 0
    - syscall cases (return success, NoRunningProcess or failure):
        - fork: new process generated by the running process and added at the back
        of the ready_q. Plan the next ready process or the current if the condition
        with remaining >= min_remaining_timeslice is met => returns a pid
        - wait: move the running process to the waiting queue, changing its state
        in the waiting for the parameter event
        - sleep: the current process has to sleep time_param time, changing the
process sleep_time, using ProcessState::Waiting{ event: None }
        - signal: moves all process that are waiting for the signal_param to the
ready_q queue
        - exit: removes a process from the processes set
        In the syscall cases, timeslice changes can occur, depending on the condition
with min_remaining_timeslice.

Case 2: reason is expired process
    - elapsed time is now equal to current RoundRobin timeslice
    - consider planning the next process on the ready_q or planning itself

match used to identify which syscall the process wants to execute

the value of the timeslice of time NonZeroUsize is accessed via get() method

conversion of usize to i32 to perform safe usize substractions

used the enums StopReason, Syscall, SyscallResult, ProcessState

remove and len methods on VecDeque variables

RoundRobin ~ list function:

General idea: used by the processor when checking the state of
processes and its fields

All process of type ProcessInfo from ready_q, sleep_q, wait_q are merged
into a VecDeque of type &dyn crate::Process which ProcessInfo implements it.
Since the order of the displayed process should be increasingly by pid, 
sort_by_key(|p| p.pid()) method is used on the merged VecDeque.

RoundRobinPQ:

Starting from RoundRobin and adding minor modifications (sorting ready_q by
priority_queue after making it contiguous, planify_again field because now
the running process is not at the front of the ready_q, rewarding and
penalizing processes using a clamp function).

## Bibliography
https://doc.rust-lang.org/std/collections/struct.VecDeque.html
SO Course: Round Robin and scheduling algorithms from pptx
https://github.com/UPB-CS-Rust/teme/issues
https://tourofrust.com/
