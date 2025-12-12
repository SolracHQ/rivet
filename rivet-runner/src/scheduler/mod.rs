//! Scheduler layer for the runner
//!
//! This layer handles polling the orchestrator for new jobs and
//! coordinating job execution. It manages the lifecycle of jobs
//! from claiming to completion.

pub mod poller;

pub use poller::JobPoller;
