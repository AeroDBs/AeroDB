//! Crash tests for AeroDB
//!
//! Per CRASH_TESTING.md:
//! - All crash tests are here
//! - Tests run sequentially (no parallel)
//! - Real filesystem (no mocks)

mod crash;

// The test scenarios are in crash/scenarios/*
// They are run as part of the test crate
