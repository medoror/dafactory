//! Command implementations. v0 implements `init` (B1); `validate`, `run`, and `ls`
//! are declared in the clap surface but are explicit failing stubs in `main` until
//! their backlog items land.

pub mod init;
pub mod ls;
pub mod run;
pub mod scenarios;
pub mod validate;
