// Copyright (C) 2025 Ryan Daum <ryan.daum@gmail.com> This program is free
// software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, version
// 3.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with
// this program. If not, see <https://www.gnu.org/licenses/>.
//

pub use crate::tasks::ServerOptions;
pub use crate::tasks::scheduler_client::SchedulerClient;
pub use crate::tasks::suspension::{SuspendedTask, WakeCondition};
pub use crate::tasks::task::Task;
pub use crate::vm::vm_counters;
pub use moor_common::tasks::TaskId;

use std::cell::Cell;
use std::marker::PhantomData;

pub mod builtins;
pub mod config;
pub mod objdef;
pub mod tasks;
pub mod textdump;
pub mod vm;

/// A phantom type for explicitly marking types as !Sync
type PhantomUnsync = PhantomData<Cell<()>>;
