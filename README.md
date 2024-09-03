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


## Implementation details

Simple Round Robin:



## Bibliography
